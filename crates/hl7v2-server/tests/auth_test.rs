//! Authentication integration tests.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // For `oneshot`
use hl7v2_server::routes::build_router;
use hl7v2_server::server::AppState;
use std::sync::Arc;
use std::time::Instant;
use hl7v2_server::metrics;

#[tokio::test]
async fn test_auth_success() {
    // Set up the app state
    let metrics_handle = metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Some("test-secret-key".to_string()),
    });

    let app = build_router(state);

    // Create a request WITH the API key header
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "test-secret-key")
                .body(Body::from(r#"{"message": "MSH|^~\\&|..."}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // We expect OK (200) or BAD_REQUEST (400) if body is invalid, but NOT UNAUTHORIZED (401)
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED, "Endpoint should allow access with correct API key!");
}

#[tokio::test]
async fn test_auth_missing_header_rejection() {
    // Set up the app state
    let metrics_handle = metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Some("test-secret-key".to_string()),
    });

    let app = build_router(state);

    // Create a request WITHOUT the API key header
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"message": "MSH|^~\\&|..."}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "Endpoint accessed without API key!");
}
