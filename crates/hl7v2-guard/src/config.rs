//! Configuration types for hl7v2-guard.

use chrono::Duration;

/// Guard action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GuardAction {
    /// Allow the message (no anomaly detected).
    Allow,
    /// Send alert notification but allow message.
    #[default]
    Alert,
    /// Hold message for review (return 202 Accepted).
    Quarantine,
    /// Reject message (return 403 Forbidden).
    Block,
    /// Rate limit sender (return 429 Too Many Requests).
    Throttle,
}

/// Severity levels for anomalies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// No anomaly detected.
    None,
    /// Low severity (Z-score 1.0-2.0).
    Info,
    /// Medium severity (Z-score 2.0-3.0).
    Warning,
    /// High severity (Z-score > 3.0).
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::None => write!(f, "None"),
            Severity::Info => write!(f, "Info"),
            Severity::Warning => write!(f, "Warning"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

/// Guard configuration.
#[derive(Debug, Clone)]
pub struct GuardConfig {
    /// Master enable/disable switch.
    pub enabled: bool,
    /// Baseline learning window in days.
    pub baseline_days: u32,
    /// Z-score threshold for anomaly detection.
    pub z_threshold: f64,
    /// Duplicate detection window.
    pub duplicate_window: Duration,
    /// Default action for detected anomalies.
    pub action: GuardAction,
    /// Webhook URL for alerts (optional).
    pub webhook_url: Option<String>,
    /// Maximum memory for baseline storage in MB.
    pub max_memory_mb: usize,
    /// Business hours start (24-hour format, e.g., 8 for 08:00).
    pub business_hours_start: u32,
    /// Business hours end (24-hour format, e.g., 18 for 18:00).
    pub business_hours_end: u32,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            baseline_days: 7,
            z_threshold: 3.0,
            duplicate_window: Duration::minutes(5),
            action: GuardAction::Alert,
            webhook_url: None,
            max_memory_mb: 100,
            business_hours_start: 8,
            business_hours_end: 18,
        }
    }
}

impl GuardConfig {
    /// Create a builder for GuardConfig.
    pub fn builder() -> GuardConfigBuilder {
        GuardConfigBuilder::default()
    }
}

/// Builder for GuardConfig.
#[derive(Debug, Default)]
pub struct GuardConfigBuilder {
    enabled: Option<bool>,
    baseline_days: Option<u32>,
    z_threshold: Option<f64>,
    duplicate_window: Option<Duration>,
    action: Option<GuardAction>,
    webhook_url: Option<String>,
    max_memory_mb: Option<usize>,
    business_hours_start: Option<u32>,
    business_hours_end: Option<u32>,
}

impl GuardConfigBuilder {
    /// Set enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// Set baseline days.
    pub fn baseline_days(mut self, days: u32) -> Self {
        self.baseline_days = Some(days);
        self
    }

    /// Set Z-score threshold.
    pub fn z_threshold(mut self, threshold: f64) -> Self {
        self.z_threshold = Some(threshold);
        self
    }

    /// Set duplicate detection window.
    pub fn duplicate_window(mut self, window: Duration) -> Self {
        self.duplicate_window = Some(window);
        self
    }

    /// Set default action.
    pub fn action(mut self, action: GuardAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set webhook URL.
    pub fn webhook_url(mut self, url: impl Into<String>) -> Self {
        self.webhook_url = Some(url.into());
        self
    }

    /// Set max memory limit.
    pub fn max_memory_mb(mut self, mb: usize) -> Self {
        self.max_memory_mb = Some(mb);
        self
    }

    /// Set business hours start.
    pub fn business_hours_start(mut self, hour: u32) -> Self {
        self.business_hours_start = Some(hour);
        self
    }

    /// Set business hours end.
    pub fn business_hours_end(mut self, hour: u32) -> Self {
        self.business_hours_end = Some(hour);
        self
    }

    /// Build the GuardConfig.
    pub fn build(self) -> GuardConfig {
        GuardConfig {
            enabled: self.enabled.unwrap_or(true),
            baseline_days: self.baseline_days.unwrap_or(7),
            z_threshold: self.z_threshold.unwrap_or(3.0),
            duplicate_window: self
                .duplicate_window
                .unwrap_or_else(|| Duration::minutes(5)),
            action: self.action.unwrap_or_default(),
            webhook_url: self.webhook_url,
            max_memory_mb: self.max_memory_mb.unwrap_or(100),
            business_hours_start: self.business_hours_start.unwrap_or(8),
            business_hours_end: self.business_hours_end.unwrap_or(18),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_action_default() {
        assert_eq!(GuardAction::default(), GuardAction::Alert);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::None < Severity::Info);
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Critical);
    }

    #[test]
    fn test_config_builder() {
        let config = GuardConfig::builder()
            .enabled(true)
            .baseline_days(14)
            .z_threshold(2.5)
            .action(GuardAction::Block)
            .webhook_url("https://example.com/webhook")
            .max_memory_mb(200)
            .business_hours_start(9)
            .business_hours_end(17)
            .build();

        assert!(config.enabled);
        assert_eq!(config.baseline_days, 14);
        assert_eq!(config.z_threshold, 2.5);
        assert_eq!(config.action, GuardAction::Block);
        assert_eq!(
            config.webhook_url,
            Some("https://example.com/webhook".to_string())
        );
        assert_eq!(config.max_memory_mb, 200);
        assert_eq!(config.business_hours_start, 9);
        assert_eq!(config.business_hours_end, 17);
    }

    #[test]
    fn test_config_defaults() {
        let config = GuardConfig::builder().build();

        assert!(config.enabled);
        assert_eq!(config.baseline_days, 7);
        assert_eq!(config.z_threshold, 3.0);
        assert_eq!(config.action, GuardAction::Alert);
        assert_eq!(config.webhook_url, None);
        assert_eq!(config.max_memory_mb, 100);
        assert_eq!(config.business_hours_start, 8);
        assert_eq!(config.business_hours_end, 18);
    }
}
