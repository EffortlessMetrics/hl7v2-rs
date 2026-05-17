//! Integration tests for the /hl7/validate-redacted endpoint.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "server endpoint tests use static JSON fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_test_utils::{
    PHI_LEAK_SENTINEL_MESSAGE as PHI_MESSAGE, PHI_LEAK_SENTINEL_POLICY as REDACTION_POLICY,
    assert_no_phi_leak_sentinels, safe_error_phi_parity_fixture,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;

const VALIDATION_PROFILE: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
    required: true
    max_uses: 1
  - id: "PID"
    required: true
    max_uses: 1
constraints:
  - path: "PID.3"
    required: true
"#;

const PROFILE_REQUIRES_DROPPED_NAME: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
    required: true
    max_uses: 1
  - id: "PID"
    required: true
    max_uses: 1
constraints:
  - path: "PID.5"
    required: true
"#;

fn validate_redacted_request(request_body: Value) -> Request<Body> {
    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri("/hl7/validate-redacted")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap()
}

#[tokio::test]
async fn test_validate_redacted_returns_report_receipt_and_redacted_hl7_without_phi() {
    let app = common::create_test_router();
    let fixture = safe_error_phi_parity_fixture().unwrap();
    let request_body = json!({
        "message": &fixture.phi.message,
        "profile": VALIDATION_PROFILE,
        "redaction_policy": &fixture.phi.policy,
        "include_redacted_hl7": true
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(body_json["validation_report"]["valid"], true);
    assert_eq!(body_json["validation_report"]["message_type"], "ADT^A01");
    assert_eq!(body_json["redaction_receipt"]["phi_removed"], true);
    assert_eq!(body_json["redaction_receipt"]["hash_algorithm"], "sha256");
    assert!(body_json.get("redaction_receipt_v2").is_none());
    assert!(
        body_json["redacted_hl7"]
            .as_str()
            .unwrap()
            .contains("hash:sha256:")
    );
    fixture.assert_no_forbidden("REST validate-redacted response", &body_text);
}

#[tokio::test]
async fn test_validate_redacted_report_schema_v2_returns_nested_provenance_report_without_phi() {
    let app = common::create_test_router();
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": PROFILE_REQUIRES_DROPPED_NAME,
        "redaction_policy": REDACTION_POLICY,
        "include_redacted_hl7": true,
        "report_schema_version": 2
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let report_v2 = &body_json["validation_report_v2"];

    assert_eq!(body_json["validation_report"]["valid"], false);
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
    assert_eq!(report_v2["issues"][0]["path"], "PID.5");
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_validate_redacted_receipt_schema_v2_returns_nested_provenance_receipt_without_phi() {
    let app = common::create_test_router();
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": VALIDATION_PROFILE,
        "redaction_policy": REDACTION_POLICY,
        "include_redacted_hl7": true,
        "redaction_receipt_schema_version": 2
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let receipt_v2 = &body_json["redaction_receipt_v2"];

    assert_eq!(body_json["redaction_receipt"]["phi_removed"], true);
    assert_eq!(receipt_v2["schema_version"], "2");
    assert_eq!(receipt_v2["tool_name"], "hl7v2-server");
    assert_eq!(receipt_v2["tool_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(receipt_v2["phi_removed"], true);
    assert_eq!(receipt_v2["hash_algorithm"], "sha256");
    assert!(
        receipt_v2["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["path"] == "PID.3"
                && action["action"] == "hash"
                && action["status"] == "applied")
    );
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_validate_redacted_rejects_unsupported_receipt_schema_version() {
    let app = common::create_test_router();
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": VALIDATION_PROFILE,
        "redaction_policy": REDACTION_POLICY,
        "redaction_receipt_schema_version": 3
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body_json["code"], "VALIDATION_ERROR");
    assert!(
        body_json["message"]
            .as_str()
            .unwrap()
            .contains("unsupported redaction receipt schema version 3; expected 1 or 2")
    );
}

#[tokio::test]
async fn test_validate_redacted_omits_redacted_hl7_unless_requested() {
    let app = common::create_test_router();
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": VALIDATION_PROFILE,
        "redaction_policy": REDACTION_POLICY
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body_json["validation_report"]["valid"], true);
    assert!(body_json.get("redacted_hl7").is_none());
}

#[tokio::test]
async fn test_validate_redacted_report_is_generated_from_redacted_message() {
    let app = common::create_test_router();
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": PROFILE_REQUIRES_DROPPED_NAME,
        "redaction_policy": REDACTION_POLICY
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(body_json["validation_report"]["valid"], false);
    assert_eq!(body_json["validation_report"]["issues"][0]["path"], "PID.5");
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_validate_redacted_invalid_profile_does_not_echo_profile_text() {
    let app = common::create_test_router();
    let sensitive_profile =
        "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": sensitive_profile,
        "redaction_policy": REDACTION_POLICY
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

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
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_validate_redacted_fails_closed_when_policy_misses_sensitive_fields() {
    let app = common::create_test_router();
    let incomplete_policy = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "hash patient identifier"
"#;
    let request_body = json!({
        "message": PHI_MESSAGE,
        "profile": VALIDATION_PROFILE,
        "redaction_policy": incomplete_policy,
        "include_redacted_hl7": true
    });

    let response = app
        .oneshot(validate_redacted_request(request_body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = serde_json::from_slice(&body).unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(body_json["code"], "REDACTION_ERROR");
    assert!(body_json["message"].as_str().unwrap().contains("PID.5"));
    assert_no_phi(&body_text);
}

fn assert_no_phi(content: &str) {
    assert_no_phi_leak_sentinels("validate-redacted response", content);
}
