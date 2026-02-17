use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // For `oneshot`

mod common;

#[tokio::test]
async fn test_unprotected_endpoint_rejected() {
    let app = common::create_test_router();

    let request_body = r#"{
        "message": "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rEVN|A01|20231119120000\r",
        "mllp_framed": false
    }"#;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                // No X-API-Key header!
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "Endpoint should reject request without API key");
}

#[tokio::test]
async fn test_protected_endpoint_accepted() {
    let app = common::create_test_router();

    let request_body = r#"{
        "message": "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rEVN|A01|20231119120000\r",
        "mllp_framed": false,
        "options": {
            "include_json": true
        }
    }"#;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "test-key") // Correct API key
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Endpoint should accept request with correct API key");
}
