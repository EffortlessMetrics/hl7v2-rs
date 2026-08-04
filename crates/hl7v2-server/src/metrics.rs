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
//! - `hl7v2_parse_failures_total`: Parse failures by bounded operation label
//! - `hl7v2_validation_failures_total`: Validation failures by bounded operation label
//! - `hl7v2_redaction_failures_total`: Redaction failures by bounded operation label
//! - `hl7v2_bundles_created_total`: Evidence bundles created by the server
//! - `hl7v2_replays_total`: Evidence replay attempts
//! - `hl7v2_replay_failures_total`: Evidence replay attempts that did not reproduce
//! - `hl7v2_corpus_diffs_total`: Inline corpus diff requests completed by the server
//! - `hl7v2_parse_errors_total`: Compatibility parse error counter
//! - `hl7v2_validation_errors_total`: Compatibility validation error counter
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
//! metrics::record_parse_success(metrics::operation::PARSE, 256);
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

/// Bounded operation labels for evidence workflow metrics.
///
/// These labels are deliberately static and low-cardinality. Do not use raw
/// request data, profile names, message IDs, bundle IDs, file paths, or route
/// parameters as metric labels.
pub mod operation {
    /// `POST /hl7/ack`.
    pub const ACK: &str = "ack";
    /// `POST /hl7/ack-policy`.
    pub const ACK_POLICY: &str = "ack_policy";
    /// `POST /hl7/bundle`.
    pub const BUNDLE: &str = "bundle";
    /// `POST /hl7/corpus/diff`.
    pub const CORPUS_DIFF: &str = "corpus_diff";
    /// `POST /hl7/normalize`.
    pub const NORMALIZE: &str = "normalize";
    /// `POST /hl7/parse`.
    pub const PARSE: &str = "parse";
    /// `POST /hl7/replay`.
    pub const REPLAY: &str = "replay";
    /// `POST /hl7/validate`.
    pub const VALIDATE: &str = "validate";
    /// `POST /hl7/validate-redacted`.
    pub const VALIDATE_REDACTED: &str = "validate_redacted";
}

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
            describe_metrics();

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

fn describe_metrics() {
    metrics::describe_counter!(
        "hl7v2_requests_total",
        "Total HTTP requests by endpoint and HTTP status code."
    );
    metrics::describe_histogram!(
        "hl7v2_request_duration_seconds",
        "HTTP request latency in seconds by endpoint."
    );
    metrics::describe_counter!(
        "hl7v2_messages_parsed_total",
        "HL7 messages parsed successfully by server evidence workflows."
    );
    metrics::describe_counter!(
        "hl7v2_messages_validated_total",
        "HL7 messages validated by server evidence workflows."
    );
    metrics::describe_histogram!(
        "hl7v2_message_size_bytes",
        "Input HL7 message size in bytes."
    );
    metrics::describe_counter!(
        "hl7v2_parse_failures_total",
        "Parse failures by bounded server operation label."
    );
    metrics::describe_counter!(
        "hl7v2_validation_failures_total",
        "Validation failures by bounded server operation label."
    );
    metrics::describe_counter!(
        "hl7v2_redaction_failures_total",
        "Redaction failures by bounded server operation label."
    );
    metrics::describe_counter!(
        "hl7v2_bundles_created_total",
        "Evidence bundles created by the server."
    );
    metrics::describe_counter!("hl7v2_replays_total", "Evidence replay attempts.");
    metrics::describe_counter!(
        "hl7v2_replay_failures_total",
        "Evidence replay attempts that did not reproduce."
    );
    metrics::describe_counter!(
        "hl7v2_corpus_diffs_total",
        "Inline corpus diff requests completed by the server."
    );
    metrics::describe_counter!(
        "hl7v2_parse_errors_total",
        "Compatibility counter for parse errors."
    );
    metrics::describe_counter!(
        "hl7v2_validation_errors_total",
        "Compatibility counter for validation errors."
    );
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
    record_request_with_shared_labels(
        Arc::<str>::from(endpoint),
        Arc::<str>::from(status),
        duration_seconds,
    );
}

fn record_request_with_shared_labels(endpoint: Arc<str>, status: Arc<str>, duration_seconds: f64) {
    metrics::counter!(
        "hl7v2_requests_total",
        "endpoint" => Arc::clone(&endpoint),
        "status" => status
    )
    .increment(1);

    metrics::histogram!(
        "hl7v2_request_duration_seconds",
        "endpoint" => endpoint
    )
    .record(duration_seconds);
}

/// Record a successfully parsed HL7 message for an evidence workflow.
pub fn record_parse_success(operation: &'static str, size_bytes: usize) {
    metrics::counter!("hl7v2_messages_parsed_total", "operation" => operation).increment(1);
    record_message_size(size_bytes);
}

/// Record a parse failure for an evidence workflow.
pub fn record_parse_failure(operation: &'static str) {
    metrics::counter!("hl7v2_parse_failures_total", "operation" => operation).increment(1);
    metrics::counter!("hl7v2_parse_errors_total").increment(1);
}

/// Record a validation result for an evidence workflow.
pub fn record_validation_result(operation: &'static str, valid: bool) {
    metrics::counter!("hl7v2_messages_validated_total", "operation" => operation).increment(1);

    if !valid {
        metrics::counter!("hl7v2_validation_failures_total", "operation" => operation).increment(1);
        metrics::counter!("hl7v2_validation_errors_total").increment(1);
    }
}

/// Record a redaction failure for an evidence workflow.
pub fn record_redaction_failure(operation: &'static str) {
    metrics::counter!("hl7v2_redaction_failures_total", "operation" => operation).increment(1);
}

/// Record a successfully written evidence bundle.
pub fn record_bundle_created() {
    metrics::counter!("hl7v2_bundles_created_total").increment(1);
}

/// Record an evidence replay attempt.
pub fn record_replay_result(reproduced: bool) {
    metrics::counter!("hl7v2_replays_total").increment(1);

    if !reproduced {
        metrics::counter!("hl7v2_replay_failures_total").increment(1);
    }
}

/// Record a completed inline corpus diff request.
pub fn record_corpus_diff() {
    metrics::counter!("hl7v2_corpus_diffs_total").increment(1);
}

/// Increment the count of successfully parsed messages
pub fn increment_messages_parsed() {
    metrics::counter!("hl7v2_messages_parsed_total", "operation" => "unspecified").increment(1);
}

/// Increment the count of validated messages
pub fn increment_messages_validated() {
    metrics::counter!("hl7v2_messages_validated_total", "operation" => "unspecified").increment(1);
}

/// Increment the count of validation errors
pub fn increment_validation_errors() {
    record_validation_result("unspecified", false);
}

/// Increment the count of parse errors
pub fn increment_parse_errors() {
    record_parse_failure("unspecified");
}

/// Record message size in bytes
pub fn record_message_size(size_bytes: usize) {
    metrics::histogram!("hl7v2_message_size_bytes").record(message_size_for_histogram(size_bytes));
}

fn message_size_for_histogram(size_bytes: usize) -> f64 {
    f64::from(u32::try_from(size_bytes).unwrap_or(u32::MAX))
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
        let path = Arc::<str>::from(request.uri().path());

        // Process the request
        let response = next.run(request).await;

        // Record metrics
        let duration = start.elapsed();
        let status = Arc::<str>::from(response.status().as_u16().to_string());

        record_request_with_shared_labels(path, status, duration.as_secs_f64());

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
        record_redaction_failure(operation::VALIDATE_REDACTED);
        record_bundle_created();
        record_replay_result(false);
        record_corpus_diff();
        // No panic means success
    }

    #[test]
    fn test_record_message_size() {
        // Test recording message size
        record_message_size(1024);
        // No panic means success
    }

    #[test]
    fn message_size_histogram_value_is_bounded() {
        assert_eq!(
            message_size_for_histogram(1024).to_bits(),
            1024.0f64.to_bits()
        );
        assert_eq!(
            message_size_for_histogram(usize::MAX).to_bits(),
            f64::from(u32::MAX).to_bits()
        );
    }

    #[test]
    fn test_evidence_contract_metrics_render() {
        let handle = init_metrics_recorder();

        record_request("/hl7/validate", "200", 0.015);
        record_parse_success(operation::PARSE, 256);
        record_parse_failure(operation::VALIDATE);
        record_validation_result(operation::VALIDATE, false);
        record_redaction_failure(operation::BUNDLE);
        record_bundle_created();
        record_replay_result(false);
        record_corpus_diff();

        let output = handle.render();
        for metric_name in [
            "hl7v2_requests_total",
            "hl7v2_request_duration_seconds",
            "hl7v2_messages_parsed_total",
            "hl7v2_messages_validated_total",
            "hl7v2_message_size_bytes",
            "hl7v2_parse_failures_total",
            "hl7v2_validation_failures_total",
            "hl7v2_redaction_failures_total",
            "hl7v2_bundles_created_total",
            "hl7v2_replays_total",
            "hl7v2_replay_failures_total",
            "hl7v2_corpus_diffs_total",
        ] {
            assert!(
                output.contains(metric_name),
                "Prometheus output should include {metric_name}; output was: {output}"
            );
        }
    }
}
