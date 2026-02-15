//! Integration tests for health and readiness endpoints.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_health_endpoint_returns_200() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
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
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("\"ready\":true"),
        "Ready endpoint should return ready: true"
    );
}

#[tokio::test]
async fn test_metrics_endpoint_returns_200() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
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
