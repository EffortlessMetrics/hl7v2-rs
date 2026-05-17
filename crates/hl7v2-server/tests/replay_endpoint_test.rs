//! Integration tests for the /hl7/replay endpoint.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "endpoint integration tests use static JSON fixtures for contract coverage"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_server::{AppState, CorsAllowedOrigins, build_router};
use hl7v2_test_utils::{
    PHI_LEAK_SENTINEL_MESSAGE as PHI_MESSAGE, PHI_LEAK_SENTINEL_POLICY as POLICY,
    assert_no_phi_leak_sentinels,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
constraints:
  - path: PID.3
    required: true
"#;

const TRIPLET_MESSAGE: &str = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||||19800101|X||Caucasian\r";

const TRIPLET_PROFILE: &str = r#"
message_structure: "GENERIC"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
valuesets:
  - path: "PID.8"
    name: "HL70001"
    codes:
      - "F"
      - "M"
      - "O"
      - "U"
      - "A"
      - "N"
"#;

const TRIPLET_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.8"
action = "retain"
reason = "administrative sex is required to reproduce validation"
"#;

const DIRTY_ADT_PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
  - id: PID
  - id: ZPV
constraints:
  - path: MSH.9
    required: true
  - path: PID.3
    required: true
"#;

const DIRTY_SAFE_ANALYSIS_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "ZPV.1"
action = "retain"
reason = "synthetic room marker is useful for dirty-corpus analysis"

[[rules]]
path = "ZPV.2"
action = "retain"
reason = "synthetic dirty-corpus note is useful for support triage"
"#;

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hl7v2-server-replay-{}-{nonce}-{name}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        match fs::remove_dir_all(&self.path) {
            Ok(()) | Err(_) => {}
        }
    }
}

fn test_router(bundle_output_root: Option<PathBuf>) -> axum::Router {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: None,
        cors_allowed_origins: CorsAllowedOrigins::default(),
        readiness_checks: hl7v2_server::ServerConfig::default().readiness_checks(),
        bundle_output_root,
        ack_policy: Default::default(),
        quarantine: Default::default(),
    });
    build_router(state)
}

fn json_request(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri(uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

async fn post_json(app: axum::Router, uri: &str, body: Value) -> (StatusCode, Value, String) {
    let response = app.oneshot(json_request(uri, body)).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let value = serde_json::from_str(&body_text).unwrap_or_else(|_| json!({}));
    (status, value, body_text)
}

fn dirty_real_world_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_data/dirty-real-world")
}

fn normalize_fixture_segments(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\n', "\r")
}

fn dirty_z_segment_message() -> String {
    let source = dirty_real_world_fixture_root()
        .join("after")
        .join("z-segment.hl7");
    let bytes = fs::read(&source).unwrap();
    normalize_fixture_segments(&bytes)
}

async fn create_bundle(root: &TempRoot, bundle_id: &str) {
    create_bundle_with_input(root, bundle_id, PHI_MESSAGE, PROFILE, POLICY).await;
}

async fn create_bundle_with_input(
    root: &TempRoot,
    bundle_id: &str,
    message: &str,
    profile: &str,
    policy: &str,
) -> Value {
    let body = json!({
        "message": message,
        "profile": profile,
        "redaction_policy": policy,
        "bundle_id": bundle_id
    });
    let (status, value, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/bundle",
        body,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body_text}");
    value
}

fn replay_body(bundle_id: &str) -> Value {
    json!({ "bundle_id": bundle_id })
}

#[tokio::test]
async fn test_replay_endpoint_replays_server_bundle_without_phi_or_root_paths() {
    let root = TempRoot::new("success");
    let bundle_id = "case-001";
    create_bundle(&root, bundle_id).await;

    let (status, report, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        replay_body(bundle_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["replay_version"], "1");
    assert_eq!(report["tool_name"], "hl7v2-server");
    assert_eq!(report["reproduced"], true);
    assert_eq!(report["message_type"], "ADT^A01");
    assert_eq!(report["validation_valid"], true);
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "manifest-hashes" && check["status"] == "pass")
    );
    assert_no_phi(&body_text);
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn test_rest_dirty_real_world_validate_redact_bundle_replay_workflow() {
    let root = TempRoot::new("dirty-z-workflow");
    let bundle_id = "dirty-z-rest-workflow";
    let message = dirty_z_segment_message();

    let (status, validate_redacted, validate_text) = post_json(
        test_router(None),
        "/hl7/validate-redacted",
        json!({
            "message": message.as_str(),
            "profile": DIRTY_ADT_PROFILE,
            "redaction_policy": DIRTY_SAFE_ANALYSIS_POLICY,
            "include_redacted_hl7": true,
            "report_schema_version": 2,
            "redaction_receipt_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{validate_text}");
    assert_eq!(validate_redacted["validation_report"]["valid"], true);
    assert_eq!(
        validate_redacted["validation_report"]["message_type"],
        "ADT^A01"
    );
    assert_eq!(validate_redacted["redaction_receipt"]["phi_removed"], true);
    assert_eq!(
        validate_redacted["validation_report_v2"]["schema_version"],
        "2"
    );
    assert_eq!(
        validate_redacted["redaction_receipt_v2"]["schema_version"],
        "2"
    );
    let redacted_hl7 = validate_redacted["redacted_hl7"].as_str().unwrap();
    assert!(redacted_hl7.contains("hash:sha256:"));
    assert!(redacted_hl7.contains("ZPV|legacy-room|dirty interface note"));
    assert!(!validate_text.contains("MRN-Z"));
    assert!(!validate_text.contains("Example^Zed"));
    assert!(!validate_text.contains("19700101"));

    let (status, bundle, bundle_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/bundle",
        json!({
            "message": message.as_str(),
            "profile": DIRTY_ADT_PROFILE,
            "redaction_policy": DIRTY_SAFE_ANALYSIS_POLICY,
            "bundle_id": bundle_id,
            "bundle_artifact_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{bundle_text}");
    assert_eq!(bundle["message_type"], "ADT^A01");
    assert_eq!(bundle["validation_valid"], true);
    assert_eq!(bundle["redaction_phi_removed"], true);
    assert!(!bundle_text.contains(root.path().to_string_lossy().as_ref()));
    assert!(!bundle_text.contains(bundle_id));
    assert!(!bundle_text.contains("MRN-Z"));
    assert!(!bundle_text.contains("Example^Zed"));
    assert!(!bundle_text.contains("19700101"));

    let bundle_dir = root.path().join(bundle_id);
    let redacted_message = fs::read_to_string(bundle_dir.join("message.redacted.hl7")).unwrap();
    assert!(redacted_message.contains("hash:sha256:"));
    assert!(redacted_message.contains("ZPV|legacy-room|dirty interface note"));
    assert!(!redacted_message.contains("MRN-Z"));
    assert!(!redacted_message.contains("Example^Zed"));
    assert!(!redacted_message.contains("19700101"));

    let (status, replay, replay_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        json!({
            "bundle_id": bundle_id,
            "replay_report_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{replay_text}");
    assert_eq!(replay["schema_version"], "2");
    assert_eq!(replay["reproduced"], true, "{replay_text}");
    assert_eq!(replay["message_type"], "ADT^A01");
    assert_eq!(replay["validation_valid"], true);
    assert!(!replay_text.contains(root.path().to_string_lossy().as_ref()));
    assert!(!replay_text.contains("MRN-Z"));
    assert!(!replay_text.contains("Example^Zed"));
    assert!(!replay_text.contains("19700101"));
}

#[tokio::test]
async fn test_replay_endpoint_schema_version_two_returns_v2_replay_report() {
    let root = TempRoot::new("v2");
    let bundle_id = "case-v2";
    create_bundle(&root, bundle_id).await;

    let (status, report, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        json!({
            "bundle_id": bundle_id,
            "replay_report_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["schema_version"], "2");
    assert_eq!(report["replay_version"], "1");
    assert_eq!(report["tool_name"], "hl7v2-server");
    assert_eq!(report["reproduced"], true);
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_replay_endpoint_reproduces_bundle_when_msh9_has_message_structure_component() {
    let root = TempRoot::new("msh9-triplet");
    let bundle_id = "case-triplet";
    let summary = create_bundle_with_input(
        &root,
        bundle_id,
        TRIPLET_MESSAGE,
        TRIPLET_PROFILE,
        TRIPLET_POLICY,
    )
    .await;
    assert_eq!(summary["message_type"], "ADT^A01");

    let (status, report, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        json!({
            "bundle_id": bundle_id,
            "replay_report_schema_version": 2
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["schema_version"], "2");
    assert_eq!(report["reproduced"], true, "{body_text}");
    assert_eq!(report["message_type"], "ADT^A01");
    assert_eq!(report["validation_valid"], false);
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "environment-match" && check["status"] == "pass"),
        "{body_text}"
    );
    assert_no_phi(&body_text);
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn test_replay_endpoint_reports_tampered_bundle_without_leaking_phi() {
    let root = TempRoot::new("tampered");
    let bundle_id = "case-tampered";
    create_bundle(&root, bundle_id).await;
    fs::write(
        root.path().join(bundle_id).join("validation-report.json"),
        "{}",
    )
    .unwrap();

    let (status, report, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        replay_body(bundle_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["reproduced"], false);
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "manifest-hashes" && check["status"] == "fail")
    );
    assert_no_phi(&body_text);
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn test_replay_endpoint_fails_closed_without_configured_output_root() {
    let (status, body, body_text) =
        post_json(test_router(None), "/hl7/replay", replay_body("case-001")).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "BUNDLE_OUTPUT_NOT_CONFIGURED");
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_replay_endpoint_rejects_unsafe_bundle_id() {
    let root = TempRoot::new("unsafe-id");

    let (status, body, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        replay_body("../escape"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BUNDLE_ERROR");
    assert_no_phi(&body_text);
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn test_replay_endpoint_returns_not_found_for_missing_bundle_id() {
    let root = TempRoot::new("missing");
    let sensitive_bundle_id = "MRN-SECRET-123";

    let (status, body, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        replay_body(sensitive_bundle_id),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "BUNDLE_NOT_FOUND");
    assert_no_phi(&body_text);
    assert!(!body_text.contains(sensitive_bundle_id));
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn test_replay_endpoint_rejects_unsupported_schema_version() {
    let root = TempRoot::new("bad-schema");
    let bundle_id = "case-schema";
    create_bundle(&root, bundle_id).await;

    let (status, body, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        json!({
            "bundle_id": bundle_id,
            "replay_report_schema_version": 3
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(
        body["message"]
            .as_str()
            .is_some_and(|message| message.contains("replay report schema version"))
    );
    assert_no_phi(&body_text);
}

fn assert_no_phi(content: &str) {
    assert_no_phi_leak_sentinels("replay response", content);
}
