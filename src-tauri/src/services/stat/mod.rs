#![allow(clippy::module_name_repetitions)]

mod export;
mod health;
mod migration;
mod trends;
mod writer;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Duration as ChronoDuration, NaiveDate, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::error::{AppError, Result};
use crate::models::config::Config;
use crate::models::statistics::{
    CycleEventDraft, CycleOutcome, CycleOutcomesPayload, CycleReason, RestSessionDraft,
    StatPersistenceKind, StatisticsTrendPayload,
};
use crate::services::{Service, ServiceContext};

#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 2;

/// Bounded queue for the stat-writer channel. Hot path posts at most one
/// item per timer tick (1 Hz), so 256 buffers ~4 minutes even if WAL stalls.
const STAT_WRITE_QUEUE_CAPACITY: usize = 256;

/// Commands consumed by the dedicated stat-writer task. `Shutdown` is appended
/// at the channel tail during `Service::shutdown` so the writer drains every
/// queued draft before exiting.
#[derive(Debug)]
pub(crate) enum StatWriteCmd {
    RestSession(RestSessionDraft),
    CycleEvent(CycleEventDraft),
    Shutdown,
}

impl StatWriteCmd {
    pub(super) const fn kind(&self) -> Option<StatPersistenceKind> {
        match self {
            Self::RestSession(_) => Some(StatPersistenceKind::RestSession),
            Self::CycleEvent(_) => Some(StatPersistenceKind::CycleEvent),
            Self::Shutdown => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct StatService {
    pub(super) db_path: PathBuf,
    pub(super) pool: Arc<Mutex<Option<SqlitePool>>>,
    /// Writer-task ingress. Arc-internal clone, safe to share across tasks.
    writer_tx: mpsc::Sender<StatWriteCmd>,
    /// Taken out of the `Option` exactly once when `start()` spawns the writer.
    writer_rx: Arc<Mutex<Option<mpsc::Receiver<StatWriteCmd>>>>,
    /// Joined by `shutdown()` after the Shutdown sentinel drains the channel.
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[derive(Debug, Clone)]
pub(super) struct StoredRestSession {
    pub(super) started_at_utc: DateTime<Utc>,
    pub(super) duration_secs: u32,
}

#[derive(Debug, Clone)]
pub(super) struct StoredCycleEvent {
    pub(super) occurred_at_utc: DateTime<Utc>,
    pub(super) outcome: CycleOutcome,
    pub(super) reason: Option<CycleReason>,
}

fn parse_rfc3339_utc(raw: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| AppError::IoError {
            message: format!("invalid stored statistics timestamp: {err}"),
        })
}

impl StatService {
    #[must_use]
    pub(crate) fn new(db_path: PathBuf) -> Self {
        let (writer_tx, writer_rx) = mpsc::channel(STAT_WRITE_QUEUE_CAPACITY);
        Self {
            db_path,
            pool: Arc::new(Mutex::new(None)),
            writer_tx,
            writer_rx: Arc::new(Mutex::new(Some(writer_rx))),
            writer_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Queue a completed rest session for asynchronous persistence.
    ///
    /// # Errors
    ///
    /// `AppError::IoError` when the bounded writer queue is full or the
    /// writer task has shut down. `execute_timer_effect` translates the
    /// failure into a `stat_persistence_error` event for the UI.
    pub(crate) fn enqueue_rest_session(&self, session: RestSessionDraft) -> Result<()> {
        self.writer_tx
            .try_send(StatWriteCmd::RestSession(session))
            .map_err(|err| writer::map_send_error(&err))
    }

    /// Queue a cycle event for asynchronous persistence. See
    /// `enqueue_rest_session` for failure semantics.
    pub(crate) fn enqueue_cycle_event(&self, draft: CycleEventDraft) -> Result<()> {
        self.writer_tx
            .try_send(StatWriteCmd::CycleEvent(draft))
            .map_err(|err| writer::map_send_error(&err))
    }

    /// Synchronous persistence retained for tests that read back a write.
    #[cfg(test)]
    pub(crate) async fn record_rest_session(&self, session: RestSessionDraft) -> Result<()> {
        writer::persist_rest_session(&self.pool, session).await
    }

    /// Synchronous companion to `record_rest_session`.
    #[cfg(test)]
    pub(crate) async fn record_cycle_event(&self, draft: CycleEventDraft) -> Result<()> {
        writer::persist_cycle_event(&self.pool, draft).await
    }

    pub(crate) async fn statistics_trends(
        &self,
        timezone: Option<&str>,
    ) -> Result<StatisticsTrendPayload> {
        let tz = export::resolve_timezone(timezone)?;
        let sessions = self.fetch_rest_sessions().await?;
        Ok(trends::aggregate_sessions(&sessions, tz))
    }

    /// Today's outcome roll-up + 24h ribbon + Eye-Care Index + streak
    /// rhythm cards. Reads exclusively from `rest_cycle_events`.
    pub(crate) async fn cycle_outcomes(
        &self,
        timezone: Option<&str>,
        config: &Config,
    ) -> Result<CycleOutcomesPayload> {
        let tz = export::resolve_timezone(timezone)?;
        let now_utc = Utc::now();
        let today_local: NaiveDate = now_utc.with_timezone(&tz).date_naive();
        let events = self
            .fetch_cycle_events_since(now_utc - ChronoDuration::days(45))
            .await?;

        let (today_taken, today_skipped, today_suppressed, today_reason_breakdown) =
            health::today_counts(&events, tz, today_local);
        let today_adherence_rate = health::adherence_rate(today_taken, today_skipped);
        let last_24h_ribbon = health::ribbon_entries(&events, now_utc - ChronoDuration::hours(24));
        let is_rest_day =
            health::is_rest_day_today(config.schedule.active_days, today_local.weekday());
        let target_work_secs = health::target_work_secs(config);

        let today_taken_events: Vec<&StoredCycleEvent> = events
            .iter()
            .filter(|e| {
                e.outcome == CycleOutcome::Taken
                    && e.occurred_at_utc.with_timezone(&tz).date_naive() == today_local
            })
            .collect();
        let longest_work =
            health::longest_work_secs_today(&today_taken_events, now_utc, tz, today_local);
        let eye_care_index = health::compute_eye_care_index(
            today_taken,
            today_skipped,
            longest_work,
            target_work_secs,
            is_rest_day,
        );
        let rhythm = health::compute_rhythm(&events, tz, today_local, target_work_secs);

        Ok(CycleOutcomesPayload {
            timezone: tz.name().to_string(),
            today_taken,
            today_skipped,
            today_suppressed,
            today_adherence_rate,
            today_reason_breakdown,
            last_24h_ribbon,
            eye_care_index,
            rhythm,
            is_beta: true,
        })
    }

    async fn fetch_cycle_events_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<StoredCycleEvent>> {
        let pool = self.pool().await?;
        let rows = sqlx::query(
            r"
            SELECT occurred_at, outcome, reason
            FROM rest_cycle_events
            WHERE occurred_at >= ?1
            ORDER BY occurred_at ASC
            ",
        )
        .bind(since.to_rfc3339())
        .fetch_all(&pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let occurred_at: String = row.try_get("occurred_at")?;
                let outcome: String = row.try_get("outcome")?;
                let reason: Option<String> = row.try_get("reason")?;
                Ok(StoredCycleEvent {
                    occurred_at_utc: parse_rfc3339_utc(&occurred_at)?,
                    outcome: trends::parse_outcome(&outcome).ok_or_else(|| AppError::IoError {
                        message: format!("invalid stored cycle outcome: {outcome}"),
                    })?,
                    reason: reason.as_deref().and_then(trends::parse_reason),
                })
            })
            .collect()
    }

    /// Atomically export the statistics database via `VACUUM INTO`.
    ///
    /// `VACUUM INTO` requires the destination not to exist, so any pre-existing
    /// file is removed first (the OS save dialog already confirmed the
    /// overwrite). The filename is interpolated into the SQL because
    /// `VACUUM INTO` is a directive that does not accept a bind parameter, so
    /// `validate_export_path` enforces the trust boundary explicitly and any
    /// embedded single quote is doubled per the SQL escape rule.
    pub(crate) async fn export_to(&self, target_path: PathBuf) -> Result<()> {
        export::validate_export_path(&target_path, &self.db_path)?;
        let pool = self.pool().await?;

        if let Some(parent) = target_path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        match tokio::fs::remove_file(&target_path).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        let target_sql = target_path.to_string_lossy().replace('\'', "''");
        sqlx::query(&format!("VACUUM INTO '{target_sql}'"))
            .execute(&pool)
            .await?;

        info!("statistics database exported to {}", target_path.display());
        Ok(())
    }

    async fn pool(&self) -> Result<SqlitePool> {
        self.pool
            .lock()
            .await
            .clone()
            .ok_or_else(|| AppError::InvalidOperation {
                operation: "statistics database".to_string(),
                reason: "not initialized".to_string(),
            })
    }

    async fn fetch_rest_sessions(&self) -> Result<Vec<StoredRestSession>> {
        let pool = self.pool().await?;
        // Post v0.6 single source of truth: trends read taken cycles from
        // rest_cycle_events. Legacy activity_segments rows were backfilled
        // at the v1->v2 migration so historical charts stay continuous.
        let rows = sqlx::query(
            r"
            SELECT occurred_at, duration_secs
            FROM rest_cycle_events
            WHERE outcome = ?1
            ORDER BY occurred_at ASC
            ",
        )
        .bind("taken")
        .fetch_all(&pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let occurred_at: String = row.try_get("occurred_at")?;
                let duration_secs: Option<i64> = row.try_get("duration_secs")?;
                Ok(StoredRestSession {
                    started_at_utc: parse_rfc3339_utc(&occurred_at)?,
                    duration_secs: duration_secs
                        .map_or(0, |d| u32::try_from(d).unwrap_or(u32::MAX)),
                })
            })
            .collect()
    }

    async fn open_pool(&self) -> Result<SqlitePool> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(&self.db_path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await?;
        migration::migrate(&pool).await?;
        Ok(pool)
    }

    #[cfg(test)]
    pub(super) async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        migration::migrate(&pool).await?;
        let (writer_tx, writer_rx) = mpsc::channel(STAT_WRITE_QUEUE_CAPACITY);
        Ok(Self {
            db_path: PathBuf::from(":memory:"),
            pool: Arc::new(Mutex::new(Some(pool))),
            writer_tx,
            writer_rx: Arc::new(Mutex::new(Some(writer_rx))),
            writer_handle: Arc::new(Mutex::new(None)),
        })
    }
}

impl Service for StatService {
    async fn init(&self, _app: &ServiceContext) -> Result<()> {
        let pool = self.open_pool().await?;
        *self.pool.lock().await = Some(pool);
        info!(
            "statistics database initialized at {}",
            self.db_path.display()
        );
        Ok(())
    }

    async fn start(&self, app: &ServiceContext) -> Result<()> {
        let Some(receiver) = self.writer_rx.lock().await.take() else {
            return Err(AppError::InvalidOperation {
                operation: "stat writer".to_string(),
                reason: "writer task already started".to_string(),
            });
        };

        // Holding a full `StatService` clone here would keep a duplicate
        // Sender alive and block channel shutdown.
        let pool = Arc::clone(&self.pool);
        let app = app.clone();
        let handle = tokio::spawn(async move {
            writer::run_writer_loop(pool, receiver, app).await;
        });
        *self.writer_handle.lock().await = Some(handle);
        info!("statistics writer task started");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        // Shutdown sentinel rides the same channel so the writer drains every
        // queued draft before exiting. If the queue is already saturated the
        // sentinel is rejected, but the writer falls out of `recv()` once the
        // queue empties; the join below still bounds wait time.
        if let Err(err) = self.writer_tx.try_send(StatWriteCmd::Shutdown) {
            tracing::warn!("stat writer shutdown sentinel rejected: {err}");
        }
        if let Some(handle) = self.writer_handle.lock().await.take() {
            match handle.await {
                Ok(()) => info!("statistics writer task drained"),
                Err(err) if err.is_cancelled() => info!("statistics writer task cancelled"),
                Err(err) => error!("statistics writer task panicked: {err}"),
            }
        }
        if let Some(pool) = self.pool.lock().await.take() {
            pool.close().await;
            info!("statistics database closed");
        }
        Ok(())
    }
}
