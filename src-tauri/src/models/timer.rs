//! Timer-specific DTO types for IPC commands and state snapshots.
//!
//! This module contains types primarily consumed by Tauri commands that query
//! timer state or detector capabilities.

use serde::{Deserialize, Serialize};

// Re-export event payload types for backward compatibility with existing code
// that imports `StatePayload` from this module. Frontend command responses
// use `StatePayload`; backend events emit `StateChangedPayload`.
pub use super::events::{PomodoroStatePayload, StateChangedPayload};

/// Alias for backward compatibility. Frontend uses this name; backend emits
/// `StateChangedPayload` in events.
pub type StatePayload = StateChangedPayload;

/// Platform detector capabilities reported to the Settings UI.
///
/// Each field indicates whether the underlying platform API supports the
/// corresponding detection feature. Settings uses these to gray out controls
/// for unavailable features.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct DetectorCapabilities {
    pub afk_detection_supported: bool,
    pub foreground_process_detection_supported: bool,
    pub fullscreen_detection_supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::TimerMode;

    #[test]
    fn state_payload_alias_works() {
        // Verify the alias works for both construction and deserialization.
        let payload = StatePayload {
            state: "working".to_string(),
            remaining_secs: 120,
            work_minutes: 20,
            rest_seconds: 20,
            mode: TimerMode::TwentyTwentyTwenty,
            pomodoro: None,
        };

        let json = serde_json::to_string(&payload).expect("should serialize");
        let decoded: StatePayload = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn detector_capabilities_json_roundtrip() {
        let capabilities = DetectorCapabilities {
            afk_detection_supported: true,
            foreground_process_detection_supported: true,
            fullscreen_detection_supported: true,
        };

        let json = serde_json::to_string(&capabilities).expect("should serialize");
        let decoded: DetectorCapabilities =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(decoded, capabilities);
    }
}
