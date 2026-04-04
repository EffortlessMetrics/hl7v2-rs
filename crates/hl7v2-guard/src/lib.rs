//! ML-based anomaly detection and threat response for HL7 messages.
//!
//! This crate provides statistical ML-based anomaly detection for HL7 messages,
//! focusing on explainable, deterministic methods suitable for healthcare
//! compliance (FDA/HIPAA).
//!
//! # Features
//!
//! - **Volume Spike Detection**: Z-score based message volume analysis
//! - **Field Outlier Detection**: Statistical outliers in numeric and coded fields
//! - **Duplicate Detection**: SHA-256 hash comparison within time window
//! - **Replay Detection**: Control ID tracking with content hash verification
//! - **Sender Behavior Profiles**: Expected message types per sender
//! - **Automated Response**: Alert, quarantine, block, or throttle actions
//!
//! # Safety First
//!
//! Default action is `alert` - never blocks messages by default to ensure
//! patient safety. Operators must explicitly opt-in to blocking actions.
//!
//! # Example
//!
//! ```
//! use hl7v2_guard::{Guard, GuardConfig, GuardAction};
//!
//! let config = GuardConfig::builder()
//!     .enabled(true)
//!     .baseline_days(7)
//!     .z_threshold(3.0)
//!     .action(GuardAction::Alert)
//!     .build();
//!
//! let guard = Guard::new(config);
//! ```

use chrono::{DateTime, Datelike, Timelike, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub mod config;
pub mod detection;
pub mod response;
pub mod stats;

pub use config::{GuardAction, GuardConfig, GuardConfigBuilder, Severity};
pub use detection::{AnomalyResult, PatternDetector};
pub use response::AlertChannel;
pub use stats::{SenderBaseline, StreamingStats, TimeSeriesStats};

/// Default Z-score threshold for anomaly detection
pub const DEFAULT_Z_THRESHOLD: f64 = 3.0;

/// Default baseline learning window in days
pub const DEFAULT_BASELINE_DAYS: u32 = 7;

/// Default duplicate detection window in seconds (5 minutes)
pub const DEFAULT_DUPLICATE_WINDOW_SECS: u64 = 300;

/// Default maximum memory for baseline storage in MB
pub const DEFAULT_MAX_MEMORY_MB: usize = 100;

/// Anomaly detection error types.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GuardError {
    /// Guard is disabled.
    #[error("Guard is disabled")]
    Disabled,

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Baseline not established.
    #[error("Baseline not established for sender: {0}")]
    BaselineNotEstablished(String),

    /// Memory limit exceeded.
    #[error("Memory limit exceeded: {0} MB")]
    MemoryLimitExceeded(usize),

    /// Detection error.
    #[error("Detection error: {0}")]
    DetectionError(String),
}

/// A message feature extracted for anomaly detection.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageFeatures {
    /// Message hash (SHA-256)
    pub message_hash: String,
    /// Sender identifier
    pub sender_id: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Message type (e.g., "ADT^A01")
    pub message_type: String,
    /// Message control ID (MSH-10)
    pub control_id: String,
    /// Hour of day (0-23)
    pub hour_of_day: u32,
    /// Day of week (0-6, where 0 is Monday)
    pub day_of_week: u32,
    /// Whether it's a business hour
    pub is_business_hours: bool,
}

impl MessageFeatures {
    /// Extract features from raw HL7 message bytes.
    pub fn from_bytes(bytes: &[u8], sender_id: impl Into<String>) -> Self {
        let sender_id = sender_id.into();
        let timestamp = Utc::now();
        let message_hash = calculate_hash(bytes);
        let (message_type, control_id) = extract_msh_fields(bytes);
        let hour_of_day = timestamp.hour();
        let day_of_week = timestamp.weekday().num_days_from_monday();
        let is_business_hours = (8..18).contains(&hour_of_day) && day_of_week < 5;

        Self {
            message_hash,
            sender_id,
            timestamp,
            message_type,
            control_id,
            hour_of_day,
            day_of_week,
            is_business_hours,
        }
    }
}

/// Calculate SHA-256 hash of message bytes.
pub fn calculate_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Extract MSH fields from HL7 message bytes.
fn extract_msh_fields(bytes: &[u8]) -> (String, String) {
    // Convert to string for parsing, fallback to empty if invalid UTF-8
    let content = String::from_utf8_lossy(bytes);
    let mut message_type = String::new();
    let mut control_id = String::new();

    // Find MSH segment
    for line in content.lines() {
        if line.starts_with("MSH|") {
            let fields: Vec<&str> = line.split('|').collect();
            // Message type is typically at field 9 (index 8)
            if fields.len() > 8 {
                message_type = fields[8].to_string();
            }
            // Control ID is typically at field 10 (index 9)
            if fields.len() > 9 {
                control_id = fields[9].to_string();
            }
            break;
        }
    }

    (message_type, control_id)
}

/// Multi-factor anomaly score combining multiple detection signals.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AnomalyScore {
    /// Volume anomaly score (0.0-1.0)
    pub volume_score: f64,
    /// Timing anomaly score (0.0-1.0)
    pub timing_score: f64,
    /// Sender behavior anomaly score (0.0-1.0)
    pub sender_score: f64,
    /// Pattern anomaly score (0.0-1.0)
    pub pattern_score: f64,
}

impl AnomalyScore {
    /// Calculate combined score using weighted average.
    pub fn combined(&self) -> f64 {
        0.4 * self.volume_score
            + 0.2 * self.timing_score
            + 0.2 * self.sender_score
            + 0.2 * self.pattern_score
    }

    /// Determine severity based on combined score.
    pub fn severity(&self, _z_threshold: f64) -> Severity {
        let combined = self.combined();
        // Map Z-score threshold to expected score range
        // A Z-score of 3.0 typically maps to a high anomaly score
        // We expect combined score around 0.5-1.0 for anomalies

        if combined > 0.8 {
            Severity::Critical
        } else if combined > 0.5 {
            Severity::Warning
        } else if combined > 0.2 {
            Severity::Info
        } else {
            Severity::None
        }
    }
}

/// The main Guard struct for anomaly detection.
#[derive(Debug)]
pub struct Guard {
    config: GuardConfig,
    baselines: Arc<RwLock<HashMap<String, SenderBaseline>>>,
    pattern_detector: Arc<RwLock<PatternDetector>>,
}

impl Guard {
    /// Create a new Guard with the given configuration.
    pub fn new(config: GuardConfig) -> Self {
        let pattern_detector = PatternDetector::new(config.duplicate_window, config.baseline_days);

        Self {
            config,
            baselines: Arc::new(RwLock::new(HashMap::new())),
            pattern_detector: Arc::new(RwLock::new(pattern_detector)),
        }
    }

    /// Analyze a message for anomalies.
    pub fn analyze(&self, features: &MessageFeatures) -> Result<AnomalyResult, GuardError> {
        if !self.config.enabled {
            return Err(GuardError::Disabled);
        }

        let mut reasons = Vec::new();
        let mut score = AnomalyScore::default();

        // Get or create baseline for sender
        let baseline = self.get_or_create_baseline(&features.sender_id);

        // Check for volume spike
        let volume_z = baseline.volume_stats.z_score(1.0); // Current observation
        if detect_volume_spike(volume_z, self.config.z_threshold) {
            score.volume_score = (volume_z.abs() / 5.0).min(1.0); // Normalize to 0-1
            reasons.push(format!("Volume spike detected (Z-score: {:.2})", volume_z));
        }

        // Check for timing anomaly (off-hours)
        if !features.is_business_hours {
            // Check if sender has off-hours pattern in baseline
            let off_hours_ratio = baseline.off_hours_ratio();
            if off_hours_ratio < 0.1 {
                // Sender rarely sends during off-hours
                score.timing_score = 0.7;
                reasons.push("Off-hours access detected".to_string());
            }
        }

        // Check for unexpected message type
        if !baseline.is_expected_message_type(&features.message_type) {
            score.sender_score = 0.6;
            reasons.push(format!(
                "Unexpected message type from sender: {}",
                features.message_type
            ));
        }

        // Pattern detection
        let mut detector = self.pattern_detector.write().unwrap();
        let pattern_result = detector.check_patterns(features)?;
        if pattern_result.is_duplicate {
            score.pattern_score = 0.5;
            reasons.push("Duplicate message detected".to_string());
        }
        if pattern_result.is_replay {
            score.pattern_score = 1.0;
            reasons.push("Replay attack detected".to_string());
        }

        // Record message for future duplicate/replay detection
        detector.record_message(features);

        // Determine severity and action
        let severity = if reasons.is_empty() {
            Severity::None
        } else {
            score.severity(self.config.z_threshold)
        };

        let action_taken = if severity == Severity::None {
            GuardAction::Allow
        } else {
            self.config.action
        };

        // Update baseline with this observation
        self.update_baseline(&features.sender_id, features);

        Ok(AnomalyResult {
            message_id: features.control_id.clone(),
            timestamp: Utc::now(),
            score,
            severity,
            reasons,
            action_taken,
            is_duplicate: pattern_result.is_duplicate,
            is_replay: pattern_result.is_replay,
        })
    }

    /// Get or create a baseline for a sender.
    fn get_or_create_baseline(&self, sender_id: &str) -> SenderBaseline {
        let baselines = self.baselines.read().unwrap();
        if let Some(baseline) = baselines.get(sender_id) {
            return baseline.clone();
        }
        drop(baselines);

        // Create new baseline
        let mut baselines = self.baselines.write().unwrap();
        baselines
            .entry(sender_id.to_string())
            .or_insert_with(|| SenderBaseline::new(sender_id, self.config.baseline_days))
            .clone()
    }

    /// Update baseline with a new observation.
    fn update_baseline(&self, sender_id: &str, features: &MessageFeatures) {
        let mut baselines = self.baselines.write().unwrap();
        if let Some(baseline) = baselines.get_mut(sender_id) {
            baseline.update(features);
        }
    }

    /// Get current baseline stats for a sender.
    pub fn get_sender_stats(&self, sender_id: &str) -> Option<SenderBaseline> {
        let baselines = self.baselines.read().unwrap();
        baselines.get(sender_id).cloned()
    }

    /// Reset baseline for a sender.
    pub fn reset_baseline(&self, sender_id: &str) {
        let mut baselines = self.baselines.write().unwrap();
        baselines.remove(sender_id);
    }

    /// Get the guard configuration.
    pub fn config(&self) -> &GuardConfig {
        &self.config
    }
}

/// Detect volume spike using Z-score.
pub fn detect_volume_spike(z_score: f64, threshold: f64) -> bool {
    z_score.abs() > threshold
}

/// Calculate Z-score for a value.
pub fn calculate_z_score(value: f64, mean: f64, stddev: f64) -> f64 {
    if stddev == 0.0 {
        0.0
    } else {
        (value - mean) / stddev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_hash() {
        let data = b"MSH|^~\\&|TEST";
        let hash1 = calculate_hash(data);
        let hash2 = calculate_hash(data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex string

        let hash3 = calculate_hash(b"different data");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_calculate_z_score() {
        assert_eq!(calculate_z_score(10.0, 5.0, 2.5), 2.0);
        assert_eq!(calculate_z_score(5.0, 5.0, 2.5), 0.0);
        assert_eq!(calculate_z_score(0.0, 5.0, 0.0), 0.0); // Handle zero stddev
    }

    #[test]
    fn test_detect_volume_spike() {
        assert!(detect_volume_spike(3.5, 3.0));
        assert!(!detect_volume_spike(2.5, 3.0));
        assert!(detect_volume_spike(-3.5, 3.0)); // Both directions
    }

    #[test]
    fn test_anomaly_score_combined() {
        let score = AnomalyScore {
            volume_score: 0.5,
            timing_score: 0.5,
            sender_score: 0.5,
            pattern_score: 0.5,
        };
        assert_eq!(score.combined(), 0.5);
    }

    #[test]
    fn test_anomaly_score_severity() {
        // Combined = 0.4 * 0.9 = 0.36, which is Info (0.2 < score < 0.5)
        let score = AnomalyScore {
            volume_score: 0.9,
            timing_score: 0.0,
            sender_score: 0.0,
            pattern_score: 0.0,
        };
        assert_eq!(score.severity(3.0), Severity::Info);

        // Combined = 1.0, which is Critical (> 0.8)
        let score = AnomalyScore {
            volume_score: 1.0,
            timing_score: 1.0,
            sender_score: 1.0,
            pattern_score: 1.0,
        };
        assert_eq!(score.severity(3.0), Severity::Critical);

        // Combined = 0.4 * 0.5 + 0.2 * 0.5 = 0.3, which is Info
        let score = AnomalyScore {
            volume_score: 0.5,
            timing_score: 0.5,
            sender_score: 0.0,
            pattern_score: 0.0,
        };
        assert_eq!(score.severity(3.0), Severity::Info);

        // Combined = 0.52, which is Warning (> 0.5)
        let score = AnomalyScore {
            volume_score: 1.0,
            timing_score: 0.6,
            sender_score: 0.0,
            pattern_score: 0.0,
        };
        assert_eq!(score.severity(3.0), Severity::Warning);
    }

    #[test]
    fn test_extract_msh_fields() {
        let msg = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1";
        let (msg_type, control_id) = extract_msh_fields(msg);
        assert_eq!(msg_type, "ADT^A01");
        assert_eq!(control_id, "ABC123");
    }

    #[test]
    fn test_guard_disabled() {
        let config = GuardConfig::builder().enabled(false).build();
        let guard = Guard::new(config);
        let features = MessageFeatures::from_bytes(b"test", "sender1");
        assert!(matches!(
            guard.analyze(&features),
            Err(GuardError::Disabled)
        ));
    }
}
