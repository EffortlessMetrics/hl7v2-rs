//! Integration tests for the /hl7/ack-policy endpoint.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "endpoint integration tests use static JSON fixtures for contract coverage"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_server::{
    AckPolicyConfig, AckPolicyMode, AppState, CorsAllowedOrigins, ServerConfig, build_router,
};
use hl7v2_test_utils::{PHI_LEAK_SENTINEL_MESSAGE as SAMPLE_MSG, assert_no_phi_leak_sentinels};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

const VALID_PROFILE: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
    required: true
constraints:
  - path: "PID.3"
    required: true
"#;
const INVALID_PROFILE: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
    required: true
constraints:
  - path: "PID.99"
    required: true
"#;
const PARTIAL_PARSE_MESSAGE: &str =
    "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPI";

fn test_router(policy: AckPolicyConfig, api_key: Option<&str>) -> axum::Router {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: api_key.map(str::to_string),
        cors_allowed_origins: CorsAllowedOrigins::default(),
        readiness_checks: ServerConfig::default().readiness_checks(),
        bundle_output_root: None,
        ack_policy: policy,
        quarantine: Default::default(),
    });
    build_router(state)
}

fn ack_policy_request(message: &str, profile: &str) -> Request<Body> {
    let body = json!({
        "message": message,
        "profile": profile,
        "mllp_framed": false,
        "mllp_frame": false
    });

    Request::builder()
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            8080,
        ))))
        .uri("/hl7/ack-policy")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

async fn post_ack_policy(
    app: axum::Router,
    message: &str,
    profile: &str,
) -> (StatusCode, Value, String) {
    let response = app
        .oneshot(ack_policy_request(message, profile))
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let value = serde_json::from_str(&body_text).unwrap_or_else(|_| json!({}));
    (status, value, body_text)
}

#[tokio::test]
async fn test_ack_policy_accepts_valid_message_with_original_mode() {
    let (status, body, body_text) = post_ack_policy(
        test_router(AckPolicyConfig::default(), None),
        SAMPLE_MSG,
        VALID_PROFILE,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ack_code"], "AA");
    assert_eq!(body["decision"]["mode"], "original");
    assert_eq!(body["decision"]["outcome"], "accepted");
    assert_eq!(body["decision"]["reason"], "valid");
    assert_eq!(body["validation_report"]["valid"], true);
    assert!(
        body["ack_message"]
            .as_str()
            .unwrap()
            .contains("MSA|AA|CTRL123")
    );
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_ack_policy_rejects_validation_failure_without_phi() {
    let (status, body, body_text) = post_ack_policy(
        test_router(AckPolicyConfig::default(), None),
        SAMPLE_MSG,
        INVALID_PROFILE,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ack_code"], "AR");
    assert_eq!(body["decision"]["outcome"], "rejected");
    assert_eq!(body["decision"]["reason"], "validation_error");
    assert_eq!(
        body["decision"]["error_text"],
        "message validation failed with 1 issue(s)"
    );
    assert_eq!(body["validation_report"]["valid"], false);
    assert!(
        body["ack_message"]
            .as_str()
            .unwrap()
            .contains("MSA|AR|CTRL123")
    );
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_ack_policy_uses_enhanced_mode_codes() {
    let policy = AckPolicyConfig {
        mode: AckPolicyMode::Enhanced,
        ..AckPolicyConfig::default()
    };

    let (status, body, body_text) =
        post_ack_policy(test_router(policy, None), SAMPLE_MSG, VALID_PROFILE).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ack_code"], "CA");
    assert_eq!(body["decision"]["mode"], "enhanced");
    assert!(
        body["ack_message"]
            .as_str()
            .unwrap()
            .contains("MSA|CA|CTRL123")
    );
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_ack_policy_rejects_parse_error_when_msh_is_usable() {
    let (status, body, body_text) = post_ack_policy(
        test_router(AckPolicyConfig::default(), None),
        PARTIAL_PARSE_MESSAGE,
        VALID_PROFILE,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ack_code"], "AR");
    assert_eq!(body["decision"]["reason"], "parse_error");
    assert!(body["validation_report"].is_null());
    assert!(
        body["ack_message"]
            .as_str()
            .unwrap()
            .contains("MSA|AR|CTRL123")
    );
    assert_no_phi(&body_text);
}

#[tokio::test]
async fn test_ack_policy_still_requires_auth_when_configured() {
    let response = test_router(AckPolicyConfig::default(), Some("secret"))
        .oneshot(ack_policy_request(SAMPLE_MSG, VALID_PROFILE))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn assert_no_phi(content: &str) {
    assert_no_phi_leak_sentinels("ack-policy response", content);
}
