//! Integration tests for API authentication.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_auth_missing_api_key() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                // No X-API-Key header
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request without API key should be unauthorized"
    );
}

#[tokio::test]
async fn test_auth_invalid_api_key() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("X-API-Key", "invalid-key")
                .header("Content-Type", "application/json")
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
async fn test_auth_valid_api_key() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("X-API-Key", "test-key")
                .header("Content-Type", "application/json")
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

#[tokio::test]
async fn test_health_endpoint_public() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Health endpoint should be public"
    );
}
