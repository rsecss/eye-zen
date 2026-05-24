use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::error::{AppError, Result};
use crate::models::config::TimerMode;
use crate::models::statistics::{
    CycleEventDraft, CycleReason, RestSessionDraft, StatPersistenceErrorPayload,
    StatPersistenceKind,
};
use crate::services::ServiceContext;

use super::StatWriteCmd;

/// Drain the writer channel into `SQLite`. Persistence failures surface as
/// a `stat_persistence_error` event so the UI can warn the user; the loop
/// keeps running so a transient failure does not silently disable all
/// future writes. The loop exits when it receives a `Shutdown` sentinel
/// (graceful drain) or when every Sender has been dropped (defensive
/// fallback).
pub(super) async fn run_writer_loop(
    pool: Arc<Mutex<Option<SqlitePool>>>,
    mut rx: mpsc::Receiver<StatWriteCmd>,
    app: ServiceContext,
) {
    while let Some(cmd) = rx.recv().await {
        let kind = cmd.kind();
        let outcome = match cmd {
            StatWriteCmd::RestSession(session) => persist_rest_session(&pool, session).await,
            StatWriteCmd::CycleEvent(draft) => persist_cycle_event(&pool, draft).await,
            StatWriteCmd::Shutdown => break,
        };
        if let Err(err) = outcome {
            if let Some(kind) = kind {
                emit_persistence_error(&app, kind, &err);
            }
        }
    }
    info!("statistics writer task exit");
}

async fn locked_pool(pool: &Arc<Mutex<Option<SqlitePool>>>) -> Result<SqlitePool> {
    pool.lock()
        .await
        .clone()
        .ok_or_else(|| AppError::InvalidOperation {
            operation: "statistics database".to_string(),
            reason: "not initialized".to_string(),
        })
}

pub(super) async fn persist_rest_session(
    pool: &Arc<Mutex<Option<SqlitePool>>>,
    session: RestSessionDraft,
) -> Result<()> {
    let pool = locked_pool(pool).await?;
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

pub(super) async fn persist_cycle_event(
    pool: &Arc<Mutex<Option<SqlitePool>>>,
    draft: CycleEventDraft,
) -> Result<()> {
    let pool = locked_pool(pool).await?;
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

#[cfg(not(test))]
fn emit_persistence_error(app: &ServiceContext, kind: StatPersistenceKind, err: &AppError) {
    let payload = StatPersistenceErrorPayload {
        kind,
        occurred_at: Utc::now().to_rfc3339(),
        message: err.to_string(),
    };
    tracing::error!("stat persistence failed [{:?}]: {err}", kind);
    if let Some(handle) = app.app_handle() {
        use tauri::Emitter;
        if let Err(emit_err) = handle.emit(crate::events::STAT_PERSISTENCE_ERROR, &payload) {
            tracing::error!("failed to emit stat_persistence_error: {emit_err}");
        }
    }
}

#[cfg(test)]
fn emit_persistence_error(_app: &ServiceContext, kind: StatPersistenceKind, err: &AppError) {
    // Tests inspect StatService state directly; no AppHandle in cfg(test).
    let _ = Utc::now();
    tracing::error!("stat persistence failed [{:?}]: {err}", kind);
}

#[cfg(test)]
const _: fn() = || {
    // Silence "unused" warnings for the payload type in test builds; the
    // production cfg arm exercises it, but cfg(test) does not.
    let _ = std::mem::size_of::<StatPersistenceErrorPayload>();
};

const fn timer_mode_label(mode: TimerMode) -> &'static str {
    match mode {
        TimerMode::TwentyTwentyTwenty => "twenty_twenty_twenty",
        TimerMode::Pomodoro => "pomodoro",
    }
}

/// Translate a `try_send` failure into the unified `AppError` so callers
/// can branch on `IoError` without caring about tokio internals.
pub(super) fn map_send_error(err: &mpsc::error::TrySendError<StatWriteCmd>) -> AppError {
    match err {
        mpsc::error::TrySendError::Full(_) => AppError::IoError {
            message: "stat writer queue full (256 pending drafts)".to_string(),
        },
        mpsc::error::TrySendError::Closed(_) => AppError::IoError {
            message: "stat writer task has shut down".to_string(),
        },
    }
}

#[cfg(test)]
pub(super) fn session_at(
    started_at_utc: chrono::DateTime<chrono::Utc>,
    duration_secs: u32,
) -> RestSessionDraft {
    RestSessionDraft {
        started_at_utc,
        ended_at_utc: started_at_utc + chrono::Duration::seconds(i64::from(duration_secs)),
        duration_secs,
    }
}

/// Production emits both `RecordRestSession` and `RecordCycleEvent` for
/// every completed rest. Test fixtures mirror that by writing to both
/// tables synchronously.
#[cfg(test)]
pub(super) async fn record_taken(service: &super::StatService, draft: RestSessionDraft) {
    use crate::models::statistics::CycleOutcome;

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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use sqlx::SqlitePool;

    use crate::models::config::TimerMode;
    use crate::models::statistics::{CycleEventDraft, CycleOutcome, RestSessionDraft};
    use crate::services::Service;
    use crate::services::ServiceContext;

    use super::super::{StatService, STAT_WRITE_QUEUE_CAPACITY};
    use super::session_at;

    /// Poll until the writer task has flushed `expected` rows; bounded at 1s.
    async fn wait_for_row_count(pool: &SqlitePool, table: &str, expected: i64) {
        for _ in 0..50 {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(pool)
                .await
                .expect("count query should succeed");
            if count == expected {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let actual: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .expect("count query should succeed");
        panic!("writer task did not persist {expected} {table} rows within 1s, observed {actual}");
    }

    async fn pool_of(service: &StatService) -> SqlitePool {
        service
            .pool
            .lock()
            .await
            .clone()
            .expect("pool should be initialized")
    }

    async fn started_service() -> StatService {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        service
            .start(&ServiceContext::default())
            .await
            .expect("writer task should start");
        service
    }

    fn anchor() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 20, 10, 0, 0)
            .single()
            .expect("valid UTC datetime")
    }

    fn make_session(offset_secs: i64) -> RestSessionDraft {
        session_at(anchor() + chrono::Duration::seconds(offset_secs), 20)
    }

    fn skipped_at(occurred_at_utc: DateTime<Utc>) -> CycleEventDraft {
        CycleEventDraft {
            occurred_at_utc,
            outcome: CycleOutcome::Skipped,
            reason: None,
            process_hint: None,
            duration_secs: None,
            mode: TimerMode::TwentyTwentyTwenty,
            is_long_break: false,
        }
    }

    #[tokio::test]
    async fn writer_task_persists_enqueued_rest_session() {
        let service = started_service().await;
        service
            .enqueue_rest_session(make_session(0))
            .expect("enqueue should succeed");

        let pool = pool_of(&service).await;
        wait_for_row_count(&pool, "activity_segments", 1).await;
        service.shutdown().await.expect("shutdown should succeed");
    }

    #[tokio::test]
    async fn writer_task_persists_enqueued_cycle_event() {
        let service = started_service().await;
        service
            .enqueue_cycle_event(skipped_at(anchor()))
            .expect("enqueue should succeed");

        let pool = pool_of(&service).await;
        wait_for_row_count(&pool, "rest_cycle_events", 1).await;
        service.shutdown().await.expect("shutdown should succeed");
    }

    #[tokio::test]
    async fn shutdown_drains_queued_drafts_before_exit() {
        // File-backed so the verify pool can re-open after shutdown; an
        // in-memory pool would be torn down with the service.
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path.clone());
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");
        service
            .start(&ServiceContext::default())
            .await
            .expect("writer task should start");

        // Burst 10 then request shutdown. The sentinel sits at the queue
        // tail so all 10 MUST persist before the writer exits.
        for offset in 0..10 {
            service
                .enqueue_rest_session(make_session(offset))
                .expect("enqueue should succeed");
        }
        service.shutdown().await.expect("shutdown should drain");

        let verify_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&db_path))
            .await
            .expect("verify pool should open");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_segments")
            .fetch_one(&verify_pool)
            .await
            .expect("count query should succeed");
        assert_eq!(count, 10, "shutdown must drain every queued draft");
        verify_pool.close().await;
    }

    #[tokio::test]
    async fn enqueue_after_shutdown_returns_io_error() {
        let service = started_service().await;
        service.shutdown().await.expect("shutdown should succeed");

        match service.enqueue_rest_session(make_session(0)) {
            Err(crate::error::AppError::IoError { message }) => {
                assert!(
                    message.contains("shut down") || message.contains("queue full"),
                    "expected closed-channel error, got {message}"
                );
            }
            other => panic!("expected IoError after shutdown, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn enqueue_when_queue_full_returns_io_error() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        // No `start()`: receiver sits idle and the channel fills up. After
        // STAT_WRITE_QUEUE_CAPACITY items, the next one MUST fail.
        for offset in 0..STAT_WRITE_QUEUE_CAPACITY {
            service
                .enqueue_rest_session(make_session(i64::try_from(offset).unwrap_or(i64::MAX)))
                .expect("fill should succeed under capacity");
        }
        match service.enqueue_rest_session(make_session(3600)) {
            Err(crate::error::AppError::IoError { message }) => {
                assert!(message.contains("queue full"), "got {message}");
            }
            other => panic!("expected IoError when queue saturated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn record_cycle_event_persists_skipped_with_no_reason() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        service
            .record_cycle_event(skipped_at(
                Utc.with_ymd_and_hms(2026, 5, 20, 8, 0, 0)
                    .single()
                    .expect("valid UTC datetime"),
            ))
            .await
            .expect("skipped event should persist");

        let pool = pool_of(&service).await;
        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT outcome, reason FROM rest_cycle_events ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .expect("row should fetch");
        assert_eq!(row.0, "skipped");
        assert!(row.1.is_none());
    }
}
