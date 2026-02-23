//! Application state.

use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use std::time::Instant;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Server start time for uptime calculation
    pub start_time: Instant,
    /// Prometheus metrics handle
    pub metrics_handle: Arc<PrometheusHandle>,
    /// API Key for authentication
    pub api_key: Option<String>,
}
