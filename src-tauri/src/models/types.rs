use serde::{Deserialize, Serialize};

/// Shared timer payload placeholder for commands and events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatePayload {
    pub state: String,
    pub remaining_secs: u32,
    pub work_minutes: u32,
    pub rest_seconds: u32,
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
}
