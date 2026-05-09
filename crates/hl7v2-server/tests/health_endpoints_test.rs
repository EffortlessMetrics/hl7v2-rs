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
