//! Integration tests for error handling across all endpoints.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_404_for_unknown_route() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/this/route/does/not/exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Unknown routes should return 404 Not Found"
    );
}

#[tokio::test]
async fn test_cors_headers_present() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("Origin", "http://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // CORS headers should be present
    let cors_header = response.headers().get("access-control-allow-origin");
    assert!(
        cors_header.is_some(),
        "CORS headers should be present in responses"
    );
}

#[tokio::test]
async fn test_content_type_validation() {
    let app = common::create_test_router();

    // Try to POST to /hl7/parse with wrong content type
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "text/plain")
                .body(Body::from("some text"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum will reject non-JSON content type for JSON endpoints
    assert!(
        response.status() != StatusCode::OK,
        "Wrong content type should be rejected"
    );
}

#[tokio::test]
async fn test_large_request_handling() {
    let app = common::create_test_router();

    // Create a very large (but still reasonable) HL7 message
    let mut large_message = String::from(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\r"
    );

    // Add many NTE segments (notes can be quite large in practice)
    for i in 0..100 {
        large_message.push_str(&format!("NTE|{}||This is note number {}\r", i, i));
    }

    let request_body = serde_json::json!({
        "message": large_message,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "test-key")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle large but reasonable messages
    assert!(
        response.status() == StatusCode::OK || response.status().is_client_error(),
        "Should either parse large message or reject it gracefully. Got: {}", response.status()
    );
}

#[tokio::test]
async fn test_missing_content_type_header() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Missing content-type might be rejected or assumed as default
    assert!(
        response.status() != StatusCode::INTERNAL_SERVER_ERROR,
        "Missing content-type should not cause 500 error"
    );
}

#[tokio::test]
async fn test_options_request_for_cors() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("OPTIONS")
                .header("Origin", "http://example.com")
                .header("Access-Control-Request-Method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // CORS preflight should be handled
    assert_ne!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "OPTIONS request should be handled properly"
    );
}

#[tokio::test]
async fn test_gzip_compression_accepted() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("Accept-Encoding", "gzip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Compression header should not cause errors"
    );
}
