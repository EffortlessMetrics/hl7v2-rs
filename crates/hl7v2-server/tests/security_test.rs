//! Security integration tests.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_auth_missing_api_key_returns_401() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    // Make request WITHOUT API Key
    let response = app
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

    // Currently this fails (returns 200), demonstrating the vulnerability
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request without API key should be unauthorized"
    );
}

#[tokio::test]
async fn test_auth_invalid_api_key_returns_401() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    // Make request WITH INVALID API Key
    let response = app
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

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request with invalid API key should be unauthorized"
    );
}

#[tokio::test]
async fn test_auth_valid_api_key_returns_200() {
    // We need to set the env var for the test router to pick it up (if using env var)
    // Or if we move to AppState, we need to ensure create_test_router sets it.
    // For now, let's assume we'll fix create_test_router too.
    unsafe { std::env::set_var("HL7V2_API_KEY", "test-secret-key") };

    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    // Make request WITH VALID API Key
    let response = app
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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Request with valid API key should be authorized"
    );
}
