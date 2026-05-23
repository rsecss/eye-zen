use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::config::TimerMode;

/// Aggregated statistics bucket exposed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct StatBucket {
    pub label: String,
    pub rest_sessions: u32,
    pub total_rest_secs: u32,
}

/// Daily, weekly, and monthly trend data for the Statistics page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct StatisticsTrendPayload {
    pub timezone: String,
    pub daily: Vec<StatBucket>,
    pub weekly: Vec<StatBucket>,
    pub monthly: Vec<StatBucket>,
    pub total_sessions: u32,
    pub total_rest_secs: u32,
}

/// Completed rest session produced by the timer effect pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RestSessionDraft {
    pub(crate) started_at_utc: DateTime<Utc>,
    pub(crate) ended_at_utc: DateTime<Utc>,
    pub(crate) duration_secs: u32,
}

/// Outcome of a single rest cycle event. The schema persists the
/// `snake_case` form ("taken", "skipped", "suppressed").
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "snake_case")]
pub enum CycleOutcome {
    Taken,
    Skipped,
    Suppressed,
}

impl CycleOutcome {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Taken => "taken",
            Self::Skipped => "skipped",
            Self::Suppressed => "suppressed",
        }
    }
}

/// Reason a rest cycle was `suppressed` (and only suppressed). Other
/// outcomes always store `reason = NULL`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "snake_case")]
pub enum CycleReason {
    Fullscreen,
    Schedule,
    Afk,
    ProcessWhitelisted,
}

impl CycleReason {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Fullscreen => "fullscreen",
            Self::Schedule => "schedule",
            Self::Afk => "afk",
            Self::ProcessWhitelisted => "process_whitelisted",
        }
    }
}

/// Draft cycle event produced by the timer state machine / loop before
/// being committed to `rest_cycle_events`. The `mode` and `is_long_break`
/// snapshot the timer rhythm at emission time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CycleEventDraft {
    pub(crate) occurred_at_utc: DateTime<Utc>,
    pub(crate) outcome: CycleOutcome,
    pub(crate) reason: Option<CycleReason>,
    pub(crate) process_hint: Option<String>,
    pub(crate) duration_secs: Option<u32>,
    pub(crate) mode: TimerMode,
    pub(crate) is_long_break: bool,
}

/// Breakdown of today's `suppressed` cycles by reason.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct ReasonBreakdown {
    pub fullscreen: u32,
    pub schedule: u32,
    pub afk: u32,
    pub process_whitelisted: u32,
}

/// One entry in the last-24h adherence ribbon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct RibbonEntry {
    /// RFC3339 UTC timestamp; frontend converts to local time.
    pub occurred_at: String,
    pub outcome: CycleOutcome,
    pub reason: Option<CycleReason>,
}

/// Component contributions to the v0.6 Eye-Care Index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct EyeCareComponents {
    pub adherence: f32,
    pub longest_session: f32,
}

/// Eye-Care Index (Beta) summary. `score` is `None` when not computable
/// (warming up at start of day, or schedule-inactive "rest day").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct EyeCareIndex {
    pub score: Option<u8>,
    pub is_warming_up: bool,
    pub is_rest_day: bool,
    pub components: EyeCareComponents,
}

/// Streak metrics derived from daily `taken` counts vs the user's
/// personal threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct RhythmPayload {
    pub current_streak_days: u32,
    pub best_streak_days: u32,
    pub threshold: u32,
}

/// Today's outcome roll-up + 24h ribbon + Eye-Care Index + rhythm cards.
/// Returned by the `statistics_cycle_outcomes` IPC command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct CycleOutcomesPayload {
    pub timezone: String,
    pub today_taken: u32,
    pub today_skipped: u32,
    pub today_suppressed: u32,
    pub today_adherence_rate: Option<f32>,
    pub today_reason_breakdown: ReasonBreakdown,
    pub last_24h_ribbon: Vec<RibbonEntry>,
    pub eye_care_index: EyeCareIndex,
    pub rhythm: RhythmPayload,
    /// Always `true` in v0.6 to render the Beta label / tooltip.
    pub is_beta: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_payload_json_roundtrip() {
        let payload = StatisticsTrendPayload {
            timezone: "UTC".to_string(),
            daily: vec![StatBucket {
                label: "2026-05-20".to_string(),
                rest_sessions: 5,
                total_rest_secs: 100,
            }],
            weekly: Vec::new(),
            monthly: Vec::new(),
            total_sessions: 5,
            total_rest_secs: 100,
        };

        let json = serde_json::to_string(&payload).expect("payload should serialize");
        let decoded: StatisticsTrendPayload =
            serde_json::from_str(&json).expect("payload should deserialize");
        assert_eq!(decoded, payload);
    }
}
