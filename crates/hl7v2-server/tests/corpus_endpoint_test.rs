//! Integration tests for inline corpus evidence endpoints.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "endpoint integration tests use static JSON fixtures for contract coverage"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tower::ServiceExt;

mod common;

fn dirty_real_world_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_data/dirty-real-world")
}

fn normalize_fixture_segments(bytes: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\n', "\r")
        .into_bytes()
}

fn dirty_corpus_messages(category: &str) -> Vec<Value> {
    let source = dirty_real_world_fixture_root().join(category);
    let mut paths = std::fs::read_dir(&source)
        .expect("dirty fixture category should be readable")
        .map(|entry| {
            entry
                .expect("dirty fixture entry should be readable")
                .path()
        })
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path).expect("dirty fixture file should be readable");
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("dirty fixture file should have a UTF-8 name");
            let message = String::from_utf8(normalize_fixture_segments(&bytes))
                .expect("dirty fixture should be UTF-8 after normalization");
            json!({ "id": file_name, "message": message })
        })
        .collect()
}

fn dirty_after_corpus_messages() -> Vec<Value> {
    let mut messages = dirty_corpus_messages("after");
    let source = dirty_real_world_fixture_root().join("sources/mllp-source.hl7");
    let bytes = std::fs::read(&source).expect("MLLP source fixture should be readable");
    let normalized = normalize_fixture_segments(&bytes);
    let wrapped = hl7v2::wrap_mllp(&normalized);
    let framed = String::from_utf8(wrapped.clone()).expect("MLLP fixture should be UTF-8");
    messages.push(json!({ "id": "mllp-framed.hl7", "message": framed }));
    let mut truncated = wrapped;
    let _ = truncated.pop();
    let truncated = String::from_utf8(truncated).expect("truncated MLLP fixture should be UTF-8");
    messages.push(json!({ "id": "mllp-truncated.hl7", "message": truncated }));
    messages
}

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
async fn test_corpus_endpoints_share_dirty_real_world_fixture_categories() {
    let before = dirty_corpus_messages("before");
    let after = dirty_after_corpus_messages();

    let (status, summary, summary_text) = post_corpus(
        "/hl7/corpus/summarize",
        json!({
            "messages": after,
            "summary_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["schema_version"], "2");
    assert_eq!(summary["tool_name"], "hl7v2-server");
    assert_eq!(summary["root"], "<inline-corpus>");
    assert_eq!(summary["file_count"], 7);
    assert_eq!(summary["message_count"], 4);
    assert_eq!(summary["parse_error_count"], 3);
    assert!(
        summary["message_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["value"] == "ADT^A08" && entry["count"] == 1)
    );
    assert!(
        summary["message_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["value"] == "ADT^A04" && entry["count"] == 1)
    );
    assert!(
        summary["segments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["value"] == "ZPV" && entry["count"] == 1)
    );
    assert!(
        summary["parse_errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "malformed-delimiters.hl7")
    );
    assert!(
        summary["parse_errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "partial-batch.hl7")
    );
    assert!(
        summary["parse_errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "mllp-truncated.hl7")
    );
    assert!(!summary_text.contains("MRN-DIRTY"));

    let (status, fingerprint, fingerprint_text) = post_corpus(
        "/hl7/corpus/fingerprint",
        json!({
            "messages": dirty_after_corpus_messages(),
            "fingerprint_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(fingerprint["schema_version"], "2");
    assert_eq!(fingerprint["tool_name"], "hl7v2-server");
    assert_eq!(fingerprint["file_count"], 7);
    assert_eq!(fingerprint["message_count"], 4);
    assert_eq!(fingerprint["parse_error_count"], 3);
    assert!(
        fingerprint["field_cardinality"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "OBX.5"
                && entry["max_per_message"] == 20
                && entry["total_occurrences"] == 20)
    );
    assert!(
        fingerprint["field_cardinality"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "ZPV.1" && entry["total_occurrences"] == 1)
    );
    assert!(!fingerprint_text.contains("MRN-DIRTY"));

    let (status, diff, diff_text) = post_corpus(
        "/hl7/corpus/diff",
        json!({
            "before": before,
            "after": dirty_after_corpus_messages(),
            "diff_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(diff["schema_version"], "2");
    assert_eq!(diff["tool_name"], "hl7v2-server");
    assert_eq!(diff["file_count"]["delta"], 5);
    assert_eq!(diff["message_count"]["delta"], 2);
    assert_eq!(diff["parse_error_count"]["delta"], 3);
    assert!(
        diff["field_cardinality"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "OBX.5"
                && entry["max_per_message_delta"] == 15
                && entry["total_occurrences_delta"] == 15)
    );
    assert!(!diff_text.contains("MRN-DIRTY"));
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
