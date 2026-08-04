//! Integration tests for the /hl7/parse endpoint.

#![expect(
    clippy::unwrap_used,
    reason = "legacy parse endpoint tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hl7v2_test_utils::safe_error_phi_parity_fixture;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_valid_adt_a01_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "mllp_framed": false,
        "options": {
            "include_json": true,
            "validate_structure": false
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Valid ADT^A01 message should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_valid_adt_a04_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A04_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Valid ADT^A04 message should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_valid_oru_r01_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ORU_R01_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Valid ORU^R01 message should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_minimal_valid_message() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Minimal valid message (MSH only) should parse successfully"
    );
}

#[tokio::test]
async fn test_parse_malformed_message_returns_error() {
    let app = common::create_test_router();
    let fixture = safe_error_phi_parity_fixture().unwrap();

    let request_body = json!({
        "message": &fixture.malformed_message.message,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::OK,
        "Malformed message should return error"
    );
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Should return 4xx or 5xx status code"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    let body_json: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(
        body_json.get("code").and_then(Value::as_str),
        Some(fixture.malformed_message.rest_code.as_str())
    );
    assert_eq!(
        body_json.get("location").and_then(Value::as_str),
        Some(fixture.malformed_message.rest_location.as_str())
    );
    assert!(
        body_json
            .get("safe_detail")
            .and_then(Value::as_str)
            .is_some_and(
                |detail| detail.contains(&fixture.malformed_message.rest_safe_detail_contains)
            )
    );
    assert!(
        body_json
            .get("suggested_next_action")
            .and_then(Value::as_str)
            .is_some_and(|action| action.contains(&fixture.malformed_message.rest_action_contains))
    );
    fixture.assert_no_forbidden("REST parse safe error", &body_text);
}

#[tokio::test]
async fn test_parse_invalid_encoding_may_succeed_if_has_msh() {
    let app = common::create_test_router();

    // Note: "MSH|Wrong encoding characters" may actually parse successfully
    // since it has MSH and field separator. The encoding characters are
    // in MSH.2, so this is technically a valid (though non-standard) message.
    let request_body = json!({
        "message": common::fixtures::INVALID_ENCODING,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // This test just verifies the endpoint doesn't crash
    // The actual parsing behavior depends on the parser's strictness
    assert!(
        response.status().is_success() || response.status().is_client_error(),
        "Should handle message gracefully, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_parse_empty_request_body_returns_400() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Empty request body should return 400 or 422, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_parse_rejects_message_over_configured_application_limit() {
    let app = common::create_test_router_with_message_size_limit(64);
    let message = format!(
        "{}{}",
        common::fixtures::MINIMAL_VALID,
        "ZTX|oversized\r".repeat(8)
    );
    let request_body = json!({
        "message": message,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body.get("code").and_then(Value::as_str),
        Some("MESSAGE_TOO_LARGE")
    );
    assert!(
        body.get("safe_detail")
            .and_then(Value::as_str)
            .is_some_and(|detail| detail.contains("configured application-level size limit"))
    );
}

#[tokio::test]
async fn test_parse_accepts_mllp_payload_at_decoded_size_limit() {
    let app =
        common::create_test_router_with_message_size_limit(common::fixtures::MINIMAL_VALID.len());
    let framed = hl7v2::wrap_mllp(common::fixtures::MINIMAL_VALID.as_bytes());
    let message = String::from_utf8(framed).unwrap();
    let request_body = json!({
        "message": message,
        "mllp_framed": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_parse_reports_decoded_size_for_oversized_mllp_payload() {
    let payload = common::fixtures::MINIMAL_VALID.as_bytes();
    let app = common::create_test_router_with_message_size_limit(payload.len() - 1);
    let framed = hl7v2::wrap_mllp(payload);
    let request_body = json!({
        "message": String::from_utf8(framed).unwrap(),
        "mllp_framed": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    let expected = format!(
        "message size {} bytes exceeds maximum of {} bytes",
        payload.len(),
        payload.len() - 1
    );
    assert_eq!(
        body.get("message").and_then(Value::as_str),
        Some(expected.as_str())
    );
}

#[tokio::test]
async fn test_parse_invalid_json_returns_400() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("not valid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Invalid JSON should return 400 Bad Request"
    );
}

#[tokio::test]
async fn test_parse_response_contains_segments() {
    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::ADT_A01_VALID,
        "mllp_framed": false,
        "options": {
            "include_json": true
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Response should contain segment information
    assert!(
        body_str.contains("MSH") || body_str.contains("segments") || body_str.contains("metadata"),
        "Parse response should contain segment information"
    );
}

#[tokio::test]
async fn test_parse_get_method_not_allowed() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "GET method should not be allowed on /hl7/parse"
    );
}
