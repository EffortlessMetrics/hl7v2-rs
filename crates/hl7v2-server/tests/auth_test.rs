//! Integration tests for API authentication.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_missing_api_key_returns_401() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request without API key should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_parse_invalid_api_key_returns_401() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "invalid-key")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request with invalid API key should return 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_validate_missing_api_key_returns_401() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request without API key should return 401 Unauthorized"
    );
}
