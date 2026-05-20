//! Schedule judgement: pure function that decides whether the current weekday
//! is within the user's configured work schedule.
//!
//! Kept I/O free so it can be unit-tested without touching the system clock.
//! The wall clock is provided by callers via [`chrono::DateTime`].

use chrono::{DateTime, Datelike, Local};

use crate::models::config::ScheduleConfig;

/// Return `true` when reminders are allowed to surface at `now` under `schedule`.
///
/// - When `schedule.enabled == false`, the schedule is treated as inactive and
///   the function always returns `true` (no suppression).
/// - When enabled, the current weekday (Mon=0 ... Sun=6) is checked against
///   `schedule.active_days`.
#[must_use]
pub(crate) fn is_schedule_active(now: DateTime<Local>, schedule: &ScheduleConfig) -> bool {
    if !schedule.enabled {
        return true;
    }

    let index = now.weekday().num_days_from_monday() as usize;
    schedule.active_days.get(index).copied().unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(year: i32, month: u32, day: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, 12, 0, 0)
            .single()
            .expect("valid local datetime")
    }

    #[test]
    fn disabled_schedule_is_always_active() {
        let schedule = ScheduleConfig {
            enabled: false,
            active_days: [false; 7],
        };

        // 2026-05-19 is a Tuesday.
        assert!(is_schedule_active(at(2026, 5, 19), &schedule));
    }

    #[test]
    fn enabled_schedule_blocks_inactive_day() {
        let schedule = ScheduleConfig {
            enabled: true,
            // Mon-Fri active, weekend off.
            active_days: [true, true, true, true, true, false, false],
        };

        // 2026-05-16 is a Saturday (index 5).
        assert!(!is_schedule_active(at(2026, 5, 16), &schedule));
        // 2026-05-17 is a Sunday (index 6).
        assert!(!is_schedule_active(at(2026, 5, 17), &schedule));
    }

    #[test]
    fn enabled_schedule_allows_active_day() {
        let schedule = ScheduleConfig {
            enabled: true,
            active_days: [true, true, true, true, true, false, false],
        };

        // 2026-05-18 Monday, 19 Tuesday, ..., 22 Friday.
        for day in 18..=22 {
            assert!(
                is_schedule_active(at(2026, 5, day), &schedule),
                "weekday {day} should be active"
            );
        }
    }

    #[test]
    fn each_weekday_index_maps_correctly() {
        // Anchor calendar: 2026-05-18 Mon, 19 Tue, 20 Wed, 21 Thu, 22 Fri, 23 Sat, 24 Sun
        let days = [
            ((2026, 5, 18), 0),
            ((2026, 5, 19), 1),
            ((2026, 5, 20), 2),
            ((2026, 5, 21), 3),
            ((2026, 5, 22), 4),
            ((2026, 5, 23), 5),
            ((2026, 5, 24), 6),
        ];

        for ((y, m, d), expected_index) in days {
            // Build a schedule where only the expected index is active.
            let mut active = [false; 7];
            active[expected_index] = true;
            let schedule = ScheduleConfig {
                enabled: true,
                active_days: active,
            };

            assert!(
                is_schedule_active(at(y, m, d), &schedule),
                "date {y}-{m:02}-{d:02} should map to index {expected_index}"
            );
        }
    }

    #[test]
    fn all_days_disabled_blocks_every_weekday() {
        let schedule = ScheduleConfig {
            enabled: true,
            active_days: [false; 7],
        };

        for day in 18..=24 {
            assert!(
                !is_schedule_active(at(2026, 5, day), &schedule),
                "every day should be blocked when all active_days are false"
            );
        }
    }
}
