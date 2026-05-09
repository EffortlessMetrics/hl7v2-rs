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

async fn create_bundle(root: &TempRoot, bundle_id: &str) {
    let body = json!({
        "message": PHI_MESSAGE,
        "profile": PROFILE,
        "redaction_policy": POLICY,
        "bundle_id": bundle_id
    });
    let (status, _value, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/bundle",
        body,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body_text}");
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

    let (status, body, body_text) = post_json(
        test_router(Some(root.path().to_path_buf())),
        "/hl7/replay",
        replay_body("missing-case"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "BUNDLE_NOT_FOUND");
    assert_no_phi(&body_text);
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
