use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use hl7v2_server::{server::AppState, routes::build_router};
use std::sync::Arc;
use std::time::Instant;

mod common;

#[tokio::test]
async fn test_small_limit_rejects_large_body() {
    // Set a very small limit (10 bytes)
    let limit = 10;

    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
    });
    let app = build_router(state, limit);

    // Body larger than 10 bytes
    let body = "This is definitely more than 10 bytes";

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/hl7/parse") // Use a real endpoint
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_large_limit_allows_large_body() {
    // Set limit to 5MB (default Axum is 2MB)
    let limit = 5 * 1024 * 1024;

    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
    });
    let app = build_router(state, limit);

    // Create a 3MB body (larger than default 2MB)
    let large_padding = "a".repeat(3 * 1024 * 1024);

    // Construct a valid JSON request
    let request_body = serde_json::json!({
        "message": "MSH|^~\\&|...",
        "mllp_framed": false,
        "padding": large_padding
    });
    let body_str = serde_json::to_string(&request_body).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/hl7/parse")
                .header("Content-Type", "application/json")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should NOT be 413.
    // If the limit was still 2MB, this would return 413.
    // If the limit is 5MB, this should be accepted (and fail later with parse error or succeed).
    assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "Should accept 3MB body with 5MB limit");
}
