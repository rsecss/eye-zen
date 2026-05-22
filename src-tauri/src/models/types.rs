use serde::{Deserialize, Serialize};

/// Shared timer payload placeholder for commands and events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct StatePayload {
    pub state: String,
    pub remaining_secs: u32,
    pub work_minutes: u32,
    pub rest_seconds: u32,
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
