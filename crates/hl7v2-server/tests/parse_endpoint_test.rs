//! Integration tests for the /hl7/parse endpoint.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_valid_adt_a01_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "mllp_framed": false,
        "options": {
            "include_json": true,
            "validate_structure": false
        }
    });

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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Valid ADT^A01 message should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_valid_adt_a04_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A04_VALID,
        "mllp_framed": false
    });

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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Valid ADT^A04 message should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_valid_oru_r01_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ORU_R01_VALID,
        "mllp_framed": false
    });

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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Valid ORU^R01 message should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_minimal_valid_message() {
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
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Minimal valid message (MSH only) should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_malformed_message_returns_error() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::INVALID_MALFORMED,
        "mllp_framed": false
    });

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

    assert_ne!(
        response.status(),
        StatusCode::OK,
        "Malformed message should return error"
    );
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Should return 4xx or 5xx status code"
    );
}

#[tokio::test]
async fn test_parse_invalid_encoding_may_succeed_if_has_msh() {
    let app = common::create_test_router();

    // Note: "MSH|Wrong encoding characters" may actually parse successfully
    // since it has MSH and field separator. The encoding characters are
    // in MSH.2, so this is technically a valid (though non-standard) message.
    let request_body = json!({
        "message": common::fixtures::INVALID_ENCODING,
        "mllp_framed": false
    });

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

    // This test just verifies the endpoint doesn't crash
    // The actual parsing behavior depends on the parser's strictness
    assert!(
        response.status().is_success() || response.status().is_client_error(),
        "Should handle message gracefully, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_parse_empty_request_body_returns_400() {
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

    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Empty request body should return 400 or 422, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_parse_invalid_json_returns_400() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("not valid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Invalid JSON should return 400 Bad Request"
    );
}

#[tokio::test]
async fn test_parse_response_contains_segments() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "mllp_framed": false,
        "options": {
            "include_json": true
        }
    });

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

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Response should contain segment information
    assert!(
        body_str.contains("MSH") || body_str.contains("segments") || body_str.contains("metadata"),
        "Parse response should contain segment information"
    );
}

#[tokio::test]
async fn test_parse_get_method_not_allowed() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "GET method should not be allowed on /hl7/parse"
    );
}
