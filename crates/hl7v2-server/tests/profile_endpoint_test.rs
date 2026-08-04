//! Integration tests for inline profile evidence REST endpoints.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "endpoint integration tests use static JSON fixtures for contract coverage"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;

const PROFILE_TEST_PROFILE: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;
const PROFILE_TEST_VALID_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";
const PROFILE_TEST_INVALID_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL124|P|2.5\rPID|1||||Doe^John\r";

fn post_json(path: &str, request_body: Value) -> Request<Body> {
    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri(path)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap()
}

async fn post_profile(path: &str, request_body: Value) -> (StatusCode, Value, String) {
    post_profile_with_max(path, request_body, 50 * 1024 * 1024).await
}

async fn post_profile_with_max(
    path: &str,
    request_body: Value,
    max_message_size: usize,
) -> (StatusCode, Value, String) {
    let app = common::create_test_router_with_message_size_limit(max_message_size);
    let response = app.oneshot(post_json(path, request_body)).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let value = serde_json::from_str(&body_text).unwrap_or_else(|_| json!({}));
    (status, value, body_text)
}

#[tokio::test]
async fn test_profile_lint_returns_default_report_without_echoing_profile_yaml() {
    let request_body = json!({
        "profile": common::profiles::ADT_A01_PROFILE
    });

    let (status, body, body_text) = post_profile("/hl7/profile/lint", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["valid"], true);
    assert_eq!(body["error_count"], 0);
    assert!(body.get("schema_version").is_none());
    assert!(!body_text.contains("ADT^A01 test profile"));
    assert!(!body_text.contains("DOC456"));
}

#[tokio::test]
async fn test_profile_lint_schema_version_2_adds_server_provenance() {
    let request_body = json!({
        "profile": common::profiles::ADT_A01_PROFILE,
        "report_schema_version": 2
    });

    let (status, body, body_text) = post_profile("/hl7/profile/lint", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["schema_version"], "2");
    assert_eq!(body["tool_name"], "hl7v2-server");
    assert_eq!(body["valid"], true);
    assert_eq!(body["error_count"], 0);
    assert!(!body_text.contains("ADT^A01 test profile"));
}

#[tokio::test]
async fn test_profile_lint_reports_yaml_errors_as_evidence() {
    let sensitive_profile =
        "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
    let request_body = json!({
        "profile": sensitive_profile
    });

    let (status, body, body_text) = post_profile("/hl7/profile/lint", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["valid"], false);
    assert_eq!(body["error_count"], 1);
    assert_eq!(body["issues"][0]["code"], "yaml_parse_error");
    assert!(!body_text.contains("Jane Secret"));
    assert!(!body_text.contains("MRN-SECRET-123"));
    assert!(!body_text.contains(sensitive_profile));
}

#[tokio::test]
async fn test_profile_explain_schema_version_2_explains_inline_profile() {
    let request_body = json!({
        "profile": common::profiles::ADT_A01_PROFILE,
        "profile_name": "adt-a01-inline",
        "report_schema_version": 2
    });

    let (status, body, body_text) = post_profile("/hl7/profile/explain", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["schema_version"], "2");
    assert_eq!(body["tool_name"], "hl7v2-server");
    assert_eq!(body["profile"], "adt-a01-inline");
    assert_eq!(body["message_structure"], "ADT_A01");
    assert_eq!(body["version"], "2.5.1");
    assert_eq!(body["summary"]["segment_count"], 4);
    assert_eq!(body["segments"][0]["id"], "MSH");
    assert_eq!(body["segments"][0]["required"], true);
    assert_eq!(body["segments"][0]["repetition"], false);
    assert_eq!(body["lint"]["valid"], true);
    assert_eq!(body["profile_sha256"].as_str().unwrap().len(), 64);
    assert!(!body_text.contains("ADT^A01 test profile"));
    assert!(!body_text.contains("DOC456"));
}

#[tokio::test]
async fn test_profile_explain_rejects_invalid_profile_yaml() {
    let sensitive_profile =
        "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
    let request_body = json!({
        "profile": sensitive_profile,
        "report_schema_version": 2
    });

    let (status, body, body_text) = post_profile("/hl7/profile/explain", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PROFILE_LOAD_ERROR");
    assert_eq!(
        body["message"],
        "profile could not be loaded; run profile lint for details"
    );
    assert!(!body_text.contains("Jane Secret"));
    assert!(!body_text.contains("MRN-SECRET-123"));
    assert!(!body_text.contains(sensitive_profile));
}

#[tokio::test]
async fn test_profile_explain_rejects_path_like_profile_name() {
    let request_body = json!({
        "profile": common::profiles::ADT_A01_PROFILE,
        "profile_name": "../adt-a01.yaml"
    });

    let (status, body, body_text) = post_profile("/hl7/profile/explain", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["message"].as_str().unwrap().contains("profile_name"));
    assert!(!body_text.contains("../adt-a01.yaml"));
}

#[tokio::test]
async fn test_profile_test_schema_version_2_runs_inline_fixtures_without_payload_echo() {
    let request_body = json!({
        "profile": PROFILE_TEST_PROFILE,
        "fixtures": [
            {
                "name": "valid/adt.hl7",
                "message": PROFILE_TEST_VALID_MESSAGE,
                "expectation": "valid"
            },
            {
                "name": "invalid/missing-pid3.hl7",
                "message": PROFILE_TEST_INVALID_MESSAGE,
                "expectation": "invalid",
                "expected_report_json": "{\"valid\":false,\"issue_count\":1,\"issues\":[{\"code\":\"missing_required_field\",\"path\":\"PID.3\"}]}"
            }
        ],
        "report_schema_version": 2
    });

    let (status, body, body_text) = post_profile("/hl7/profile/test", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["schema_version"], "2");
    assert_eq!(body["tool_name"], "hl7v2-server");
    assert_eq!(body["profile"], "<inline-profile>");
    assert_eq!(body["fixtures"], "<inline-fixtures>");
    assert_eq!(body["case_count"], 2);
    assert_eq!(body["failed_count"], 0);
    assert_eq!(body["cases"][0]["name"], "valid/adt.hl7");
    assert_eq!(body["cases"][0]["path"], "valid/adt.hl7");
    assert_eq!(body["cases"][1]["expected_report"]["matched"], true);
    assert!(!body_text.contains("Doe^John"));
}

#[tokio::test]
async fn test_profile_test_enforces_decoded_message_size_for_mllp_fixture() {
    let framed = hl7v2::wrap_mllp(PROFILE_TEST_VALID_MESSAGE.as_bytes());
    let framed = String::from_utf8(framed).unwrap();
    let request_body = json!({
        "profile": PROFILE_TEST_PROFILE,
        "fixtures": [{
            "message": framed,
            "expectation": "valid",
            "mllp_framed": true
        }]
    });

    let (status, body, _) = post_profile_with_max(
        "/hl7/profile/test",
        request_body,
        PROFILE_TEST_VALID_MESSAGE.len().saturating_sub(1),
    )
    .await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["code"], "MESSAGE_TOO_LARGE");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("exceeds maximum")
    );
}

#[tokio::test]
async fn test_profile_test_rejects_invalid_inline_profile_without_echoing_profile() {
    let sensitive_profile =
        "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
    let request_body = json!({
        "profile": sensitive_profile,
        "fixtures": [
            {
                "message": PROFILE_TEST_VALID_MESSAGE,
                "expectation": "valid"
            }
        ],
        "report_schema_version": 2
    });

    let (status, body, body_text) = post_profile("/hl7/profile/test", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PROFILE_LOAD_ERROR");
    assert_eq!(
        body["message"],
        "profile could not be loaded; run profile lint for details"
    );
    assert!(!body_text.contains("Jane Secret"));
    assert!(!body_text.contains("MRN-SECRET-123"));
    assert!(!body_text.contains(sensitive_profile));
}

#[tokio::test]
async fn test_profile_test_rejects_empty_fixtures() {
    let request_body = json!({
        "profile": PROFILE_TEST_PROFILE,
        "fixtures": []
    });

    let (status, body, _) = post_profile("/hl7/profile/test", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert_eq!(
        body["message"],
        "fixtures must contain at least one fixture"
    );
}

#[tokio::test]
async fn test_profile_test_rejects_path_traversal_fixture_name_without_echoing_payload() {
    let request_body = json!({
        "profile": PROFILE_TEST_PROFILE,
        "fixtures": [
            {
                "name": "../adt-a01.hl7",
                "message": PROFILE_TEST_VALID_MESSAGE,
                "expectation": "valid"
            }
        ]
    });

    let (status, body, body_text) = post_profile("/hl7/profile/test", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(!body_text.contains("../adt-a01.hl7"));
    assert!(!body_text.contains("Doe^John"));
}

#[tokio::test]
async fn test_profile_endpoints_reject_unknown_schema_versions() {
    for path in [
        "/hl7/profile/lint",
        "/hl7/profile/explain",
        "/hl7/profile/test",
    ] {
        let mut request_body = json!({
            "profile": PROFILE_TEST_PROFILE,
            "report_schema_version": 9
        });
        if path.ends_with("/test") {
            request_body["fixtures"] = json!([
                {
                    "message": PROFILE_TEST_VALID_MESSAGE,
                    "expectation": "valid"
                }
            ]);
        }

        let (status, body, _) = post_profile(path, request_body).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "VALIDATION_ERROR");
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("unsupported profile")
        );
    }
}
