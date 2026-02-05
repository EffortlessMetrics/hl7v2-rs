use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_server::{routes::build_router, server::AppState};
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt; // For `oneshot`

#[tokio::test]
async fn test_parse_unauthorized() {
    // Setup
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: "test-key".to_string(),
    });

    let app = build_router(state);

    // Request to /hl7/parse WITHOUT API Key
    let request = Request::builder()
        .uri("/hl7/parse")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_parse_invalid_key() {
    // Setup
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: "test-key".to_string(),
    });

    let app = build_router(state);

    // Request to /hl7/parse WITH INVALID API Key
    let request = Request::builder()
        .uri("/hl7/parse")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-API-Key", "wrong-key")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_parse_authorized() {
    // Setup
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: "test-key".to_string(),
    });

    let app = build_router(state);

    // Request to /hl7/parse WITH VALID API Key
    // Note: We send empty body so we expect 400 Bad Request (Validation Error or JSON error),
    // NOT 401 Unauthorized.
    let request = Request::builder()
        .uri("/hl7/parse")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-API-Key", "test-key")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    // It will likely be 422 Unprocessable Entity or 400 Bad Request
    assert!(response.status().is_client_error());
}
