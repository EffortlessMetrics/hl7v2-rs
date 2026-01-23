//! Integration tests for security features (Authentication).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_endpoint_missing_auth_header() {
    let app = common::create_test_router();

    let request_body = serde_json::json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    // Request without API key
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

    // Should be unauthorized
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Missing API key should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_parse_endpoint_with_valid_auth() {
    let app = common::create_test_router();

    let request_body = serde_json::json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    // Request with valid API key
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "test-key")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be allowed (OK or client error depending on body, but NOT Unauthorized)
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Valid API key should accept request"
    );
}
