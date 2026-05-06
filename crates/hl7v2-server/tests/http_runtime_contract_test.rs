//! HTTP runtime/API contract tests.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::uninlined_format_args,
    reason = "legacy runtime contract tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_server::{AppState, CorsAllowedOrigins, build_router};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
use tower::ServiceExt;

const SAMPLE_MSG: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";
const CUSTOM_DELIMS_MSG: &str = "MSH*%$!?*SENDAPP*SENDFAC*RECVAPP*RECVFAC*202605030101**ADT%A01*CTRL123*P*2.5\rPID*1**123456%%%HOSP%MR**Doe%John**19700101*M\r";

fn test_router(api_key: Option<&str>, cors_allowed_origins: CorsAllowedOrigins) -> axum::Router {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: api_key.map(str::to_string),
        cors_allowed_origins,
    });
    build_router(state)
}

async fn post_json(app: axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri(uri)
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
    (status, value)
}

#[tokio::test]
async fn test_http_ack_maps_codes_and_preserves_control_id() {
    for code in ["AA", "AE", "AR"] {
        let (status, body) = post_json(
            test_router(None, CorsAllowedOrigins::default()),
            "/hl7/ack",
            json!({
                "message": SAMPLE_MSG,
                "code": code,
                "mllp_framed": false
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ack_code"], code);
        let ack = body["ack_message"].as_str().unwrap();
        assert!(ack.contains(&format!("MSA|{}|CTRL123", code)));
        assert_eq!(body["metadata"]["message_type"], "ACK^ADT");
    }
}

#[tokio::test]
async fn test_http_ack_malformed_message_returns_parse_error() {
    let (status, body) = post_json(
        test_router(None, CorsAllowedOrigins::default()),
        "/hl7/ack",
        json!({
            "message": "not hl7",
            "code": "AA"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PARSE_ERROR");
}

#[tokio::test]
async fn test_http_normalize_canonical_output_and_idempotence() {
    let app = test_router(None, CorsAllowedOrigins::default());
    let (status, body) = post_json(
        app.clone(),
        "/hl7/normalize",
        json!({
            "message": CUSTOM_DELIMS_MSG,
            "options": {
                "canonical_delimiters": true
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let normalized = body["normalized_message"].as_str().unwrap();
    assert!(normalized.starts_with("MSH|^~\\&|"));
    assert!(normalized.contains("ADT^A01"));
    assert_eq!(body["metadata"]["message_type"], "ADT^A01");

    let (status, renormalized) = post_json(
        app,
        "/hl7/normalize",
        json!({
            "message": normalized,
            "options": {
                "canonical_delimiters": true
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(renormalized["normalized_message"], normalized);
}

#[tokio::test]
async fn test_http_normalize_optional_mllp_framing() {
    let (status, body) = post_json(
        test_router(None, CorsAllowedOrigins::default()),
        "/hl7/normalize",
        json!({
            "message": SAMPLE_MSG,
            "options": {
                "mllp_frame": true
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let framed = body["normalized_message"].as_str().unwrap().as_bytes();
    assert_eq!(framed[0], hl7v2::MLLP_START);
    assert_eq!(hl7v2::unwrap_mllp(framed).unwrap(), SAMPLE_MSG.as_bytes());
}

#[tokio::test]
async fn test_new_hl7_routes_require_api_key_when_configured() {
    let app = test_router(Some("secret-key"), CorsAllowedOrigins::default());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/ack")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "message": SAMPLE_MSG,
                        "code": "AA"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/normalize")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "secret-key")
                .body(Body::from(
                    serde_json::to_string(&json!({ "message": SAMPLE_MSG })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_stays_public_when_api_key_is_configured() {
    let app = test_router(Some("secret-key"), CorsAllowedOrigins::default());
    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cors_uses_configured_origin_list() {
    let app = test_router(
        None,
        CorsAllowedOrigins::list(["https://app.example", "https://ops.example"]),
    );

    let allowed = app
        .clone()
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .header("Origin", "https://app.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        allowed
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "https://app.example"
    );

    let rejected = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/health")
                .header("Origin", "https://unknown.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        rejected
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
}
