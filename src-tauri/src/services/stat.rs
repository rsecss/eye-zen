#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Tz;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tokio::sync::Mutex;
use tracing::info;

use crate::error::{AppError, Result};
use crate::models::statistics::{RestSessionDraft, StatBucket, StatisticsTrendPayload};
use crate::services::{Service, ServiceContext};

const SCHEMA_VERSION: i64 = 1;

#[derive(Clone)]
pub(crate) struct StatService {
    db_path: PathBuf,
    pool: Arc<Mutex<Option<SqlitePool>>>,
}

#[derive(Debug, Clone)]
struct StoredRestSession {
    started_at_utc: DateTime<Utc>,
    duration_secs: u32,
}

#[derive(Default)]
struct BucketAccumulator {
    rest_sessions: u32,
    total_rest_secs: u32,
}

impl StatService {
    #[must_use]
    pub(crate) fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            pool: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) async fn record_rest_session(&self, session: RestSessionDraft) -> Result<()> {
        let pool = self.pool().await?;
        let utc_date = session.started_at_utc.format("%Y-%m-%d").to_string();

        sqlx::query(
            r"
            INSERT INTO activity_segments (state, started_at, ended_at, duration_secs, date)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
        )
        .bind("resting")
        .bind(session.started_at_utc.to_rfc3339())
        .bind(session.ended_at_utc.to_rfc3339())
        .bind(i64::from(session.duration_secs))
        .bind(utc_date)
        .execute(&pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn statistics_trends(
        &self,
        timezone: Option<&str>,
    ) -> Result<StatisticsTrendPayload> {
        let tz = resolve_timezone(timezone)?;
        let sessions = self.fetch_rest_sessions().await?;
        Ok(aggregate_sessions(&sessions, tz))
    }

    /// Atomically export the statistics database to `target_path` via
    /// `VACUUM INTO`. `VACUUM INTO` requires the destination not to exist,
    /// so any pre-existing file at that path is removed first; the caller's
    /// `dialog::save` already confirmed the overwrite.
    ///
    /// `SQLite`'s `VACUUM INTO` is a directive whose filename is a string
    /// literal in the SQL grammar and does NOT accept a bind parameter, so
    /// the path is interpolated and any embedded single quote is doubled
    /// per the SQL escape rule. `target_path` is sourced from the OS save
    /// dialog (no untrusted network input), so this stays within the
    /// trusted boundary.
    pub(crate) async fn export_to(&self, target_path: PathBuf) -> Result<()> {
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
        let rows = sqlx::query(
            r"
            SELECT started_at, duration_secs
            FROM activity_segments
            WHERE state = ?1
            ORDER BY started_at ASC
            ",
        )
        .bind("resting")
        .fetch_all(&pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let started_at: String = row.try_get("started_at")?;
                let duration_secs: i64 = row.try_get("duration_secs")?;
                let started_at_utc = DateTime::parse_from_rfc3339(&started_at)
                    .map_err(|err| AppError::IoError {
                        message: format!("invalid stored statistics timestamp: {err}"),
                    })?
                    .with_timezone(&Utc);

                Ok(StoredRestSession {
                    started_at_utc,
                    duration_secs: u32::try_from(duration_secs).unwrap_or(u32::MAX),
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
        migrate(&pool).await?;
        Ok(pool)
    }

    #[cfg(test)]
    async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        migrate(&pool).await?;
        Ok(Self {
            db_path: PathBuf::from(":memory:"),
            pool: Arc::new(Mutex::new(Some(pool))),
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

    async fn start(&self, _app: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if let Some(pool) = self.pool.lock().await.take() {
            pool.close().await;
            info!("statistics database closed");
        }
        Ok(())
    }
}

async fn migrate(pool: &SqlitePool) -> Result<()> {
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;

    if version < SCHEMA_VERSION {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS activity_segments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                state TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT NOT NULL,
                duration_secs INTEGER NOT NULL,
                date TEXT NOT NULL
            )
            ",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_activity_segments_date ON activity_segments(date)",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_activity_segments_state_started_at
            ON activity_segments(state, started_at)
            ",
        )
        .execute(pool)
        .await?;
        sqlx::query("PRAGMA user_version = 1").execute(pool).await?;
    }

    Ok(())
}

fn resolve_timezone(timezone: Option<&str>) -> Result<Tz> {
    let requested = timezone
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| iana_time_zone::get_timezone().ok())
        .unwrap_or_else(|| "UTC".to_string());

    requested
        .parse::<Tz>()
        .map_err(|err| AppError::ConfigInvalid {
            field: "timezone".to_string(),
            reason: format!("invalid IANA timezone \"{requested}\": {err}"),
        })
}

fn aggregate_sessions(sessions: &[StoredRestSession], timezone: Tz) -> StatisticsTrendPayload {
    let mut daily = BTreeMap::<String, BucketAccumulator>::new();
    let mut weekly = BTreeMap::<String, BucketAccumulator>::new();
    let mut monthly = BTreeMap::<String, BucketAccumulator>::new();
    let mut total_sessions = 0_u32;
    let mut total_rest_secs = 0_u32;

    for session in sessions {
        let local = session.started_at_utc.with_timezone(&timezone);
        let date = local.date_naive();
        let iso_week = date.iso_week();

        add_bucket(
            &mut daily,
            date.format("%Y-%m-%d").to_string(),
            session.duration_secs,
        );
        add_bucket(
            &mut weekly,
            format!("{:04}-W{:02}", iso_week.year(), iso_week.week()),
            session.duration_secs,
        );
        add_bucket(
            &mut monthly,
            format!("{:04}-{:02}", date.year(), date.month()),
            session.duration_secs,
        );

        total_sessions = total_sessions.saturating_add(1);
        total_rest_secs = total_rest_secs.saturating_add(session.duration_secs);
    }

    StatisticsTrendPayload {
        timezone: timezone.name().to_string(),
        daily: into_buckets(daily),
        weekly: into_buckets(weekly),
        monthly: into_buckets(monthly),
        total_sessions,
        total_rest_secs,
    }
}

fn add_bucket(
    buckets: &mut BTreeMap<String, BucketAccumulator>,
    label: String,
    duration_secs: u32,
) {
    let bucket = buckets.entry(label).or_default();
    bucket.rest_sessions = bucket.rest_sessions.saturating_add(1);
    bucket.total_rest_secs = bucket.total_rest_secs.saturating_add(duration_secs);
}

fn into_buckets(buckets: BTreeMap<String, BucketAccumulator>) -> Vec<StatBucket> {
    buckets
        .into_iter()
        .map(|(label, bucket)| StatBucket {
            label,
            rest_sessions: bucket.rest_sessions,
            total_rest_secs: bucket.total_rest_secs,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use chrono_tz::{America::Los_Angeles, America::New_York, Asia::Shanghai};
    use tempfile::tempdir;

    use super::*;

    fn session_at(started_at_utc: DateTime<Utc>, duration_secs: u32) -> RestSessionDraft {
        RestSessionDraft {
            started_at_utc,
            ended_at_utc: started_at_utc + chrono::Duration::seconds(i64::from(duration_secs)),
            duration_secs,
        }
    }

    #[tokio::test]
    async fn init_creates_missing_database_and_schema() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path.clone());

        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        assert!(db_path.exists());
        let pool = service.pool().await.expect("pool should exist");
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'activity_segments'",
        )
        .fetch_one(&pool)
        .await
        .expect("schema query should work");
        assert_eq!(table_count, 1);
    }

    #[tokio::test]
    async fn records_survive_service_restart() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let first = StatService::new(db_path.clone());
        first
            .init(&ServiceContext::default())
            .await
            .expect("first service should init");
        first
            .record_rest_session(session_at(
                Utc.with_ymd_and_hms(2026, 5, 20, 10, 0, 0)
                    .single()
                    .expect("valid UTC datetime"),
                20,
            ))
            .await
            .expect("record should persist");
        first.shutdown().await.expect("shutdown should close db");

        let second = StatService::new(db_path);
        second
            .init(&ServiceContext::default())
            .await
            .expect("second service should init");
        let payload = second
            .statistics_trends(Some("UTC"))
            .await
            .expect("trends should load");

        assert_eq!(payload.total_sessions, 1);
        assert_eq!(payload.total_rest_secs, 20);
    }

    #[tokio::test]
    async fn aggregates_five_completed_rest_cycles() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        let started_at = Utc
            .with_ymd_and_hms(2026, 5, 20, 8, 0, 0)
            .single()
            .expect("valid UTC datetime");

        for offset in 0..5 {
            service
                .record_rest_session(session_at(
                    started_at + chrono::Duration::minutes(offset * 20),
                    20,
                ))
                .await
                .expect("record should persist");
        }

        let payload = service
            .statistics_trends(Some("UTC"))
            .await
            .expect("trends should aggregate");

        assert_eq!(payload.total_sessions, 5);
        assert_eq!(payload.total_rest_secs, 100);
        assert_eq!(payload.daily[0].label, "2026-05-20");
        assert_eq!(payload.daily[0].rest_sessions, 5);
        assert_eq!(payload.weekly[0].rest_sessions, 5);
        assert_eq!(payload.monthly[0].rest_sessions, 5);
    }

    #[test]
    fn aggregation_respects_requested_timezone_day_boundary() {
        let sessions = vec![StoredRestSession {
            started_at_utc: Utc
                .with_ymd_and_hms(2026, 1, 1, 1, 30, 0)
                .single()
                .expect("valid UTC datetime"),
            duration_secs: 20,
        }];

        let utc = aggregate_sessions(&sessions, chrono_tz::UTC);
        let pacific = aggregate_sessions(&sessions, Los_Angeles);

        assert_eq!(utc.daily[0].label, "2026-01-01");
        assert_eq!(pacific.daily[0].label, "2025-12-31");
    }

    #[test]
    fn aggregation_handles_month_end_in_local_timezone() {
        let sessions = vec![StoredRestSession {
            started_at_utc: Utc
                .with_ymd_and_hms(2026, 1, 31, 17, 30, 0)
                .single()
                .expect("valid UTC datetime"),
            duration_secs: 20,
        }];

        let payload = aggregate_sessions(&sessions, Shanghai);

        assert_eq!(payload.daily[0].label, "2026-02-01");
        assert_eq!(payload.monthly[0].label, "2026-02");
    }

    #[test]
    fn aggregation_handles_dst_fallback_without_splitting_local_day() {
        let sessions = vec![
            StoredRestSession {
                started_at_utc: Utc
                    .with_ymd_and_hms(2026, 11, 1, 5, 30, 0)
                    .single()
                    .expect("valid UTC datetime"),
                duration_secs: 20,
            },
            StoredRestSession {
                started_at_utc: Utc
                    .with_ymd_and_hms(2026, 11, 1, 6, 30, 0)
                    .single()
                    .expect("valid UTC datetime"),
                duration_secs: 20,
            },
        ];

        let payload = aggregate_sessions(&sessions, New_York);

        assert_eq!(payload.daily.len(), 1);
        assert_eq!(payload.daily[0].label, "2026-11-01");
        assert_eq!(payload.daily[0].rest_sessions, 2);
        assert_eq!(payload.total_rest_secs, 40);
    }

    #[tokio::test]
    async fn export_to_writes_a_valid_sqlite_file_preserving_rows() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        for offset in 0..3 {
            service
                .record_rest_session(session_at(
                    Utc.with_ymd_and_hms(2026, 5, 20, 8, 0, 0)
                        .single()
                        .expect("valid UTC datetime")
                        + chrono::Duration::minutes(offset * 20),
                    20,
                ))
                .await
                .expect("record should persist");
        }

        let target = dir.path().join("backup.db");
        service
            .export_to(target.clone())
            .await
            .expect("export should succeed");
        assert!(target.exists(), "backup file should be created");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&target))
            .await
            .expect("backup should open as sqlite");
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM activity_segments WHERE state = 'resting'")
                .fetch_one(&pool)
                .await
                .expect("backup should expose activity_segments");
        assert_eq!(rows, 3);

        let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(user_version, SCHEMA_VERSION);
        pool.close().await;
    }

    #[tokio::test]
    async fn export_to_succeeds_on_empty_database() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        let target = dir.path().join("empty-backup.db");
        service
            .export_to(target.clone())
            .await
            .expect("empty export should succeed");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&target))
            .await
            .expect("empty backup should open as sqlite");
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_segments")
            .fetch_one(&pool)
            .await
            .expect("schema should still exist");
        assert_eq!(rows, 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn export_to_overwrites_existing_target_file() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        let target = dir.path().join("backup.db");
        std::fs::write(&target, b"old contents that VACUUM INTO would reject")
            .expect("seed file should write");
        assert!(target.exists());

        service
            .export_to(target.clone())
            .await
            .expect("export should overwrite existing target");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&target))
            .await
            .expect("overwritten file should be a valid sqlite db");
        let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(user_version, SCHEMA_VERSION);
        pool.close().await;
    }

    #[tokio::test]
    async fn export_to_returns_error_for_unwritable_target_directory() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        // Use the source database file itself as a fake "parent directory".
        // `create_dir_all` will fail because that path is a file, not a dir.
        let bogus_target = service.db_path.join("nested").join("backup.db");
        let err = service
            .export_to(bogus_target)
            .await
            .expect_err("export to invalid path should fail");
        assert!(matches!(err, AppError::IoError { .. }));
    }
}
