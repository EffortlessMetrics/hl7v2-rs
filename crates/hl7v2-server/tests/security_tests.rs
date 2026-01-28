use hl7v2_server::{routes::build_router, server::AppState};
use std::sync::Arc;
use std::time::Instant;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // For `oneshot`

#[tokio::test]
async fn test_auth_enforcement() {
    // Set up the environment
    // SAFETY: This is a test, and we're setting the environment variable for the test process.
    // In a real multi-threaded test environment this could be race-y, but for this specific test it's acceptable.
    unsafe {
        std::env::set_var("HL7V2_API_KEY", "test-secret-key");
    }

    // Initialize state
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
    });

    let app = build_router(state);

    // We need a valid body so it doesn't fail on parsing
    let hl7_message = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|123456|P|2.5\rPID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M\r";
    let request_body = serde_json::json!({
        "message": hl7_message,
        "mllp_framed": false,
        "options": {
            "include_json": true,
            "validate_structure": true
        }
    });

    // 1. Request WITHOUT API key
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "Security check: Should be 401 UNAUTHORIZED when auth is missing");

    // 2. Request WITH API key
    let response_auth = app.clone()
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "test-secret-key")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response_auth.status(), StatusCode::OK, "Security check: Should be 200 OK when correct auth is provided");

    // 3. Request WITH INVALID API key
    let response_invalid = app.clone()
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "wrong-key")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response_invalid.status(), StatusCode::UNAUTHORIZED, "Security check: Should be 401 UNAUTHORIZED when wrong auth is provided");
}
