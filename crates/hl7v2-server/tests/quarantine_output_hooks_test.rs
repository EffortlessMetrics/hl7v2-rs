//! Integration tests for configured quarantine output hooks.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "endpoint integration tests use static JSON fixtures for contract coverage"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_server::{AppState, CorsAllowedOrigins, QuarantineConfig, build_router};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const PHI_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M|||123 Main St||5558675309\rNK1|1|Watcher^Nora||900 Support Way|5550001234\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";

const VALID_PROFILE: &str = r#"
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

const FAILING_PROFILE: &str = r#"
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

const REDACTION_POLICY: &str = r#"
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
path = "PID.13"
action = "drop"
reason = "patient phone"

[[rules]]
path = "NK1.2"
action = "drop"
reason = "next-of-kin name"

[[rules]]
path = "NK1.4"
action = "drop"
reason = "next-of-kin address"

[[rules]]
path = "NK1.5"
action = "drop"
reason = "next-of-kin phone"

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
            "hl7v2-server-quarantine-{}-{nonce}-{name}",
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

fn test_router(quarantine: QuarantineConfig) -> axum::Router {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: None,
        cors_allowed_origins: CorsAllowedOrigins::default(),
        readiness_checks: hl7v2_server::ServerConfig::default().readiness_checks(),
        bundle_output_root: None,
        ack_policy: Default::default(),
        quarantine,
    });
    build_router(state)
}

fn validate_redacted_request(profile: &str) -> Request<Body> {
    let body = json!({
        "message": PHI_MESSAGE,
        "profile": profile,
        "redaction_policy": REDACTION_POLICY,
        "include_redacted_hl7": false
    });

    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri("/hl7/validate-redacted")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

async fn post_validate_redacted(app: axum::Router, profile: &str) -> (StatusCode, Value, String) {
    let response = app
        .oneshot(validate_redacted_request(profile))
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let value = serde_json::from_str(&body_text).unwrap_or_else(|_| json!({}));
    (status, value, body_text)
}

#[tokio::test]
async fn test_quarantine_hook_writes_bundle_for_failed_redacted_validation() {
    let root = TempRoot::new("bundle");
    let quarantine = QuarantineConfig {
        enabled: true,
        path: Some(root.path().to_path_buf()),
        ..Default::default()
    };

    let (status, body, body_text) =
        post_validate_redacted(test_router(quarantine), FAILING_PROFILE).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["validation_report"]["valid"], false);
    assert_eq!(body["quarantine"]["quarantine_version"], "1");
    assert_eq!(body["quarantine"]["reason"], "validation_error");
    assert_eq!(body["quarantine"]["validation_issue_count"], 1);
    assert_no_phi(&body_text);
    assert!(!body_text.contains(root.path().to_string_lossy().as_ref()));

    let output_dir = body["quarantine"]["output_dir"].as_str().unwrap();
    assert!(output_dir.starts_with("quarantine-"));
    let quarantine_dir = root.path().join(output_dir);
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
            quarantine_dir.join(artifact).exists(),
            "missing quarantine artifact {artifact}"
        );
        let content = fs::read_to_string(quarantine_dir.join(artifact)).unwrap();
        assert_no_phi(&content);
        assert!(!content.contains(root.path().to_string_lossy().as_ref()));
    }
}

#[tokio::test]
async fn test_quarantine_hook_does_not_write_for_valid_redacted_validation() {
    let root = TempRoot::new("valid");
    let quarantine = QuarantineConfig {
        enabled: true,
        path: Some(root.path().to_path_buf()),
        ..Default::default()
    };

    let (status, body, _body_text) =
        post_validate_redacted(test_router(quarantine), VALID_PROFILE).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["validation_report"]["valid"], true);
    assert!(body.get("quarantine").is_none());
    assert!(fs::read_dir(root.path()).unwrap().next().is_none());
}

#[tokio::test]
async fn test_quarantine_hook_writes_selected_artifacts_without_full_bundle() {
    let root = TempRoot::new("partial");
    let quarantine = QuarantineConfig {
        enabled: true,
        path: Some(root.path().to_path_buf()),
        write_bundle: false,
        write_report: true,
        write_redacted: true,
    };

    let (status, body, body_text) =
        post_validate_redacted(test_router(quarantine), FAILING_PROFILE).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["quarantine"]["artifacts"].as_array().unwrap().len(), 3);
    assert_no_phi(&body_text);

    let output_dir = body["quarantine"]["output_dir"].as_str().unwrap();
    let quarantine_dir = root.path().join(output_dir);
    assert!(quarantine_dir.join("validation-report.json").exists());
    assert!(quarantine_dir.join("message.redacted.hl7").exists());
    assert!(quarantine_dir.join("redaction-receipt.json").exists());
    assert!(!quarantine_dir.join("manifest.json").exists());
}

#[tokio::test]
async fn test_quarantine_hook_fails_closed_without_configured_path() {
    let quarantine = QuarantineConfig {
        enabled: true,
        path: None,
        ..Default::default()
    };

    let (status, body, body_text) =
        post_validate_redacted(test_router(quarantine), FAILING_PROFILE).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "QUARANTINE_OUTPUT_NOT_CONFIGURED");
    assert_no_phi(&body_text);
}

fn assert_no_phi(content: &str) {
    for sentinel in [
        "Doe^John",
        "123456^^^HOSP^MR",
        "19700101",
        "123 Main St",
        "5558675309",
        "Watcher^Nora",
        "900 Support Way",
        "5550001234",
    ] {
        assert!(!content.contains(sentinel), "content leaked {sentinel}");
    }
}
