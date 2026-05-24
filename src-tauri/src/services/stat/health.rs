use std::collections::BTreeMap;

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, TimeZone, Utc, Weekday};
use chrono_tz::Tz;

use crate::models::config::{Config, TimerMode};
use crate::models::statistics::{
    CycleOutcome, CycleReason, EyeCareComponents, EyeCareIndex, ReasonBreakdown, RhythmPayload,
    RibbonEntry,
};

use super::StoredCycleEvent;

pub(super) fn today_counts(
    events: &[StoredCycleEvent],
    tz: Tz,
    today_local: NaiveDate,
) -> (u32, u32, u32, ReasonBreakdown) {
    let mut taken = 0_u32;
    let mut skipped = 0_u32;
    let mut suppressed = 0_u32;
    let mut breakdown = ReasonBreakdown::default();
    let bump = |c: &mut u32| *c = c.saturating_add(1);

    for event in events {
        let event_local = event.occurred_at_utc.with_timezone(&tz).date_naive();
        if event_local != today_local {
            continue;
        }
        match event.outcome {
            CycleOutcome::Taken => bump(&mut taken),
            CycleOutcome::Skipped => bump(&mut skipped),
            CycleOutcome::Suppressed => {
                bump(&mut suppressed);
                match event.reason {
                    Some(CycleReason::Fullscreen) => bump(&mut breakdown.fullscreen),
                    Some(CycleReason::Schedule) => bump(&mut breakdown.schedule),
                    Some(CycleReason::Afk) => bump(&mut breakdown.afk),
                    Some(CycleReason::ProcessWhitelisted) => {
                        bump(&mut breakdown.process_whitelisted);
                    }
                    None => {}
                }
            }
        }
    }

    (taken, skipped, suppressed, breakdown)
}

pub(super) fn adherence_rate(taken: u32, skipped: u32) -> Option<f32> {
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

pub(super) fn ribbon_entries(
    events: &[StoredCycleEvent],
    cutoff_utc: DateTime<Utc>,
) -> Vec<RibbonEntry> {
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

/// True when the user has marked today's weekday as a rest day. Inactive
/// `ScheduleConfig` (the default for new users) means every day is a work day.
pub(super) fn is_rest_day_today(active_days: [bool; 7], weekday: Weekday) -> bool {
    let index = weekday.num_days_from_monday() as usize;
    !active_days.get(index).copied().unwrap_or(true)
}

pub(super) const fn target_work_secs(config: &Config) -> u32 {
    match config.timer.mode {
        TimerMode::TwentyTwentyTwenty => config.timer.work_minutes.saturating_mul(60),
        TimerMode::Pomodoro => config.pomodoro.focus_minutes.saturating_mul(60),
    }
}

/// Approximate longest unbroken work segment today as the max gap between
/// consecutive taken-rest timestamps, capped by today's midnight and `now_utc`.
/// Collapses Skipped/Suppressed cycles into the surrounding work segment
/// (v0.6 "best-effort" stance per PRD).
pub(super) fn longest_work_secs_today(
    today_taken_events: &[&StoredCycleEvent],
    now_utc: DateTime<Utc>,
    tz: Tz,
    today_local: NaiveDate,
) -> u32 {
    // Day start in UTC: local midnight today, converted back.
    let midnight_naive = today_local.and_hms_opt(0, 0, 0).unwrap_or_default();
    let day_start_utc = tz
        .from_local_datetime(&midnight_naive)
        .single()
        .unwrap_or_else(|| tz.from_utc_datetime(&midnight_naive))
        .with_timezone(&Utc);

    let mut prev = day_start_utc;
    let mut longest = ChronoDuration::zero();
    for event in today_taken_events {
        longest = longest.max(event.occurred_at_utc - prev);
        prev = event.occurred_at_utc;
    }
    // Tail: from the last taken (or day-start) up to now.
    longest = longest.max(now_utc - prev);

    u32::try_from(longest.num_seconds().max(0)).unwrap_or(u32::MAX)
}

/// Pure compute for the v0.6 Eye-Care Index (Beta). Formula:
///
/// - `adherence_p = clamp((taken / (taken + skipped)) * 100, 0, 100)`
/// - `longest_session_p = clamp(100 - max(0, (longest_secs - target_work_secs) / 60), 0, 100)`
/// - `score = round(0.7 * adherence_p + 0.3 * longest_session_p)`
///
/// `is_rest_day` short-circuits to `(score=None, is_rest_day=true)`;
/// `taken + skipped == 0` on a work day yields `is_warming_up=true`.
pub(super) fn compute_eye_care_index(
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
pub(super) fn compute_rhythm(
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
        let expected = (24 * 3600_u32).checked_div(target_work_secs).unwrap_or(0);
        u32::try_from((u64::from(expected) * 60) / 100).unwrap_or(u32::MAX)
    } else {
        median(&recent).max(1)
    };

    // Current streak: walk backwards from today.
    let mut current = 0_u32;
    let mut cursor = today_local;
    loop {
        match per_day.get(&cursor) {
            Some(count) if *count >= threshold => current = current.saturating_add(1),
            _ => break,
        }
        let Some(prev) = cursor.pred_opt() else { break };
        cursor = prev;
    }

    // Best streak over all observed history.
    let mut best = 0_u32;
    let mut run = 0_u32;
    let mut last: Option<NaiveDate> = None;
    for (date, count) in &per_day {
        run = if *count < threshold {
            0
        } else {
            match last {
                Some(prev) if date.signed_duration_since(prev).num_days() == 1 => {
                    run.saturating_add(1)
                }
                _ => 1,
            }
        };
        best = best.max(run);
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

    use crate::models::config::{Config, TimerMode};
    use crate::models::statistics::{CycleEventDraft, CycleOutcome, CycleReason};

    use super::super::StatService;
    use super::*;

    fn eci(taken: u32, skipped: u32, longest: u32, rest_day: bool) -> EyeCareIndex {
        compute_eye_care_index(taken, skipped, longest, 20 * 60, rest_day)
    }

    fn event_at(
        occurred_at_utc: DateTime<Utc>,
        outcome: CycleOutcome,
        reason: Option<CycleReason>,
        duration_secs: Option<u32>,
    ) -> CycleEventDraft {
        CycleEventDraft {
            occurred_at_utc,
            outcome,
            reason,
            process_hint: None,
            duration_secs,
            mode: TimerMode::TwentyTwentyTwenty,
            is_long_break: false,
        }
    }

    #[test]
    fn eye_care_index_special_cases_and_scoring() {
        // is_warming_up: no events on a work day → score=None.
        let warming = eci(0, 0, 0, false);
        assert!(warming.is_warming_up && !warming.is_rest_day && warming.score.is_none());

        // is_rest_day short-circuits to score=None regardless of counts.
        let rest = eci(0, 0, 0, true);
        assert!(rest.is_rest_day && !rest.is_warming_up && rest.score.is_none());

        // 0 taken / 5 skipped → adherence 0, longest_p 100, 0.3 * 100 = 30.
        assert_eq!(eci(0, 5, 0, false).score, Some(30));
        // Perfect adherence under target → 100.
        assert_eq!(eci(5, 0, 0, false).score, Some(100));
        // Target + 60 min overshoot → longest_p 40, 0.7*100 + 0.3*40 = 82.
        assert_eq!(eci(5, 0, 20 * 60 + 60 * 60, false).score, Some(82));
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
        // Day start 00:00 -> first rest 10:00 = 10h; tail 13:00->18:00 = 5h.
        assert_eq!(result, 10 * 3600);
    }

    #[tokio::test]
    async fn cycle_outcomes_today_counts_match_inserts() {
        let service = StatService::new_in_memory()
            .await
            .expect("in-memory stat service should init");
        let now = Utc::now();

        // 2 taken, 1 skipped, 1 suppressed/afk today.
        for _ in 0..2 {
            service
                .record_cycle_event(event_at(now, CycleOutcome::Taken, None, Some(20)))
                .await
                .expect("taken event should persist");
        }
        service
            .record_cycle_event(event_at(now, CycleOutcome::Skipped, None, None))
            .await
            .expect("skipped event should persist");
        service
            .record_cycle_event(event_at(
                now,
                CycleOutcome::Suppressed,
                Some(CycleReason::Afk),
                None,
            ))
            .await
            .expect("suppressed event should persist");

        let payload = service
            .cycle_outcomes(Some("UTC"), &Config::default())
            .await
            .expect("outcomes should aggregate");

        assert_eq!(payload.today_taken, 2);
        assert_eq!(payload.today_skipped, 1);
        assert_eq!(payload.today_suppressed, 1);
        assert_eq!(payload.today_reason_breakdown.afk, 1);
        assert!(payload.is_beta);
        let rate = payload.today_adherence_rate.expect("rate should be Some");
        assert!((rate - (2.0 / 3.0)).abs() < 1e-4);
    }
}
