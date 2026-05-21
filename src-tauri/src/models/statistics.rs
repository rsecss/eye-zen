use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
