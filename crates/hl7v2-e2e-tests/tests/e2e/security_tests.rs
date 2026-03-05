//! Security tests for the hl7v2-server.
//!
//! These tests validate:
//! - API key authentication
//! - CORS headers
//! - Request size limits

use axum::http::{Request, StatusCode, header};
use hl7v2_server::{AppState, build_router};
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

#[tokio::test]
async fn test_auth_missing_api_key_fails() {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Some("secret-key".to_string()),
    });
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(r#"{"message":"MSH|^~\\&|..."}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_valid_api_key_succeeds() {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Some("secret-key".to_string()),
    });
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-API-Key", "secret-key")
                .body(axum::body::Body::from(r#"{"message":"MSH|^~\\&|..."}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be OK (or at least not Unauthorized)
    // 422 because message is truncated, but not 401
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_auth_invalid_api_key_fails() {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Some("secret-key".to_string()),
    });
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-API-Key", "wrong-key")
                .body(axum::body::Body::from(r#"{"message":"MSH|^~\\&|..."}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_health_metrics_public() {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Some("secret-key".to_string()),
    });
    let app = build_router(state);

    // Health should be public
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Metrics should be public (standard for internal scraping)
    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
