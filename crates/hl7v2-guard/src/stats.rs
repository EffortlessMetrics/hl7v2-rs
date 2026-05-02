//! Statistical types and calculations for anomaly detection.

use crate::MessageFeatures;
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, HashSet, VecDeque};

/// Timestamped value for time-series data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimestampedValue {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

impl TimestampedValue {
    /// Create a new timestamped value with current time.
    pub fn now(value: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            value,
        }
    }

    /// Get age of the value.
    pub fn age(&self) -> Duration {
        Utc::now() - self.timestamp
    }
}

/// Time-series statistics with sliding window.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeriesStats {
    /// Window duration.
    pub window: Duration,
    /// Timestamped values.
    pub values: VecDeque<TimestampedValue>,
}

impl TimeSeriesStats {
    /// Create new time-series stats with given window.
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            values: VecDeque::new(),
        }
    }

    /// Add a value and clean up expired entries.
    pub fn add(&mut self, value: f64) {
        self.values.push_back(TimestampedValue::now(value));
        self.cleanup();
    }

    /// Clean up values outside the window.
    pub fn cleanup(&mut self) {
        self.values.retain(|v| v.age() < self.window);
    }

    /// Calculate mean.
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.values.iter().map(|v| v.value).sum();
        sum / self.values.len() as f64
    }

    /// Calculate standard deviation.
    pub fn stddev(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance: f64 = self
            .values
            .iter()
            .map(|v| (v.value - mean).powi(2))
            .sum::<f64>()
            / (self.values.len() - 1) as f64;
        variance.sqrt()
    }

    /// Calculate Z-score for a value.
    pub fn z_score(&self, value: f64) -> f64 {
        let mean = self.mean();
        let stddev = self.stddev();
        if stddev == 0.0 {
            0.0
        } else {
            (value - mean) / stddev
        }
    }

    /// Get count of values.
    pub fn count(&self) -> usize {
        self.values.len()
    }
}

/// Streaming statistics using Welford's online algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StreamingStats {
    count: usize,
    mean: f64,
    m2: f64, // Sum of squares of differences from the current mean
}

impl StreamingStats {
    /// Create new streaming stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update with a new value.
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    /// Get mean.
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Get variance.
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    /// Get standard deviation.
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Get count.
    pub fn count(&self) -> usize {
        self.count
    }
}

/// Per-sender baseline statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct SenderBaseline {
    /// Sender identifier.
    pub sender_id: String,
    /// Volume statistics (messages per window).
    pub volume_stats: TimeSeriesStats,
    /// Off-hours message count.
    off_hours_count: usize,
    /// Total message count.
    total_count: usize,
    /// Expected message types.
    expected_message_types: HashSet<String>,
    /// Message type distribution.
    message_type_counts: HashMap<String, usize>,
}

impl SenderBaseline {
    /// Create a new sender baseline.
    pub fn new(sender_id: impl Into<String>, baseline_days: u32) -> Self {
        let window = Duration::days(baseline_days as i64);
        Self {
            sender_id: sender_id.into(),
            volume_stats: TimeSeriesStats::new(window),
            off_hours_count: 0,
            total_count: 0,
            expected_message_types: HashSet::new(),
            message_type_counts: HashMap::new(),
        }
    }

    /// Update baseline with a new observation.
    pub fn update(&mut self, features: &MessageFeatures) {
        self.volume_stats.add(1.0);
        self.total_count += 1;

        // Track off-hours ratio
        if !features.is_business_hours {
            self.off_hours_count += 1;
        }

        // Track message types
        self.expected_message_types
            .insert(features.message_type.clone());
        *self
            .message_type_counts
            .entry(features.message_type.clone())
            .or_insert(0) += 1;
    }

    /// Get ratio of off-hours messages.
    pub fn off_hours_ratio(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.off_hours_count as f64 / self.total_count as f64
        }
    }

    /// Check if message type is expected.
    pub fn is_expected_message_type(&self, message_type: &str) -> bool {
        // If no baseline yet, accept all types
        if self.total_count < 10 {
            return true;
        }
        self.expected_message_types.contains(message_type)
    }

    /// Get message type distribution.
    pub fn message_type_distribution(&self) -> HashMap<String, f64> {
        if self.total_count == 0 {
            return HashMap::new();
        }
        self.message_type_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / self.total_count as f64))
            .collect()
    }
}

/// Field-level statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldStats {
    pub field_path: String,
    pub numeric_stats: Option<NumericStats>,
    pub cardinality: usize,
    pub unique_values: HashSet<String>,
}

impl FieldStats {
    /// Create new field stats.
    pub fn new(field_path: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
            numeric_stats: None,
            cardinality: 0,
            unique_values: HashSet::new(),
        }
    }

    /// Update with a value.
    pub fn update(&mut self, value: &str) {
        self.unique_values.insert(value.to_string());
        self.cardinality = self.unique_values.len();

        // Try to parse as numeric
        if let Ok(num) = value.parse::<f64>() {
            let stats = self.numeric_stats.get_or_insert_with(NumericStats::new);
            stats.update(num);
        }
    }
}

/// Numeric field statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericStats {
    pub count: usize,
    pub mean: f64,
    pub m2: f64,
    pub min: f64,
    pub max: f64,
}

impl NumericStats {
    /// Create new numeric stats.
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    /// Update with a value.
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;

        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    /// Get standard deviation.
    pub fn stddev(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            (self.m2 / (self.count - 1) as f64).sqrt()
        }
    }
}

impl Default for NumericStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamped_value_age() {
        let value = TimestampedValue::now(10.0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let age = value.age();
        assert!(age > Duration::zero());
    }

    #[test]
    fn test_time_series_stats() {
        let mut stats = TimeSeriesStats::new(Duration::hours(1));

        stats.add(10.0);
        stats.add(20.0);
        stats.add(30.0);

        assert_eq!(stats.mean(), 20.0);
        assert!(stats.stddev() > 0.0);
        assert_eq!(stats.count(), 3);
    }

    #[test]
    fn test_time_series_cleanup() {
        let mut stats = TimeSeriesStats::new(Duration::milliseconds(1));

        stats.add(10.0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        stats.add(20.0); // This should trigger cleanup

        // First value should be cleaned up
        assert_eq!(stats.count(), 1);
    }

    #[test]
    fn test_streaming_stats() {
        let mut stats = StreamingStats::new();

        stats.update(10.0);
        stats.update(20.0);
        stats.update(30.0);

        assert_eq!(stats.mean(), 20.0);
        assert!(stats.stddev() > 0.0);
        assert_eq!(stats.count(), 3);
    }

    #[test]
    fn test_sender_baseline() {
        let mut baseline = SenderBaseline::new("sender1", 7);

        // Add 10+ messages to establish baseline
        for i in 0..12 {
            let features = MessageFeatures {
                message_hash: format!("hash{}", i),
                sender_id: "sender1".to_string(),
                timestamp: Utc::now(),
                message_type: "ADT^A01".to_string(),
                control_id: format!("ctrl{}", i),
                hour_of_day: 12,
                day_of_week: 1,
                is_business_hours: true,
            };
            baseline.update(&features);
        }

        assert_eq!(baseline.total_count, 12);
        // With enough messages, only ADT^A01 should be expected
        assert!(baseline.is_expected_message_type("ADT^A01"));
        assert!(!baseline.is_expected_message_type("ORM^O01"));
    }

    #[test]
    fn test_sender_baseline_off_hours() {
        let mut baseline = SenderBaseline::new("sender1", 7);

        let business_hours = MessageFeatures {
            message_hash: "hash1".to_string(),
            sender_id: "sender1".to_string(),
            timestamp: Utc::now(),
            message_type: "ADT^A01".to_string(),
            control_id: "ctrl1".to_string(),
            hour_of_day: 12,
            day_of_week: 1,
            is_business_hours: true,
        };

        let off_hours = MessageFeatures {
            message_hash: "hash2".to_string(),
            sender_id: "sender1".to_string(),
            timestamp: Utc::now(),
            message_type: "ADT^A01".to_string(),
            control_id: "ctrl2".to_string(),
            hour_of_day: 2,
            day_of_week: 1,
            is_business_hours: false,
        };

        baseline.update(&business_hours);
        baseline.update(&off_hours);

        assert_eq!(baseline.off_hours_ratio(), 0.5);
    }

    #[test]
    fn test_field_stats() {
        let mut stats = FieldStats::new("PV1-19");

        stats.update("1000");
        stats.update("2000");
        stats.update("3000");
        stats.update("1000"); // Duplicate

        assert_eq!(stats.cardinality, 3);
        assert!(stats.numeric_stats.is_some());

        let numeric = stats.numeric_stats.unwrap();
        assert_eq!(numeric.count, 4);
        assert_eq!(numeric.min, 1000.0);
        assert_eq!(numeric.max, 3000.0);
    }
}
