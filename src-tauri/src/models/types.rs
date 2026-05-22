use serde::{Deserialize, Serialize};

use super::config::TimerMode;

/// Shared timer payload placeholder for commands and events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct StatePayload {
    pub state: String,
    pub remaining_secs: u32,
    pub work_minutes: u32,
    pub rest_seconds: u32,
    pub mode: TimerMode,
    pub pomodoro: Option<PomodoroStatePayload>,
}

/// Pomodoro runtime snapshot. Only present when `StatePayload::mode ==
/// TimerMode::Pomodoro`.
///
/// - `cycle_index`: 1-based ordinal of the *current* focus session in this
///   long-break cycle. Range `1..=cycles_per_long`.
/// - `is_long_break`: true while the user is in the long-break Resting after
///   completing the `cycles_per_long`-th focus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct PomodoroStatePayload {
    pub cycle_index: u32,
    pub cycles_per_long: u32,
    pub is_long_break: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct DetectorCapabilities {
    pub afk_detection_supported: bool,
    pub foreground_process_detection_supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_payload_json_roundtrip() {
        let payload = StatePayload {
            state: "working".to_string(),
            remaining_secs: 42,
            work_minutes: 20,
            rest_seconds: 20,
            mode: TimerMode::TwentyTwentyTwenty,
            pomodoro: None,
        };

        let json = serde_json::to_string(&payload).expect("payload should serialize");
        let decoded: StatePayload =
            serde_json::from_str(&json).expect("payload should deserialize");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn state_payload_pomodoro_json_roundtrip() {
        let payload = StatePayload {
            state: "resting".to_string(),
            remaining_secs: 300,
            work_minutes: 25,
            rest_seconds: 300,
            mode: TimerMode::Pomodoro,
            pomodoro: Some(PomodoroStatePayload {
                cycle_index: 4,
                cycles_per_long: 4,
                is_long_break: true,
            }),
        };

        let json = serde_json::to_string(&payload).expect("payload should serialize");
        let decoded: StatePayload =
            serde_json::from_str(&json).expect("payload should deserialize");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn detector_capabilities_json_roundtrip() {
        let capabilities = DetectorCapabilities {
            afk_detection_supported: true,
            foreground_process_detection_supported: true,
        };

        let json = serde_json::to_string(&capabilities).expect("capabilities should serialize");
        let decoded: DetectorCapabilities =
            serde_json::from_str(&json).expect("capabilities should deserialize");
        assert_eq!(decoded, capabilities);
    }
}
