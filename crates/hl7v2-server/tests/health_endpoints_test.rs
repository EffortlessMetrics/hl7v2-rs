//! Integration tests for health and readiness endpoints.

#![expect(
    clippy::unwrap_used,
    clippy::string_slice,
    reason = "legacy endpoint tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_server::models::{ReadinessCheck, ReadinessCheckStatus, ReadinessStatus, ReadyResponse};
use http_body_util::BodyExt;
use std::{sync::Arc, time::Instant};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_health_endpoint_returns_200() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint_returns_json() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());

    assert!(
        content_type.is_some() && content_type.unwrap().contains("application/json"),
        "Response should be JSON"
    );
}

#[tokio::test]
async fn test_health_endpoint_contains_status() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("\"status\""),
        "Health response should contain status field"
    );
    assert!(body_str.contains("\"healthy\""), "Status should be healthy");
}

#[tokio::test]
async fn test_health_endpoint_contains_uptime() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("\"uptime_seconds\""),
        "Health response should contain uptime"
    );
}

#[tokio::test]
async fn test_ready_endpoint_returns_200() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ready_endpoint_returns_ready_status() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert!(
        content_type.is_some_and(|value| value.contains("application/json")),
        "Readiness response should be JSON"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let ready: ReadyResponse = serde_json::from_slice(&body).unwrap();

    assert!(ready.ready, "Ready endpoint should return ready: true");
    assert_eq!(ready.status, ReadinessStatus::Ready);
    assert!(
        ready
            .checks
            .iter()
            .any(|check| check.name == "validation_report"
                && check.status == ReadinessCheckStatus::Pass),
        "Ready response should include validation report self-check"
    );
}

#[tokio::test]
async fn test_ready_endpoint_returns_503_when_startup_check_failed() {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(hl7v2_server::AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: None,
        cors_allowed_origins: Default::default(),
        readiness_checks: vec![ReadinessCheck::fail(
            "configured_profiles",
            "profile missing-profile.yaml could not be read",
        )],
        bundle_output_root: None,
        ack_policy: Default::default(),
        quarantine: Default::default(),
    });
    let app = hl7v2_server::build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let ready: ReadyResponse = serde_json::from_slice(&body).unwrap();
    assert!(!ready.ready);
    assert_eq!(ready.status, ReadinessStatus::NotReady);
    assert_eq!(
        ready.checks.first().map(|check| &check.status),
        Some(&ReadinessCheckStatus::Fail)
    );
}

#[tokio::test]
async fn test_ready_endpoint_does_not_expose_configured_profile_path_when_not_ready() {
    let profile_path = std::env::temp_dir()
        .join("hl7v2-sensitive-profile-root")
        .join("missing-profile.yaml")
        .display()
        .to_string();
    let config = hl7v2_server::ServerConfig {
        profile_paths: vec![profile_path.clone()],
        ..Default::default()
    };
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(hl7v2_server::AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: None,
        cors_allowed_origins: Default::default(),
        readiness_checks: config.readiness_checks(),
        bundle_output_root: None,
        ack_policy: Default::default(),
        quarantine: Default::default(),
    });
    let app = hl7v2_server::build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body_text.contains(&profile_path));
    assert!(!body_text.contains("missing-profile.yaml"));

    let ready: ReadyResponse = serde_json::from_str(&body_text).unwrap();
    assert!(!ready.ready);
    let check = ready
        .checks
        .iter()
        .find(|check| check.name == "configured_profiles");
    assert!(check.is_some(), "configured profile check should exist");
    let check = check.unwrap();
    assert_eq!(check.status, ReadinessCheckStatus::Fail);
    assert!(check.message.contains("configured profile 1"));
}

#[tokio::test]
async fn test_metrics_endpoint_returns_200() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_endpoint_returns_prometheus_format() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Prometheus metrics might be empty if no requests have been made yet,
    // or might contain metric definitions. Either case is valid.
    // The important thing is that the endpoint responds successfully.
    assert!(
        body_str.contains("# HELP")
            || body_str.contains("# TYPE")
            || body_str.is_empty()
            || body_str.contains("hl7v2_"),
        "Metrics should be in Prometheus format, empty, or contain hl7v2 metrics. Got: {}",
        if body_str.len() > 200 {
            &body_str[..200]
        } else {
            &body_str
        }
    );
}

#[tokio::test]
async fn test_metrics_endpoint_exposes_evidence_contract_metrics() {
    let app = common::create_test_router();

    hl7v2_server::metrics::record_request("/hl7/validate", "200", 0.015);
    hl7v2_server::metrics::record_parse_success(hl7v2_server::metrics::operation::PARSE, 128);
    hl7v2_server::metrics::record_parse_failure(hl7v2_server::metrics::operation::VALIDATE);
    hl7v2_server::metrics::record_validation_result(
        hl7v2_server::metrics::operation::VALIDATE,
        false,
    );
    hl7v2_server::metrics::record_redaction_failure(
        hl7v2_server::metrics::operation::VALIDATE_REDACTED,
    );
    hl7v2_server::metrics::record_bundle_created();
    hl7v2_server::metrics::record_replay_result(false);
    hl7v2_server::metrics::record_corpus_diff();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

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
            body_str.contains(metric_name),
            "metrics response should include {metric_name}; body was: {body_str}"
        );
    }

    for forbidden in ["MRN123", "Doe", "John", "PID|"] {
        assert!(
            !body_str.contains(forbidden),
            "metrics response must not contain fixture PHI sentinel {forbidden}"
        );
    }
}
