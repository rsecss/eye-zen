#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Duration as ChronoDuration, NaiveDate, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tokio::sync::Mutex;
use tracing::info;

use crate::error::{AppError, Result};
use crate::models::config::{Config, TimerMode};
use crate::models::statistics::{
    CycleEventDraft, CycleOutcome, CycleOutcomesPayload, CycleReason, EyeCareComponents,
    EyeCareIndex, ReasonBreakdown, RestSessionDraft, RhythmPayload, RibbonEntry, StatBucket,
    StatisticsTrendPayload,
};
use crate::services::{Service, ServiceContext};

#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 2;

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

#[derive(Debug, Clone)]
struct StoredCycleEvent {
    occurred_at_utc: DateTime<Utc>,
    outcome: CycleOutcome,
    reason: Option<CycleReason>,
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

    /// Persist a rest-cycle event (taken / skipped / suppressed) into
    /// `rest_cycle_events`. The dual-write for `taken` outcomes into
    /// `activity_segments` is handled by the existing
    /// `record_rest_session` path so legacy export consumers stay correct
    /// without this method having to know about that table.
    pub(crate) async fn record_cycle_event(&self, draft: CycleEventDraft) -> Result<()> {
        let pool = self.pool().await?;

        sqlx::query(
            r"
            INSERT INTO rest_cycle_events
                (occurred_at, outcome, reason, process_hint, duration_secs, mode, is_long_break)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
        )
        .bind(draft.occurred_at_utc.to_rfc3339())
        .bind(draft.outcome.as_str())
        .bind(draft.reason.map(CycleReason::as_str))
        .bind(draft.process_hint.as_deref())
        .bind(draft.duration_secs.map(i64::from))
        .bind(timer_mode_label(draft.mode))
        .bind(i64::from(draft.is_long_break))
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

    /// Today's outcome roll-up + 24h ribbon + Eye-Care Index + streak
    /// rhythm cards. Reads exclusively from `rest_cycle_events`.
    pub(crate) async fn cycle_outcomes(
        &self,
        timezone: Option<&str>,
        config: &Config,
    ) -> Result<CycleOutcomesPayload> {
        let tz = resolve_timezone(timezone)?;
        let now_utc = Utc::now();
        let now_local = now_utc.with_timezone(&tz);
        let today_local: NaiveDate = now_local.date_naive();

        let events = self
            .fetch_cycle_events_since(now_utc - ChronoDuration::days(45))
            .await?;

        let (today_taken, today_skipped, today_suppressed, today_reason_breakdown) =
            today_counts(&events, tz, today_local);

        let today_adherence_rate = adherence_rate(today_taken, today_skipped);

        let ribbon_cutoff = now_utc - ChronoDuration::hours(24);
        let last_24h_ribbon = ribbon_entries(&events, ribbon_cutoff);

        let is_rest_day = is_rest_day_today(config.schedule.active_days, today_local.weekday());

        let target_work_secs = target_work_secs(config);
        let today_taken_events: Vec<&StoredCycleEvent> = events
            .iter()
            .filter(|e| {
                e.outcome == CycleOutcome::Taken
                    && e.occurred_at_utc.with_timezone(&tz).date_naive() == today_local
            })
            .collect();
        let longest_work = longest_work_secs_today(&today_taken_events, now_utc, tz, today_local);

        let eye_care_index = compute_eye_care_index(
            today_taken,
            today_skipped,
            longest_work,
            target_work_secs,
            is_rest_day,
        );

        let rhythm = compute_rhythm(&events, tz, today_local, target_work_secs);

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

                let occurred_at_utc = DateTime::parse_from_rfc3339(&occurred_at)
                    .map_err(|err| AppError::IoError {
                        message: format!("invalid stored statistics timestamp: {err}"),
                    })?
                    .with_timezone(&Utc);

                Ok(StoredCycleEvent {
                    occurred_at_utc,
                    outcome: parse_outcome(&outcome).ok_or_else(|| AppError::IoError {
                        message: format!("invalid stored cycle outcome: {outcome}"),
                    })?,
                    reason: reason.as_deref().and_then(parse_reason),
                })
            })
            .collect()
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
                let started_at_utc = DateTime::parse_from_rfc3339(&occurred_at)
                    .map_err(|err| AppError::IoError {
                        message: format!("invalid stored statistics timestamp: {err}"),
                    })?
                    .with_timezone(&Utc);

                Ok(StoredRestSession {
                    started_at_utc,
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

    if version < 1 {
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

    if version < 2 {
        migrate_v1_to_v2(pool).await?;
    }

    Ok(())
}

/// One-shot, idempotent v1 -> v2 migration for the Health Analysis feature.
///
/// - Creates `rest_cycle_events` and its two indexes.
/// - Backfills all existing `state = 'resting'` rows from `activity_segments`
///   as `outcome = 'taken'` cycles (mode = NULL because legacy rows predate
///   the per-cycle mode snapshot).
/// - Bumps `PRAGMA user_version` to 2.
///
/// Re-running this migration on an already-v2 database is a no-op because
/// the caller gates on `version < 2`; the SQL itself is also defensive
/// (`CREATE TABLE IF NOT EXISTS` + `INSERT ... SELECT`).
async fn migrate_v1_to_v2(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS rest_cycle_events (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            occurred_at   TEXT NOT NULL,
            outcome       TEXT NOT NULL,
            reason        TEXT,
            process_hint  TEXT,
            duration_secs INTEGER,
            mode          TEXT,
            is_long_break INTEGER NOT NULL DEFAULT 0
        )
        ",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r"
        CREATE INDEX IF NOT EXISTS idx_rest_cycle_events_occurred_at
        ON rest_cycle_events(occurred_at)
        ",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r"
        CREATE INDEX IF NOT EXISTS idx_rest_cycle_events_outcome
        ON rest_cycle_events(outcome)
        ",
    )
    .execute(pool)
    .await?;

    // Backfill: every legacy resting segment becomes a `taken` cycle. mode
    // stays NULL because we cannot reconstruct the timer mode that was
    // active when the row was written.
    sqlx::query(
        r"
        INSERT INTO rest_cycle_events
               (occurred_at, outcome, reason, process_hint, duration_secs, mode, is_long_break)
        SELECT started_at, 'taken', NULL,   NULL,         duration_secs, NULL, 0
        FROM   activity_segments
        WHERE  state = 'resting'
        ",
    )
    .execute(pool)
    .await?;

    sqlx::query("PRAGMA user_version = 2").execute(pool).await?;
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

const fn timer_mode_label(mode: TimerMode) -> &'static str {
    match mode {
        TimerMode::TwentyTwentyTwenty => "twenty_twenty_twenty",
        TimerMode::Pomodoro => "pomodoro",
    }
}

fn parse_outcome(raw: &str) -> Option<CycleOutcome> {
    match raw {
        "taken" => Some(CycleOutcome::Taken),
        "skipped" => Some(CycleOutcome::Skipped),
        "suppressed" => Some(CycleOutcome::Suppressed),
        _ => None,
    }
}

fn parse_reason(raw: &str) -> Option<CycleReason> {
    match raw {
        "fullscreen" => Some(CycleReason::Fullscreen),
        "schedule" => Some(CycleReason::Schedule),
        "afk" => Some(CycleReason::Afk),
        "process_whitelisted" => Some(CycleReason::ProcessWhitelisted),
        _ => None,
    }
}

fn today_counts(
    events: &[StoredCycleEvent],
    tz: Tz,
    today_local: NaiveDate,
) -> (u32, u32, u32, ReasonBreakdown) {
    let mut taken = 0_u32;
    let mut skipped = 0_u32;
    let mut suppressed = 0_u32;
    let mut breakdown = ReasonBreakdown::default();

    for event in events {
        let event_local = event.occurred_at_utc.with_timezone(&tz).date_naive();
        if event_local != today_local {
            continue;
        }
        match event.outcome {
            CycleOutcome::Taken => taken = taken.saturating_add(1),
            CycleOutcome::Skipped => skipped = skipped.saturating_add(1),
            CycleOutcome::Suppressed => {
                suppressed = suppressed.saturating_add(1);
                match event.reason {
                    Some(CycleReason::Fullscreen) => {
                        breakdown.fullscreen = breakdown.fullscreen.saturating_add(1);
                    }
                    Some(CycleReason::Schedule) => {
                        breakdown.schedule = breakdown.schedule.saturating_add(1);
                    }
                    Some(CycleReason::Afk) => {
                        breakdown.afk = breakdown.afk.saturating_add(1);
                    }
                    Some(CycleReason::ProcessWhitelisted) => {
                        breakdown.process_whitelisted =
                            breakdown.process_whitelisted.saturating_add(1);
                    }
                    None => {}
                }
            }
        }
    }

    (taken, skipped, suppressed, breakdown)
}

fn adherence_rate(taken: u32, skipped: u32) -> Option<f32> {
    let denom = taken.saturating_add(skipped);
    if denom == 0 {
        None
    } else {
        // `denom` is bounded by tick frequency; loss of precision for f32 is
        // acceptable for a 0..1 ratio.
        #[allow(clippy::cast_precision_loss)]
        Some(taken as f32 / denom as f32)
    }
}

fn ribbon_entries(events: &[StoredCycleEvent], cutoff_utc: DateTime<Utc>) -> Vec<RibbonEntry> {
    events
        .iter()
        .filter(|e| e.occurred_at_utc >= cutoff_utc)
        .map(|e| RibbonEntry {
            occurred_at: e.occurred_at_utc.to_rfc3339(),
            outcome: e.outcome,
            reason: e.reason,
        })
        .collect()
}

/// True when the user has marked today's weekday as a rest day. Rest day
/// is the inverse of "active day"; an inactive `ScheduleConfig` (the
/// default for new users) means every day is a work day, so it is never
/// a rest day.
fn is_rest_day_today(active_days: [bool; 7], weekday: Weekday) -> bool {
    let index = weekday.num_days_from_monday() as usize;
    !active_days.get(index).copied().unwrap_or(true)
}

const fn target_work_secs(config: &Config) -> u32 {
    match config.timer.mode {
        TimerMode::TwentyTwentyTwenty => config.timer.work_minutes.saturating_mul(60),
        TimerMode::Pomodoro => config.pomodoro.focus_minutes.saturating_mul(60),
    }
}

/// Approximate the longest unbroken work segment today by measuring the
/// gap between consecutive taken-rest timestamps (and between today's
/// midnight and the first taken rest, and between the last taken rest
/// and `now_utc`). The approximation collapses Skipped / Suppressed
/// cycles into the surrounding work segment, which is the v0.6
/// "best-effort" stance noted in the PRD.
fn longest_work_secs_today(
    today_taken_events: &[&StoredCycleEvent],
    now_utc: DateTime<Utc>,
    tz: Tz,
    today_local: NaiveDate,
) -> u32 {
    // Day start in UTC: local midnight today, converted back.
    let day_start_local = tz
        .from_local_datetime(&today_local.and_hms_opt(0, 0, 0).unwrap_or_default())
        .single()
        .unwrap_or_else(|| {
            tz.from_utc_datetime(&today_local.and_hms_opt(0, 0, 0).unwrap_or_default())
        });
    let day_start_utc = day_start_local.with_timezone(&Utc);

    let mut prev = day_start_utc;
    let mut longest = ChronoDuration::zero();
    for event in today_taken_events {
        let gap = event.occurred_at_utc - prev;
        if gap > longest {
            longest = gap;
        }
        prev = event.occurred_at_utc;
    }
    // Tail: from the last taken (or day-start) up to now.
    let tail = now_utc - prev;
    if tail > longest {
        longest = tail;
    }

    u32::try_from(longest.num_seconds().max(0)).unwrap_or(u32::MAX)
}

/// Pure compute for the v0.6 Eye-Care Index (Beta). Formula:
///
/// - `adherence_p = clamp((taken / (taken + skipped)) * 100, 0, 100)`
/// - `longest_session_p = clamp(100 - max(0, (longest_secs - target_work_secs) / 60), 0, 100)`
/// - `score = round(0.7 * adherence_p + 0.3 * longest_session_p)`
///
/// Special cases (PRD §5 + Q4):
/// - `is_rest_day == true` => return `is_rest_day=true, score=None`.
/// - `taken + skipped == 0` and not rest day => `is_warming_up=true, score=None`.
fn compute_eye_care_index(
    taken: u32,
    skipped: u32,
    longest_work_secs: u32,
    target_work_secs: u32,
    is_rest_day: bool,
) -> EyeCareIndex {
    if is_rest_day {
        return EyeCareIndex {
            score: None,
            is_warming_up: false,
            is_rest_day: true,
            components: EyeCareComponents {
                adherence: 0.0,
                longest_session: 0.0,
            },
        };
    }

    let denom = taken.saturating_add(skipped);
    if denom == 0 {
        return EyeCareIndex {
            score: None,
            is_warming_up: true,
            is_rest_day: false,
            components: EyeCareComponents {
                adherence: 0.0,
                longest_session: 0.0,
            },
        };
    }

    #[allow(clippy::cast_precision_loss)]
    let adherence_p = (taken as f32 / denom as f32 * 100.0).clamp(0.0, 100.0);
    let overshoot_secs = longest_work_secs.saturating_sub(target_work_secs);
    #[allow(clippy::cast_precision_loss)]
    let longest_session_p = (100.0 - (overshoot_secs as f32 / 60.0)).clamp(0.0, 100.0);

    let score = 0.7_f32 * adherence_p + 0.3_f32 * longest_session_p;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rounded = score.round().clamp(0.0, 100.0) as u8;

    EyeCareIndex {
        score: Some(rounded),
        is_warming_up: false,
        is_rest_day: false,
        components: EyeCareComponents {
            adherence: adherence_p,
            longest_session: longest_session_p,
        },
    }
}

/// Streak threshold: median of last 30 days' `taken` counts, falling back
/// to a 60% of-expected heuristic for users with no history.
fn compute_rhythm(
    events: &[StoredCycleEvent],
    tz: Tz,
    today_local: NaiveDate,
    target_work_secs: u32,
) -> RhythmPayload {
    let mut per_day: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for event in events {
        if event.outcome != CycleOutcome::Taken {
            continue;
        }
        let local_date = event.occurred_at_utc.with_timezone(&tz).date_naive();
        let entry = per_day.entry(local_date).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    // Threshold: median of last 30 days that have data; fallback when
    // history is thin.
    let recent: Vec<u32> = per_day
        .iter()
        .filter(|(date, _)| **date <= today_local)
        .rev()
        .take(30)
        .map(|(_, count)| *count)
        .collect();
    let threshold = if recent.is_empty() {
        // 60% of expected daily rests = 60% of (24h / target_work_secs).
        let day_secs: u32 = 24 * 3600;
        let expected = day_secs.checked_div(target_work_secs).unwrap_or(0);
        u32::try_from((u64::from(expected) * 60) / 100).unwrap_or(u32::MAX)
    } else {
        median(&recent).max(1)
    };

    // Current streak: walk backwards from today.
    let mut current = 0_u32;
    let mut cursor = today_local;
    loop {
        match per_day.get(&cursor) {
            Some(count) if *count >= threshold => {
                current = current.saturating_add(1);
            }
            _ => break,
        }
        cursor = match cursor.pred_opt() {
            Some(prev) => prev,
            None => break,
        };
    }

    // Best streak over all observed history.
    let mut best = 0_u32;
    let mut run = 0_u32;
    let mut last: Option<NaiveDate> = None;
    for (date, count) in &per_day {
        if *count < threshold {
            run = 0;
            last = Some(*date);
            continue;
        }
        run = match last {
            Some(prev) if date.signed_duration_since(prev).num_days() == 1 => run.saturating_add(1),
            _ => 1,
        };
        if run > best {
            best = run;
        }
        last = Some(*date);
    }

    RhythmPayload {
        current_streak_days: current,
        best_streak_days: best.max(current),
        threshold,
    }
}

fn median(values: &[u32]) -> u32 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
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

    /// Production emits both `RecordRestSession` and `RecordCycleEvent` for
    /// every completed rest. Tests mirror that by writing to both tables.
    async fn record_taken(service: &StatService, draft: RestSessionDraft) {
        service
            .record_rest_session(draft.clone())
            .await
            .expect("record_rest_session should persist");
        service
            .record_cycle_event(CycleEventDraft {
                occurred_at_utc: draft.started_at_utc,
                outcome: CycleOutcome::Taken,
                reason: None,
                process_hint: None,
                duration_secs: Some(draft.duration_secs),
                mode: TimerMode::TwentyTwentyTwenty,
                is_long_break: false,
            })
            .await
            .expect("record_cycle_event should persist");
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
        record_taken(
            &first,
            session_at(
                Utc.with_ymd_and_hms(2026, 5, 20, 10, 0, 0)
                    .single()
                    .expect("valid UTC datetime"),
                20,
            ),
        )
        .await;
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
            record_taken(
                &service,
                session_at(started_at + chrono::Duration::minutes(offset * 20), 20),
            )
            .await;
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

    #[tokio::test]
    async fn fresh_init_reports_schema_version_two() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        let pool = service.pool().await.expect("pool should exist");
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(version, 2);

        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'rest_cycle_events'",
        )
        .fetch_one(&pool)
        .await
        .expect("rest_cycle_events table query should work");
        assert_eq!(table_count, 1);
    }

    #[tokio::test]
    async fn v1_to_v2_migration_backfills_taken_cycles() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool should open");

        // Hand-craft a v1 database: schema v1 tables only, no rest_cycle_events.
        sqlx::query(
            r"
            CREATE TABLE activity_segments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                state TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT NOT NULL,
                duration_secs INTEGER NOT NULL,
                date TEXT NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("v1 schema should create");
        sqlx::query("PRAGMA user_version = 1")
            .execute(&pool)
            .await
            .expect("v1 version should set");
        for offset in 0..3 {
            sqlx::query(
                r"
                INSERT INTO activity_segments (state, started_at, ended_at, duration_secs, date)
                VALUES ('resting', ?1, ?2, ?3, ?4)
                ",
            )
            .bind(
                (Utc.with_ymd_and_hms(2026, 5, 20, 10, 0, 0)
                    .single()
                    .expect("valid UTC datetime")
                    + chrono::Duration::minutes(i64::from(offset) * 20))
                .to_rfc3339(),
            )
            .bind(
                (Utc.with_ymd_and_hms(2026, 5, 20, 10, 0, 20)
                    .single()
                    .expect("valid UTC datetime")
                    + chrono::Duration::minutes(i64::from(offset) * 20))
                .to_rfc3339(),
            )
            .bind(20_i64)
            .bind("2026-05-20")
            .execute(&pool)
            .await
            .expect("v1 row should insert");
        }

        // Run the v2 migration directly.
        migrate(&pool).await.expect("migration should succeed");

        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(version, 2);

        let cycle_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rest_cycle_events WHERE outcome = 'taken'")
                .fetch_one(&pool)
                .await
                .expect("backfill count should query");
        assert_eq!(cycle_rows, 3);

        // Idempotency: re-running migrate must not duplicate.
        migrate(&pool).await.expect("second migrate should no-op");
        let cycle_rows_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rest_cycle_events WHERE outcome = 'taken'")
                .fetch_one(&pool)
                .await
                .expect("backfill count should query");
        assert_eq!(cycle_rows_after, 3);
    }

    #[tokio::test]
    async fn record_cycle_event_persists_skipped_with_no_reason() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        service
            .record_cycle_event(CycleEventDraft {
                occurred_at_utc: Utc
                    .with_ymd_and_hms(2026, 5, 20, 8, 0, 0)
                    .single()
                    .expect("valid UTC datetime"),
                outcome: CycleOutcome::Skipped,
                reason: None,
                process_hint: None,
                duration_secs: None,
                mode: TimerMode::TwentyTwentyTwenty,
                is_long_break: false,
            })
            .await
            .expect("skipped event should persist");

        let pool = service.pool().await.expect("pool should exist");
        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT outcome, reason FROM rest_cycle_events ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .expect("row should fetch");
        assert_eq!(row.0, "skipped");
        assert!(row.1.is_none());
    }

    #[tokio::test]
    async fn cycle_outcomes_today_counts_match_inserts() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        let today_utc = Utc::now();
        // 2 taken, 1 skipped, 1 suppressed/afk today.
        for _ in 0..2 {
            service
                .record_cycle_event(CycleEventDraft {
                    occurred_at_utc: today_utc,
                    outcome: CycleOutcome::Taken,
                    reason: None,
                    process_hint: None,
                    duration_secs: Some(20),
                    mode: TimerMode::TwentyTwentyTwenty,
                    is_long_break: false,
                })
                .await
                .expect("taken event should persist");
        }
        service
            .record_cycle_event(CycleEventDraft {
                occurred_at_utc: today_utc,
                outcome: CycleOutcome::Skipped,
                reason: None,
                process_hint: None,
                duration_secs: None,
                mode: TimerMode::TwentyTwentyTwenty,
                is_long_break: false,
            })
            .await
            .expect("skipped event should persist");
        service
            .record_cycle_event(CycleEventDraft {
                occurred_at_utc: today_utc,
                outcome: CycleOutcome::Suppressed,
                reason: Some(CycleReason::Afk),
                process_hint: None,
                duration_secs: None,
                mode: TimerMode::TwentyTwentyTwenty,
                is_long_break: false,
            })
            .await
            .expect("suppressed event should persist");

        let config = Config::default();
        let payload = service
            .cycle_outcomes(Some("UTC"), &config)
            .await
            .expect("outcomes should aggregate");

        assert_eq!(payload.today_taken, 2);
        assert_eq!(payload.today_skipped, 1);
        assert_eq!(payload.today_suppressed, 1);
        assert_eq!(payload.today_reason_breakdown.afk, 1);
        assert!(payload.is_beta);
        // adherence_rate = 2 / (2 + 1)
        let rate = payload.today_adherence_rate.expect("rate should be Some");
        assert!((rate - (2.0 / 3.0)).abs() < 1e-4);
    }

    #[test]
    fn eye_care_index_is_warming_up_when_no_events() {
        let index = compute_eye_care_index(0, 0, 0, 20 * 60, false);
        assert!(index.is_warming_up);
        assert!(!index.is_rest_day);
        assert!(index.score.is_none());
    }

    #[test]
    fn eye_care_index_is_rest_day() {
        let index = compute_eye_care_index(0, 0, 0, 20 * 60, true);
        assert!(index.is_rest_day);
        assert!(!index.is_warming_up);
        assert!(index.score.is_none());
    }

    #[test]
    fn eye_care_index_zero_taken_all_skipped_scores_low() {
        // adherence_p = 0, longest_session_p = 100, score = 0.3 * 100 = 30.
        let index = compute_eye_care_index(0, 5, 0, 20 * 60, false);
        let score = index.score.expect("score should compute");
        assert_eq!(score, 30);
    }

    #[test]
    fn eye_care_index_perfect_adherence_under_target_scores_high() {
        // adherence_p = 100, longest_session_p = 100, score = 100.
        let index = compute_eye_care_index(5, 0, 0, 20 * 60, false);
        let score = index.score.expect("score should compute");
        assert_eq!(score, 100);
    }

    #[test]
    fn eye_care_index_long_overshoot_drops_score() {
        // adherence_p = 100; longest = target + 60 min => -60 longest_p.
        let overshoot_secs = 20 * 60 + 60 * 60;
        let index = compute_eye_care_index(5, 0, overshoot_secs, 20 * 60, false);
        let score = index.score.expect("score should compute");
        // longest_session_p = 100 - 60 = 40; score = 0.7*100 + 0.3*40 = 82.
        assert_eq!(score, 82);
    }

    #[test]
    fn longest_work_secs_empty_returns_full_day_so_far() {
        let now_utc = Utc
            .with_ymd_and_hms(2026, 5, 20, 12, 0, 0)
            .single()
            .expect("valid UTC datetime");
        let result = longest_work_secs_today(
            &[],
            now_utc,
            chrono_tz::UTC,
            NaiveDate::from_ymd_opt(2026, 5, 20).expect("valid date"),
        );
        // Today started 12h ago in UTC.
        assert_eq!(result, 12 * 3600);
    }

    #[test]
    fn longest_work_secs_multiple_events_picks_max_gap() {
        let day = NaiveDate::from_ymd_opt(2026, 5, 20).expect("valid date");
        let now_utc = Utc
            .with_ymd_and_hms(2026, 5, 20, 18, 0, 0)
            .single()
            .expect("valid UTC datetime");
        let events: Vec<StoredCycleEvent> = [10, 12, 13]
            .into_iter()
            .map(|hour| StoredCycleEvent {
                occurred_at_utc: Utc
                    .with_ymd_and_hms(2026, 5, 20, hour, 0, 0)
                    .single()
                    .expect("valid UTC datetime"),
                outcome: CycleOutcome::Taken,
                reason: None,
            })
            .collect();
        let refs: Vec<&StoredCycleEvent> = events.iter().collect();
        let result = longest_work_secs_today(&refs, now_utc, chrono_tz::UTC, day);
        // Day start 00:00 -> first rest 10:00 = 10h = 36000s; tail 13:00->18:00 = 5h.
        assert_eq!(result, 10 * 3600);
    }
}
