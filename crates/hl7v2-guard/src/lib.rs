//! ML-based anomaly detection for HL7 messages.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Configuration for the guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    /// Whether anomaly detection is enabled
    pub enabled: bool,
    /// Threshold for anomaly score (0.0 to 1.0)
    pub threshold: f32,
    /// Minimum samples required before alerting
    pub warmup_samples: usize,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.8,
            warmup_samples: 50,
        }
    }
}

/// Result of an anomaly check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    /// Whether the message is considered an anomaly
    pub is_anomaly: bool,
    /// Score indicating the degree of anomaly (0.0 to 1.0)
    pub score: f32,
    /// Reason for the anomaly classification
    pub reason: Option<String>,
}

/// Statistics for a message feature
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FeatureStats {
    count: usize,
    mean: f64,
    m2: f64,
}

impl FeatureStats {
    fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
}

/// The guard engine (currently using statistical baseline detection)
pub struct Guard {
    config: GuardConfig,
    length_stats: Arc<RwLock<FeatureStats>>,
}

impl Guard {
    /// Create a new guard with the given configuration
    pub fn new(config: GuardConfig) -> Self {
        Self {
            config,
            length_stats: Arc::new(RwLock::new(FeatureStats::default())),
        }
    }

    /// Analyze a message for anomalies and update baseline
    pub fn analyze(&self, message_bytes: &[u8]) -> AnomalyResult {
        if !self.config.enabled {
            return AnomalyResult {
                is_anomaly: false,
                score: 0.0,
                reason: None,
            };
        }

        let len = message_bytes.len() as f64;
        let stats = self.length_stats.read().unwrap();

        let mut result = AnomalyResult {
            is_anomaly: false,
            score: 0.0,
            reason: None,
        };

        // Only alert after warmup
        if stats.count >= self.config.warmup_samples {
            let std_dev = stats.std_dev();
            if std_dev > 0.0 {
                let z_score = (len - stats.mean).abs() / std_dev;
                // Normalize z-score to 0..1 range (rough approximation)
                let score = (z_score / 10.0).min(1.0) as f32;
                result.score = score;

                if score >= self.config.threshold {
                    result.is_anomaly = true;
                    result.reason = Some(format!(
                        "Message length anomaly: {} bytes (mean: {:.1}, std_dev: {:.1})",
                        len, stats.mean, std_dev
                    ));
                }
            }
        }

        // Update baseline
        drop(stats);
        let mut stats = self.length_stats.write().unwrap();
        stats.update(len);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_learning_and_alert() {
        let config = GuardConfig {
            enabled: true,
            threshold: 0.5,
            warmup_samples: 10,
        };
        let guard = Guard::new(config);

        // Warmup with consistent messages (but some slight variation to ensure std_dev > 0)
        for i in 0..10 {
            let msg: &[u8] = if i % 2 == 0 { b"NORMAL" } else { b"NORMAL+" };
            let res = guard.analyze(msg);
            assert!(!res.is_anomaly);
        }

        // Anomaly: extremely long message
        let anomaly_msg = vec![b'A'; 1000];
        let res = guard.analyze(&anomaly_msg);

        assert!(res.is_anomaly);
        assert!(res.score > 0.5);
        assert!(res.reason.unwrap().contains("length anomaly"));
    }

    #[test]
    fn test_guard_disabled() {
        let config = GuardConfig {
            enabled: false,
            ..Default::default()
        };
        let guard = Guard::new(config);

        let res = guard.analyze(b"ANYTHING");
        assert!(!res.is_anomaly);
        assert_eq!(res.score, 0.0);
    }
}
