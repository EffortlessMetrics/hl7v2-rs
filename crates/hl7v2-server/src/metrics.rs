// ! Prometheus metrics for observability and monitoring.
//!
//! This module provides metrics collection and export for the HL7v2 server using
//! the `metrics` crate and Prometheus exporter.
//!
//! ## Metrics Collected
//!
//! - `hl7v2_requests_total`: Total number of HTTP requests by endpoint and status
//! - `hl7v2_request_duration_seconds`: Request duration histogram by endpoint
//! - `hl7v2_messages_parsed_total`: Total number of messages successfully parsed
//! - `hl7v2_messages_validated_total`: Total number of messages validated
//! - `hl7v2_validation_errors_total`: Total number of validation errors
//! - `hl7v2_parse_errors_total`: Total number of parse errors
//!
//! ## Usage
//!
//! ```no_run
//! use hl7v2_server::metrics;
//!
//! // Initialize metrics recorder (call once at startup)
//! let recorder_handle = metrics::init_metrics_recorder();
//!
//! // Record metrics
//! metrics::record_request("/hl7/parse", "200", 0.05);
//! metrics::increment_messages_parsed();
//! ```

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::sync::{Arc, OnceLock};

/// Global metrics handle, initialized once
static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Initialize the Prometheus metrics recorder
///
/// This should be called once at application startup before any metrics are recorded.
/// Returns a handle that can be used to render metrics in Prometheus format.
///
/// # Note
/// This function can be safely called multiple times. The first call will initialize
/// the metrics recorder, and subsequent calls will return a clone of the same handle.
pub fn init_metrics_recorder() -> PrometheusHandle {
    METRICS_HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .set_buckets_for_metric(
                    Matcher::Full("hl7v2_request_duration_seconds".to_string()),
                    &[
                        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                    ],
                )
                .expect("Failed to set histogram buckets")
                .install_recorder()
                .expect("Failed to install Prometheus recorder")
        })
        .clone()
}

/// Record an HTTP request
///
/// Records both a counter for total requests and a histogram for request duration.
///
/// # Arguments
/// * `endpoint` - The endpoint path (e.g., "/hl7/parse")
/// * `status` - The HTTP status code (e.g., "200", "400")
/// * `duration_seconds` - Request duration in seconds
pub fn record_request(endpoint: &str, status: &str, duration_seconds: f64) {
    metrics::counter!("hl7v2_requests_total", "endpoint" => endpoint.to_string(), "status" => status.to_string())
        .increment(1);

    metrics::histogram!(
        "hl7v2_request_duration_seconds",
        "endpoint" => endpoint.to_string()
    )
    .record(duration_seconds);
}

/// Increment the count of successfully parsed messages
pub fn increment_messages_parsed() {
    metrics::counter!("hl7v2_messages_parsed_total").increment(1);
}

/// Increment the count of validated messages
pub fn increment_messages_validated() {
    metrics::counter!("hl7v2_messages_validated_total").increment(1);
}

/// Increment the count of validation errors
pub fn increment_validation_errors() {
    metrics::counter!("hl7v2_validation_errors_total").increment(1);
}

/// Increment the count of parse errors
pub fn increment_parse_errors() {
    metrics::counter!("hl7v2_parse_errors_total").increment(1);
}

/// Record message size in bytes
pub fn record_message_size(size_bytes: usize) {
    metrics::histogram!("hl7v2_message_size_bytes").record(size_bytes as f64);
}

/// Axum handler for GET /metrics
///
/// Returns Prometheus-formatted metrics for scraping.
///
/// Note: This function requires AppState to be passed in, but only uses the metrics_handle.
/// The AppState dependency is required for Axum's type system.
pub async fn metrics_handler(
    State(state): State<Arc<crate::server::AppState>>,
) -> impl IntoResponse {
    let metrics = state.metrics_handle.render();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(metrics)
        .expect("Failed to build metrics response")
}

/// Middleware to record HTTP request metrics
///
/// This can be added as a layer to automatically record all requests.
pub mod middleware {
    use super::*;
    use axum::{extract::Request, middleware::Next, response::Response};
    use std::time::Instant;

    /// Metrics middleware that records request metrics
    pub async fn metrics_middleware(request: Request, next: Next) -> Response {
        let start = Instant::now();
        let path = request.uri().path().to_string();

        // Process the request
        let response = next.run(request).await;

        // Record metrics
        let duration = start.elapsed();
        let status = response.status().as_u16().to_string();

        record_request(&path, &status, duration.as_secs_f64());

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recorder_init() {
        // Test that we can initialize the metrics recorder
        // The OnceLock ensures this works even if called multiple times
        let handle = init_metrics_recorder();

        // Record some metrics so we have output to verify
        record_request("/test", "200", 0.01);

        let output = handle.render();
        // Output should contain at least our request counter
        assert!(output.contains("hl7v2_requests_total"));
    }

    #[test]
    fn test_record_request() {
        // Test recording a request
        record_request("/hl7/parse", "200", 0.05);
        // No panic means success
    }

    #[test]
    fn test_increment_counters() {
        // Test incrementing various counters
        increment_messages_parsed();
        increment_messages_validated();
        increment_validation_errors();
        increment_parse_errors();
        // No panic means success
    }

    #[test]
    fn test_record_message_size() {
        // Test recording message size
        record_message_size(1024);
        // No panic means success
    }
}
