//! Response actions for detected anomalies.

/// Alert channel for sending notifications.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertChannel {
    /// Webhook POST request.
    Webhook(String),
    /// Email notification.
    Email(Vec<String>),
    /// Slack notification.
    Slack(String),
    /// Audit log entry.
    Audit,
}

/// Alert payload sent to channels.
#[derive(Debug, Clone, PartialEq)]
pub struct AlertPayload {
    /// Message ID.
    pub message_id: String,
    /// Detection timestamp.
    pub timestamp: String,
    /// Severity level.
    pub severity: String,
    /// Anomaly reasons.
    pub reasons: Vec<String>,
    /// Combined anomaly score.
    pub score: f64,
}

impl AlertPayload {
    /// Create a new alert payload from an anomaly result.
    pub fn from_result(result: &crate::detection::AnomalyResult) -> Self {
        Self {
            message_id: result.message_id.clone(),
            timestamp: result.timestamp.to_rfc3339(),
            severity: result.severity.to_string(),
            reasons: result.reasons.clone(),
            score: result.score.combined(),
        }
    }

    /// Serialize to JSON string.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::AnomalyResult;
    use crate::{AnomalyScore, GuardAction, Severity};
    use chrono::Utc;

    fn create_test_result() -> AnomalyResult {
        AnomalyResult {
            message_id: "MSG001".to_string(),
            timestamp: Utc::now(),
            score: AnomalyScore {
                volume_score: 0.5,
                timing_score: 0.3,
                sender_score: 0.2,
                pattern_score: 0.1,
            },
            severity: Severity::Warning,
            reasons: vec!["Volume spike detected".to_string()],
            action_taken: GuardAction::Alert,
            is_duplicate: false,
            is_replay: false,
        }
    }

    #[test]
    fn test_alert_payload_from_result() {
        let result = create_test_result();
        let payload = AlertPayload::from_result(&result);

        assert_eq!(payload.message_id, "MSG001");
        assert_eq!(payload.severity, "Warning");
        assert_eq!(payload.reasons, vec!["Volume spike detected"]);
        // Combined weighted score: 0.4*0.5 + 0.2*0.3 + 0.2*0.2 + 0.2*0.1 = 0.32
        assert!((payload.score - 0.32).abs() < 0.01);
    }
}
