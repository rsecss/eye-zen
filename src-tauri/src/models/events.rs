//! IPC event payload types.
//!
//! All types exported from this module MUST derive `ts_rs::TS` for frontend
//! consumption via the `src/lib/bindings/` bridge.

use serde::{Deserialize, Serialize};

use super::config::TimerMode;

/// Payload for the `state_changed` IPC event.
///
/// Emitted by `TimerService` whenever the timer state transitions or the
/// remaining time crosses a UI-relevant threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct StateChangedPayload {
    pub state: String,
    pub remaining_secs: u32,
    pub work_minutes: u32,
    pub rest_seconds: u32,
    pub mode: TimerMode,
    pub pomodoro: Option<PomodoroStatePayload>,
}

/// Pomodoro runtime snapshot, included in `StateChangedPayload` when
/// `mode == TimerMode::Pomodoro`.
///
/// - `cycle_index`: 1-based ordinal of the current focus session within the
///   long-break cycle. Range: `1..=cycles_per_long`.
/// - `is_long_break`: true while the user is in the long-break `Resting` state
///   after completing the `cycles_per_long`-th focus session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct PomodoroStatePayload {
    pub cycle_index: u32,
    pub cycles_per_long: u32,
    pub is_long_break: bool,
}

// Re-export types that serve as event payloads but are defined elsewhere:
// - Config: payload for `config_changed` event
// - HotkeyStatus: payload for `hotkey_status_changed` event
// - StatPersistenceErrorPayload: payload for `stat_persistence_error` event
#[allow(unused_imports)]
pub use super::config::Config as ConfigChangedPayload;
#[allow(unused_imports)]
pub use super::hotkeys::HotkeyStatus as HotkeyStatusChangedPayload;
#[allow(unused_imports)]
pub use super::statistics::StatPersistenceErrorPayload;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_changed_payload_json_roundtrip() {
        let payload = StateChangedPayload {
            state: "working".to_string(),
            remaining_secs: 42,
            work_minutes: 20,
            rest_seconds: 20,
            mode: TimerMode::TwentyTwentyTwenty,
            pomodoro: None,
        };

        let json = serde_json::to_string(&payload).expect("should serialize");
        let decoded: StateChangedPayload = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn state_changed_payload_pomodoro_json_roundtrip() {
        let payload = StateChangedPayload {
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

        let json = serde_json::to_string(&payload).expect("should serialize");
        let decoded: StateChangedPayload = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(decoded, payload);
    }
}
