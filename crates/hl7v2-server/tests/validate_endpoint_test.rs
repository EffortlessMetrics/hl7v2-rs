//! Integration tests for the /hl7/validate endpoint.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "legacy validate endpoint tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;

fn validate_request(request_body: Value) -> Request<Body> {
    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri("/hl7/validate")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap()
}

#[tokio::test]
async fn test_validate_with_minimal_profile() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": common::profiles::MINIMAL_PROFILE,
        "mllp_framed": false
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Validation with minimal profile should succeed"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["valid"], true);
}

#[tokio::test]
async fn test_validate_adt_a01_with_matching_profile() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "profile": common::profiles::ADT_A01_PROFILE,
        "mllp_framed": false
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

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

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["code"], "PARSE_ERROR");
}

#[tokio::test]
async fn test_validate_invalid_profile_yaml_returns_error() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": "invalid: yaml: structure:",
        "mllp_framed": false
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["code"], "PROFILE_LOAD_ERROR");
}

#[tokio::test]
async fn test_validate_profile_missing_required_fields_returns_profile_load_error() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": r#"
message_structure: "ADT_A01"
segments:
  - id: "MSH"
"#,
        "mllp_framed": false
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["code"], "PROFILE_LOAD_ERROR");
}

#[tokio::test]
async fn test_validate_invalid_message_against_profile_returns_valid_false() {
    let app = common::create_test_router();

    let profile = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
constraints:
  - path: "PID.3"
    required: true
"#;

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": profile,
        "mllp_framed": false
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["valid"], false);
    assert_eq!(body_json["errors"][0]["code"], "MISSING_REQUIRED_FIELD");
}

#[tokio::test]
async fn test_validate_missing_message_field_returns_400() {
    let app = common::create_test_router();

    let request_body = json!({
        "profile": common::profiles::MINIMAL_PROFILE
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

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

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

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
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/validate")
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
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/validate")
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

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

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
