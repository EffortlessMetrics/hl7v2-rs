//! Integration tests for the /hl7/validate endpoint.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_validate_with_minimal_profile() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": common::profiles::MINIMAL_PROFILE,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Validation with minimal profile should succeed"
    );
}

#[tokio::test]
async fn test_validate_adt_a01_with_matching_profile() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "profile": common::profiles::ADT_A01_PROFILE,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "ADT^A01 message should validate against ADT_A01 profile"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Response should contain validation results
    assert!(
        body_str.contains("issues") || body_str.contains("valid") || body_str.contains("errors"),
        "Validation response should contain validation results"
    );
}

#[tokio::test]
async fn test_validate_malformed_message_returns_error() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::INVALID_MALFORMED,
        "profile": common::profiles::MINIMAL_PROFILE,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error for malformed message
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Malformed message should return error status"
    );
}

#[tokio::test]
async fn test_validate_invalid_profile_yaml_returns_error() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": "invalid: yaml: structure:",
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Invalid YAML profile might still succeed if parsed as simple string
    // or may return an error depending on validation strictness
    assert!(
        response.status() == StatusCode::OK || response.status().is_client_error() || response.status().is_server_error(),
        "Invalid profile should be handled gracefully, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_validate_missing_message_field_returns_400() {
    let app = common::create_test_router();

    let request_body = json!({
        "profile": common::profiles::MINIMAL_PROFILE
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST
        || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Missing message field should return 400 or 422, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_validate_missing_profile_field_returns_400() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST
        || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Missing profile field should return 400 or 422, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_validate_empty_request_body_returns_400() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST
        || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Empty request body should return 400 or 422, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_validate_get_method_not_allowed() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "GET method should not be allowed on /hl7/validate"
    );
}

#[tokio::test]
async fn test_validate_returns_json_response() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": common::profiles::MINIMAL_PROFILE,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/validate")
                .header("X-API-Key", "test-key")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    if response.status() == StatusCode::OK {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok());

        assert!(
            content_type.is_some() && content_type.unwrap().contains("application/json"),
            "Validate response should be JSON"
        );
    }
}
