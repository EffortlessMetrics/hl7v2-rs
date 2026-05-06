//! Experimental anomaly guard APIs.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Configuration for the guard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    /// Whether anomaly detection is enabled.
    pub enabled: bool,
    /// Threshold for anomaly score.
    ///
    /// Values are expected to be in the inclusive range from `0.0` to `1.0`.
    pub threshold: f32,
    /// Minimum samples required before alerting.
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

/// Result of an anomaly check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    /// Whether the message is considered an anomaly.
    pub is_anomaly: bool,
    /// Score indicating the degree of anomaly.
    ///
    /// Scores are normalized to the inclusive range from `0.0` to `1.0`.
    pub score: f32,
    /// Reason for the anomaly classification.
    pub reason: Option<String>,
}

/// Statistics for a message feature.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FeatureStats {
    count: usize,
    mean: f64,
    m2: f64,
}

impl FeatureStats {
    fn update(&mut self, value: f64) {
        let Some(next_count) = self.count.checked_add(1) else {
            return;
        };

        self.count = next_count;
        let count = usize_to_f64(self.count);
        let delta = value - self.mean;
        self.mean += delta / count;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    fn variance(&self) -> f64 {
        let Some(degrees_of_freedom) = self.count.checked_sub(1) else {
            return 0.0;
        };

        self.m2 / usize_to_f64(degrees_of_freedom)
    }

    fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
}

/// The guard engine, currently using statistical baseline detection.
pub struct Guard {
    config: GuardConfig,
    length_stats: Arc<RwLock<FeatureStats>>,
}

impl Guard {
    /// Create a new guard with the given configuration.
    pub fn new(config: GuardConfig) -> Self {
        Self {
            config,
            length_stats: Arc::new(RwLock::new(FeatureStats::default())),
        }
    }

    /// Analyze a message for anomalies and update the baseline.
    pub fn analyze(&self, message_bytes: &[u8]) -> AnomalyResult {
        if !self.config.enabled {
            return AnomalyResult {
                is_anomaly: false,
                score: 0.0,
                reason: None,
            };
        }

        let len = usize_to_f64(message_bytes.len());
        let stats = self.read_stats();

        let mut result = AnomalyResult {
            is_anomaly: false,
            score: 0.0,
            reason: None,
        };

        if stats.count >= self.config.warmup_samples {
            let std_dev = stats.std_dev();
            if std_dev > 0.0 {
                let z_score = (len - stats.mean).abs() / std_dev;
                let score = normalized_score(z_score);
                result.score = score;

                if score >= self.config.threshold {
                    result.is_anomaly = true;
                    result.reason = Some(format!(
                        "Message length anomaly: {len} bytes (mean: {:.1}, std_dev: {std_dev:.1})",
                        stats.mean
                    ));
                }
            }
        }

        drop(stats);
        self.write_stats().update(len);

        result
    }

    fn read_stats(&self) -> RwLockReadGuard<'_, FeatureStats> {
        match self.length_stats.read() {
            Ok(stats) => stats,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn write_stats(&self) -> RwLockWriteGuard<'_, FeatureStats> {
        match self.length_stats.write() {
            Ok(stats) => stats,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "guard anomaly scores are approximate statistics over message lengths"
)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the public anomaly score type is f32 and the value is clamped to 0.0..=1.0"
)]
fn normalized_score(z_score: f64) -> f32 {
    ((z_score / 10.0).clamp(0.0, 1.0)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_learns_and_alerts() {
        let config = GuardConfig {
            enabled: true,
            threshold: 0.5,
            warmup_samples: 10,
        };
        let guard = Guard::new(config);

        let warmup_messages: [&[u8]; 2] = [b"NORMAL", b"NORMAL+"];
        for msg in warmup_messages.into_iter().cycle().take(10) {
            let res = guard.analyze(msg);
            assert!(!res.is_anomaly);
        }

        let anomaly_msg = vec![b'A'; 1000];
        let res = guard.analyze(&anomaly_msg);

        assert!(res.is_anomaly);
        assert!(res.score > 0.5);
        assert!(
            res.reason
                .as_deref()
                .is_some_and(|reason| reason.contains("length anomaly"))
        );
    }

    #[test]
    fn disabled_guard_reports_no_anomaly() {
        let config = GuardConfig {
            enabled: false,
            ..Default::default()
        };
        let guard = Guard::new(config);

        let res = guard.analyze(b"ANYTHING");
        assert!(!res.is_anomaly);
        assert!(res.score.abs() <= f32::EPSILON);
    }
}
