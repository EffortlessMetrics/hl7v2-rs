//! Integration tests for inline corpus evidence endpoints.

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

async fn post_corpus(path: &str, request_body: Value) -> (StatusCode, Value, String) {
    let app = common::create_test_router();
    let response = app.oneshot(post_json(path, request_body)).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let value = serde_json::from_str(&body_text).unwrap_or_else(|_| json!({}));
    (status, value, body_text)
}

#[tokio::test]
async fn test_corpus_summarize_accepts_inline_messages_without_echoing_payloads() {
    let request_body = json!({
        "messages": [
            { "id": "adt-1", "message": common::fixtures::ADT_A01_VALID },
            { "id": "bad-1", "message": common::fixtures::INVALID_MALFORMED }
        ]
    });

    let (status, body, body_text) = post_corpus("/hl7/corpus/summarize", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["root"], "<inline-corpus>");
    assert_eq!(body["file_count"], 2);
    assert_eq!(body["message_count"], 1);
    assert_eq!(body["parse_error_count"], 1);
    assert_eq!(body["parse_errors"][0]["path"], "bad-1");
    assert!(
        body["message_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|count| count["value"] == "ADT^A01" && count["count"] == 1)
    );
    assert!(!body_text.contains("Doe"));
    assert!(!body_text.contains("MRN123"));
    assert!(!body_text.contains(common::fixtures::INVALID_MALFORMED));
}

#[tokio::test]
async fn test_corpus_fingerprint_schema_v2_includes_profile_issue_counts() {
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
        "messages": [
            { "id": "minimal-1", "message": common::fixtures::MINIMAL_VALID }
        ],
        "profile": profile,
        "fingerprint_schema_version": 2
    });

    let (status, body, body_text) = post_corpus("/hl7/corpus/fingerprint", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["schema_version"], "2");
    assert_eq!(body["tool_name"], "hl7v2-server");
    assert_eq!(body["root"], "<inline-corpus>");
    assert_eq!(body["profile"]["path"], "<inline-profile>");
    assert_eq!(body["profile"]["message_structure"], "ADT_A01");
    assert!(
        body["validation_issue_code_counts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|count| count["value"] == "missing_required_field" && count["count"] == 1)
    );
    assert!(!body_text.contains("minimal-1"));
}

#[tokio::test]
async fn test_corpus_diff_reports_inline_before_after_deltas() {
    let request_body = json!({
        "before": [
            { "id": "before-adt", "message": common::fixtures::ADT_A01_VALID }
        ],
        "after": [
            { "id": "after-adt", "message": common::fixtures::ADT_A01_VALID },
            { "id": "after-oru", "message": common::fixtures::ORU_R01_VALID }
        ],
        "diff_schema_version": 2
    });

    let (status, body, body_text) = post_corpus("/hl7/corpus/diff", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["schema_version"], "2");
    assert_eq!(body["tool_name"], "hl7v2-server");
    assert_eq!(body["before_root"], "<inline-before>");
    assert_eq!(body["after_root"], "<inline-after>");
    assert_eq!(body["message_count"]["delta"], 1);
    assert_eq!(body["new_message_types"], json!(["ORU^R01"]));
    assert!(
        body["field_presence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field["path"] == "OBX.5" && field["message_count_delta"] == 1)
    );
    assert!(!body_text.contains("Patient^Test"));
    assert!(!body_text.contains("MRN789"));
}

#[tokio::test]
async fn test_corpus_endpoints_reject_empty_message_sets() {
    let request_body = json!({
        "messages": []
    });

    let (status, body, _) = post_corpus("/hl7/corpus/summarize", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("must contain at least one message")
    );
}

#[tokio::test]
async fn test_corpus_endpoints_reject_path_like_message_ids() {
    let request_body = json!({
        "messages": [
            { "id": "../secret", "message": common::fixtures::ADT_A01_VALID }
        ]
    });

    let (status, body, body_text) = post_corpus("/hl7/corpus/summarize", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("corpus message id")
    );
    assert!(!body_text.contains("Doe"));
    assert!(!body_text.contains("MRN123"));
}

#[tokio::test]
async fn test_corpus_endpoints_reject_unknown_schema_versions() {
    let request_body = json!({
        "messages": [
            { "id": "adt-1", "message": common::fixtures::ADT_A01_VALID }
        ],
        "summary_schema_version": 9
    });

    let (status, body, _) = post_corpus("/hl7/corpus/summarize", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("unsupported corpus summary schema version")
    );
}
