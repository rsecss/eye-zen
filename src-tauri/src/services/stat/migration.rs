use sqlx::SqlitePool;

use crate::error::Result;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<()> {
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;

    if version < 1 {
        migrate_initial_to_v1(pool).await?;
    }

    if version < 2 {
        migrate_v1_to_v2(pool).await?;
    }

    Ok(())
}

/// Initial schema creation (v0 -> v1). Wrapped in a single transaction so a
/// partial failure (process crash mid-step, disk error) rolls back cleanly
/// and the next launch retries from `user_version = 0`.
async fn migrate_initial_to_v1(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;

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
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX idx_activity_segments_date ON activity_segments(date)")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r"
        CREATE INDEX idx_activity_segments_state_started_at
        ON activity_segments(state, started_at)
        ",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("PRAGMA user_version = 1")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// One-shot v1 -> v2 migration for the Health Analysis feature.
///
/// All steps run inside a single `SQLite` transaction so a partial failure
/// (process crash, disk error mid-step) rolls back atomically and leaves
/// `PRAGMA user_version = 1`. The next launch retries from a clean slate.
///
/// Steps:
/// - Create `rest_cycle_events` and its two indexes. `IF NOT EXISTS` here is
///   intentional: a user-restored snapshot can carry a v1 `user_version` yet
///   already contain a partially populated `rest_cycle_events` table; we
///   want the migration to converge instead of failing with "table already
///   exists" and leaving the user stranded at v1.
/// - Backfill `state = 'resting'` rows from `activity_segments` as
///   `outcome = 'taken'` cycles (mode = NULL because legacy rows predate
///   the per-cycle mode snapshot). Guarded by a `WHERE NOT EXISTS` clause
///   so a partial re-application (e.g. user manually restored a snapshot
///   that already contains some backfilled rows) does not duplicate.
/// - Bump `PRAGMA user_version` to 2 inside the same transaction.
async fn migrate_v1_to_v2(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;

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
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r"
        CREATE INDEX IF NOT EXISTS idx_rest_cycle_events_occurred_at
        ON rest_cycle_events(occurred_at)
        ",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r"
        CREATE INDEX IF NOT EXISTS idx_rest_cycle_events_outcome
        ON rest_cycle_events(outcome)
        ",
    )
    .execute(&mut *tx)
    .await?;

    // Backfill: every legacy resting segment becomes a `taken` cycle. mode
    // stays NULL because we cannot reconstruct the timer mode that was
    // active when the row was written. The `WHERE NOT EXISTS` predicate
    // makes the INSERT idempotent against any rest_cycle_events row that
    // already carries the same occurred_at / duration_secs signature, so a
    // recovered snapshot with partial backfill cannot produce duplicates.
    sqlx::query(
        r"
        INSERT INTO rest_cycle_events
               (occurred_at, outcome, reason, process_hint, duration_secs, mode, is_long_break)
        SELECT a.started_at, 'taken', NULL, NULL, a.duration_secs, NULL, 0
        FROM   activity_segments AS a
        WHERE  a.state = 'resting'
          AND  NOT EXISTS (
                  SELECT 1 FROM rest_cycle_events AS r
                  WHERE r.outcome = 'taken'
                    AND r.occurred_at = a.started_at
                    AND (r.duration_secs IS a.duration_secs)
              )
        ",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("PRAGMA user_version = 2")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    use super::super::StatService;
    use super::*;

    /// Hand-craft a v1 database: `activity_segments` table, `user_version = 1`,
    /// and `count` `resting` rows starting at 2026-05-20 10:00 UTC, 20 minutes
    /// apart. Returns the base timestamp so callers can reference the first
    /// row's `occurred_at` in follow-up assertions.
    async fn seed_v1_with_resting(pool: &SqlitePool, count: u32) -> DateTime<Utc> {
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
        .execute(pool)
        .await
        .expect("v1 schema should create");
        sqlx::query("PRAGMA user_version = 1")
            .execute(pool)
            .await
            .expect("v1 version should set");
        let base = Utc
            .with_ymd_and_hms(2026, 5, 20, 10, 0, 0)
            .single()
            .expect("valid UTC datetime");
        for offset in 0..count {
            let started = base + chrono::Duration::minutes(i64::from(offset) * 20);
            sqlx::query(
                "INSERT INTO activity_segments (state, started_at, ended_at, duration_secs, date)
                 VALUES ('resting', ?1, ?2, ?3, ?4)",
            )
            .bind(started.to_rfc3339())
            .bind((started + chrono::Duration::seconds(20)).to_rfc3339())
            .bind(20_i64)
            .bind("2026-05-20")
            .execute(pool)
            .await
            .expect("v1 row should insert");
        }
        base
    }

    #[tokio::test]
    async fn fresh_init_reports_schema_version_two() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        let pool = service
            .pool
            .lock()
            .await
            .clone()
            .expect("pool should exist");
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
        seed_v1_with_resting(&pool, 3).await;

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

    /// Simulates a crash mid-migration where `rest_cycle_events` and partial
    /// backfill survived but `PRAGMA user_version` never reached 2. The next
    /// run MUST complete cleanly without duplicating the rows that already
    /// landed in the table, and MUST bump the version to 2.
    #[tokio::test]
    async fn migration_v1_to_v2_is_idempotent_on_partial_failure() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool should open");
        let base = seed_v1_with_resting(&pool, 3).await;

        // Simulate the post-crash state: rest_cycle_events exists and holds
        // one of the three backfill rows, but user_version is still 1.
        sqlx::query(
            r"
            CREATE TABLE rest_cycle_events (
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
        .execute(&pool)
        .await
        .expect("rest_cycle_events table should pre-exist");
        sqlx::query(
            "INSERT INTO rest_cycle_events
                 (occurred_at, outcome, reason, process_hint, duration_secs, mode, is_long_break)
             VALUES (?1, 'taken', NULL, NULL, 20, NULL, 0)",
        )
        .bind(base.to_rfc3339())
        .execute(&pool)
        .await
        .expect("partial backfill row should insert");

        migrate(&pool)
            .await
            .expect("recovery migrate should succeed");

        let post_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(post_version, 2);
        let post_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rest_cycle_events WHERE outcome = 'taken'")
                .fetch_one(&pool)
                .await
                .expect("count should query");
        // Expect 3: the pre-existing row plus the two missing ones, no
        // duplicates for the pre-existing one.
        assert_eq!(post_rows, 3);
    }

    /// When a transaction step inside `migrate_v1_to_v2` fails, the version
    /// MUST stay at 1 (atomic rollback). We provoke a failure by pre-creating
    /// `rest_cycle_events` with a CHECK constraint that the backfill row
    /// cannot satisfy (mode = NULL hits the CHECK), then assert the rollback.
    #[tokio::test]
    async fn migration_v1_to_v2_rolls_back_on_failure() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool should open");
        seed_v1_with_resting(&pool, 1).await;

        // Pre-create rest_cycle_events with a CHECK constraint the backfill
        // row violates. `CREATE TABLE IF NOT EXISTS` inside the migration is
        // then a no-op, but the INSERT ... SELECT afterwards fails inside the
        // transaction.
        sqlx::query(
            r"
            CREATE TABLE rest_cycle_events (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                occurred_at   TEXT NOT NULL,
                outcome       TEXT NOT NULL,
                reason        TEXT,
                process_hint  TEXT,
                duration_secs INTEGER,
                mode          TEXT NOT NULL CHECK (mode IN ('forced')),
                is_long_break INTEGER NOT NULL DEFAULT 0
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("pre-existing constrained table should create");

        let result = migrate(&pool).await;
        assert!(result.is_err(), "constraint violation should fail migrate");

        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(
            version, 1,
            "rolled back transaction must leave version at 1"
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rest_cycle_events")
            .fetch_one(&pool)
            .await
            .expect("count should query");
        assert_eq!(rows, 0, "no backfill rows must survive the rollback");
    }
}
