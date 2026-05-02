//! Pattern detection for anomalies.

use crate::{GuardError, MessageFeatures};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Result of pattern detection checks.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PatternCheckResult {
    /// Whether message is a duplicate.
    pub is_duplicate: bool,
    /// Whether message is a replay attack.
    pub is_replay: bool,
}

/// Pattern detector for duplicates and replays.
#[derive(Debug)]
pub struct PatternDetector {
    duplicate_window: Duration,
    /// Hash -> first seen timestamp
    recent_hashes: HashMap<String, DateTime<Utc>>,
    /// Control ID -> (first seen, content hash)
    control_ids: HashMap<String, (DateTime<Utc>, String)>,
}

impl PatternDetector {
    /// Create a new pattern detector.
    pub fn new(duplicate_window: Duration, _baseline_days: u32) -> Self {
        Self {
            duplicate_window,
            recent_hashes: HashMap::new(),
            control_ids: HashMap::new(),
        }
    }

    /// Check patterns for a message.
    pub fn check_patterns(
        &self,
        features: &MessageFeatures,
    ) -> Result<PatternCheckResult, GuardError> {
        let mut result = PatternCheckResult::default();

        // Check for duplicate
        if self.is_duplicate(&features.message_hash) {
            result.is_duplicate = true;
        }

        // Check for replay
        if self.is_replay(&features.control_id, &features.message_hash) {
            result.is_replay = true;
        }

        Ok(result)
    }

    /// Check if message hash is a duplicate within the window.
    fn is_duplicate(&self, message_hash: &str) -> bool {
        if let Some(timestamp) = self.recent_hashes.get(message_hash) {
            Utc::now() - *timestamp < self.duplicate_window
        } else {
            false
        }
    }

    /// Check if control ID is a replay (same ID, different content).
    fn is_replay(&self, control_id: &str, message_hash: &str) -> bool {
        if control_id.is_empty() {
            return false;
        }

        if let Some((_, known_hash)) = self.control_ids.get(control_id) {
            known_hash != message_hash // Same ID, different content = replay
        } else {
            false
        }
    }

    /// Record a message for future duplicate/replay detection.
    pub fn record_message(&mut self, features: &MessageFeatures) {
        let now = Utc::now();

        // Record hash
        self.recent_hashes
            .insert(features.message_hash.clone(), now);

        // Record control ID
        if !features.control_id.is_empty() {
            self.control_ids.insert(
                features.control_id.clone(),
                (now, features.message_hash.clone()),
            );
        }

        // Clean up expired entries
        self.cleanup_expired(now);
    }

    /// Clean up expired entries.
    fn cleanup_expired(&mut self, now: DateTime<Utc>) {
        self.recent_hashes
            .retain(|_, timestamp| now - *timestamp < self.duplicate_window);

        // Control IDs use same window for cleanup
        self.control_ids
            .retain(|_, (timestamp, _)| now - *timestamp < self.duplicate_window);
    }

    /// Get count of tracked hashes.
    pub fn hash_count(&self) -> usize {
        self.recent_hashes.len()
    }

    /// Get count of tracked control IDs.
    pub fn control_id_count(&self) -> usize {
        self.control_ids.len()
    }
}

/// Anomaly detection result for a message.
#[derive(Debug, Clone, PartialEq)]
pub struct AnomalyResult {
    /// Message control ID.
    pub message_id: String,
    /// Detection timestamp.
    pub timestamp: DateTime<Utc>,
    /// Anomaly score.
    pub score: crate::AnomalyScore,
    /// Severity level.
    pub severity: crate::Severity,
    /// Human-readable reasons for anomaly.
    pub reasons: Vec<String>,
    /// Action taken.
    pub action_taken: crate::GuardAction,
    /// Whether message is a duplicate.
    pub is_duplicate: bool,
    /// Whether message is a replay.
    pub is_replay: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageFeatures;

    fn create_test_features(message_hash: &str, control_id: &str) -> MessageFeatures {
        MessageFeatures {
            message_hash: message_hash.to_string(),
            sender_id: "test-sender".to_string(),
            timestamp: Utc::now(),
            message_type: "ADT^A01".to_string(),
            control_id: control_id.to_string(),
            hour_of_day: 12,
            day_of_week: 1,
            is_business_hours: true,
        }
    }

    #[test]
    fn test_duplicate_detection() {
        let mut detector = PatternDetector::new(Duration::minutes(5), 7);
        let features = create_test_features("hash1", "ctrl1");

        // First message - not a duplicate
        let result = detector.check_patterns(&features).unwrap();
        assert!(!result.is_duplicate);

        // Record it
        detector.record_message(&features);

        // Same message again - should be duplicate
        let result = detector.check_patterns(&features).unwrap();
        assert!(result.is_duplicate);
    }

    #[test]
    fn test_replay_detection() {
        let mut detector = PatternDetector::new(Duration::minutes(5), 7);
        let features1 = create_test_features("hash1", "ctrl1");

        // First message
        detector.record_message(&features1);

        // Same control ID, different content - replay attack
        let features2 = create_test_features("hash2", "ctrl1");
        let result = detector.check_patterns(&features2).unwrap();
        assert!(result.is_replay);

        // Same control ID, same content - duplicate (not replay)
        let result = detector.check_patterns(&features1).unwrap();
        assert!(result.is_duplicate);
        assert!(!result.is_replay);
    }

    #[test]
    fn test_empty_control_id() {
        let detector = PatternDetector::new(Duration::minutes(5), 7);
        let features = create_test_features("hash1", "");

        // Empty control ID should not cause replay
        let result = detector.check_patterns(&features).unwrap();
        assert!(!result.is_replay);
    }

    #[test]
    fn test_cleanup_expired() {
        let mut detector = PatternDetector::new(Duration::seconds(1), 7);
        let features = create_test_features("hash1", "ctrl1");

        detector.record_message(&features);
        assert_eq!(detector.hash_count(), 1);
        assert_eq!(detector.control_id_count(), 1);

        // Wait for expiry
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Add another message to trigger cleanup
        let features2 = create_test_features("hash2", "ctrl2");
        detector.record_message(&features2);

        // Old entries should be cleaned up
        assert_eq!(detector.hash_count(), 1); // Only the new one
        assert_eq!(detector.control_id_count(), 1);
    }
}
