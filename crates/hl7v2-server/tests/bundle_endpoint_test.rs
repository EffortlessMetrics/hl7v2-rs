//! Integration tests for the /hl7/bundle endpoint.

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
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const PHI_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";

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

const POLICY: &str = r#"
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
path = "PID.11"
action = "drop"
reason = "patient address"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"

[[rules]]
path = "OBX.5"
action = "retain"
reason = "synthetic observation value shape is needed for analysis"
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
            "hl7v2-server-bundle-{}-{nonce}-{name}",
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
    });
    build_router(state)
}

fn bundle_request(bundle_id: &str) -> Request<Body> {
    let body = json!({
        "message": PHI_MESSAGE,
        "profile": PROFILE,
        "redaction_policy": POLICY,
        "bundle_id": bundle_id
    });

    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri("/hl7/bundle")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

async fn post_bundle(app: axum::Router, bundle_id: &str) -> (StatusCode, Value, String) {
    let response = app.oneshot(bundle_request(bundle_id)).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let value = serde_json::from_str(&body_text).unwrap_or_else(|_| json!({}));
    (status, value, body_text)
}

#[tokio::test]
async fn test_bundle_endpoint_writes_redacted_evidence_bundle() {
    let root = TempRoot::new("success");
    let bundle_id = "case-001";

    let (status, summary, body_text) =
        post_bundle(test_router(Some(root.path().to_path_buf())), bundle_id).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(summary["bundle_version"], "1");
    assert_eq!(summary["output_dir"], bundle_id);
    assert_eq!(summary["message_type"], "ADT^A01");
    assert_eq!(summary["validation_valid"], true);
    assert_eq!(summary["redaction_phi_removed"], true);
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));
    assert_no_phi(&body_text);

    let bundle_dir = root.path().join(bundle_id);
    for artifact in [
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "profile.yaml",
        "redaction-receipt.json",
        "environment.json",
        "replay.sh",
        "replay.ps1",
        "README.md",
        "manifest.json",
    ] {
        assert!(
            bundle_dir.join(artifact).exists(),
            "missing bundle artifact {artifact}"
        );
    }

    let redacted_message = fs::read_to_string(bundle_dir.join("message.redacted.hl7")).unwrap();
    assert!(redacted_message.contains("hash:sha256:"));
    assert_no_phi(&redacted_message);

    for artifact in [
        "validation-report.json",
        "field-paths.json",
        "redaction-receipt.json",
        "environment.json",
        "replay.sh",
        "replay.ps1",
        "README.md",
        "manifest.json",
    ] {
        let content = fs::read_to_string(bundle_dir.join(artifact)).unwrap();
        assert_no_phi(&content);
        assert!(
            !content.contains(root.path().to_string_lossy().as_ref()),
            "{artifact} leaked server bundle root"
        );
    }

    let validation_report: Value = serde_json::from_str(
        &fs::read_to_string(bundle_dir.join("validation-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(validation_report["profile"], "profile.yaml");

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(bundle_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["bundle_version"], "1");
    assert_eq!(manifest["tool_name"], "hl7v2-server");
    assert!(
        manifest["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["path"] == "message.redacted.hl7"
                && artifact["role"] == "redacted_message"
                && artifact["sha256"].as_str().is_some_and(is_sha256_hex))
    );

    let environment: Value =
        serde_json::from_str(&fs::read_to_string(bundle_dir.join("environment.json")).unwrap())
            .unwrap();
    assert_eq!(environment["tool_name"], "hl7v2-server");
    assert_eq!(
        environment["replay_command"],
        "hl7v2 replay . --format json"
    );
}

#[tokio::test]
async fn test_bundle_endpoint_fails_closed_without_configured_output_root() {
    let (status, body, body_text) = post_bundle(test_router(None), "case-001").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "BUNDLE_OUTPUT_NOT_CONFIGURED");
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_bundle_endpoint_fails_closed_when_output_root_is_not_ready() {
    let root = TempRoot::new("missing-root-parent");
    let missing_root = root.path().join("missing");
    let (status, body, body_text) = post_bundle(test_router(Some(missing_root)), "case-001").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "BUNDLE_OUTPUT_NOT_READY");
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_bundle_endpoint_rejects_unsafe_bundle_id_without_writing() {
    let root = TempRoot::new("unsafe-id");
    let (status, body, body_text) =
        post_bundle(test_router(Some(root.path().to_path_buf())), "../escape").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BUNDLE_ERROR");
    assert_no_phi(&body_text);
    assert!(fs::read_dir(root.path()).unwrap().next().is_none());
}

#[tokio::test]
async fn test_bundle_endpoint_rejects_existing_bundle_id() {
    let root = TempRoot::new("existing");
    fs::create_dir(root.path().join("case-001")).unwrap();

    let (status, body, body_text) =
        post_bundle(test_router(Some(root.path().to_path_buf())), "case-001").await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "BUNDLE_EXISTS");
    assert_no_phi(&body_text);
}

fn assert_no_phi(content: &str) {
    for sentinel in ["Doe^John", "123456^^^HOSP^MR", "19700101", "123 Main St"] {
        assert!(!content.contains(sentinel), "content leaked {sentinel}");
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
