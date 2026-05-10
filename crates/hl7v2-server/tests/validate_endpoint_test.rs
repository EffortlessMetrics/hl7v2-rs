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
    assert_eq!(body_json["message_type"], "ADT^A01");
    assert_eq!(body_json["profile"], "MINIMAL");
    assert_eq!(body_json["segment_count"], 1);
    assert_eq!(body_json["issue_count"], 0);
    assert_eq!(body_json["issues"].as_array().unwrap().len(), 0);
    assert!(body_json.get("validation_report_v2").is_none());
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
    let sensitive_profile =
        "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": sensitive_profile,
        "mllp_framed": false
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let body_json: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(body_json["code"], "PROFILE_LOAD_ERROR");
    assert_eq!(
        body_json["message"],
        "profile could not be loaded; run profile lint for details"
    );
    assert!(!body_text.contains("Jane Secret"));
    assert!(!body_text.contains("MRN-SECRET-123"));
    assert!(!body_text.contains(sensitive_profile));
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
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let body_json: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(body_json["code"], "PROFILE_LOAD_ERROR");
    assert_eq!(
        body_json["message"],
        "profile could not be loaded; run profile lint for details"
    );
    assert!(!body_text.contains("missing field"));
    assert!(!body_text.contains("segments"));
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
    assert_eq!(body_json["message_type"], "ADT^A01");
    assert_eq!(body_json["profile"], "ADT_A01");
    assert_eq!(body_json["segment_count"], 1);
    assert_eq!(body_json["issue_count"], 1);
    assert_eq!(body_json["issues"][0]["code"], "missing_required_field");
    assert_eq!(body_json["issues"][0]["severity"], "error");
    assert_eq!(body_json["issues"][0]["path"], "PID.3");
    assert_eq!(body_json["issues"][0]["rule_id"], "missing_required_field");
    assert_eq!(
        body_json["issues"][0]["message"],
        "Required field PID.3 is missing"
    );
    assert_eq!(body_json["issues"][0]["field_index"], 3);
    assert_eq!(body_json["errors"][0]["code"], "MISSING_REQUIRED_FIELD");
}

#[tokio::test]
async fn test_validate_report_schema_v2_returns_nested_provenance_report() {
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
        "mllp_framed": false,
        "report_schema_version": 2
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    let report_v2 = &body_json["validation_report_v2"];

    assert_eq!(body_json["valid"], false);
    assert_eq!(body_json["issue_count"], 1);
    assert_eq!(report_v2["schema_version"], "2");
    assert_eq!(report_v2["tool_name"], "hl7v2-server");
    assert_eq!(report_v2["valid"], false);
    assert_eq!(report_v2["message_type"], "ADT^A01");
    assert_eq!(report_v2["profile"], "ADT_A01");
    assert_eq!(report_v2["profile_identity"]["label"], "ADT_A01");
    assert_eq!(
        report_v2["profile_identity"]["message_structure"],
        "ADT_A01"
    );
    assert_eq!(report_v2["profile_identity"]["version"], "2.5");
    assert_eq!(
        report_v2["profile_identity"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(report_v2["issues"][0]["code"], "missing_required_field");
    assert_eq!(report_v2["issues"][0]["path"], "PID.3");
}

#[tokio::test]
async fn test_validate_report_schema_version_rejects_unknown_version() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "profile": common::profiles::MINIMAL_PROFILE,
        "mllp_framed": false,
        "report_schema_version": 3
    });

    let response = app.oneshot(validate_request(request_body)).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["code"], "VALIDATION_ERROR");
    assert!(body_json["message"].as_str().unwrap().contains("1 or 2"));
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
