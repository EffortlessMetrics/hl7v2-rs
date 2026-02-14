//! Security reproduction tests.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_endpoint_security() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "mllp_framed": false,
        "options": {
            "include_json": true,
            "validate_structure": false
        }
    });

    // Test 1: Request WITHOUT key -> Should fail (401)
    let response_no_key = app.clone()
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
        response_no_key.status(),
        StatusCode::UNAUTHORIZED,
        "Endpoint should be protected and return 401 without API Key"
    );

    // Test 2: Request with INVALID key -> Should fail (401)
    let response_bad_key = app.clone()
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
        response_bad_key.status(),
        StatusCode::UNAUTHORIZED,
        "Endpoint should return 401 with invalid API Key"
    );

    // Test 3: Request with VALID key -> Should pass (200)
    let response_valid_key = app
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

    assert_eq!(
        response_valid_key.status(),
        StatusCode::OK,
        "Endpoint should return 200 with valid API Key"
    );
}
