use std::collections::BTreeMap;

use chrono::Datelike;
use chrono_tz::Tz;

use crate::models::statistics::{CycleOutcome, CycleReason, StatBucket, StatisticsTrendPayload};

use super::StoredRestSession;

#[derive(Default)]
struct BucketAccumulator {
    rest_sessions: u32,
    total_rest_secs: u32,
}

pub(super) fn aggregate_sessions(
    sessions: &[StoredRestSession],
    timezone: Tz,
) -> StatisticsTrendPayload {
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

pub(super) fn parse_outcome(raw: &str) -> Option<CycleOutcome> {
    match raw {
        "taken" => Some(CycleOutcome::Taken),
        "skipped" => Some(CycleOutcome::Skipped),
        "suppressed" => Some(CycleOutcome::Suppressed),
        _ => None,
    }
}

pub(super) fn parse_reason(raw: &str) -> Option<CycleReason> {
    match raw {
        "fullscreen" => Some(CycleReason::Fullscreen),
        "schedule" => Some(CycleReason::Schedule),
        "afk" => Some(CycleReason::Afk),
        "process_whitelisted" => Some(CycleReason::ProcessWhitelisted),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use chrono_tz::{America::Los_Angeles, America::New_York, Asia::Shanghai};

    use crate::services::Service;
    use crate::services::ServiceContext;

    use super::super::writer::{record_taken, session_at};
    use super::super::StatService;
    use super::*;

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
    async fn records_survive_service_restart() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
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
}
