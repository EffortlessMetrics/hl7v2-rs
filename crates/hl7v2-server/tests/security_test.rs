use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_unauthorized_access() {
    // This test verifies that the /hl7/parse endpoint is protected by authentication.

    let app = common::create_test_router();

    let request_body = serde_json::json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                // No X-API-Key header
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Endpoint /hl7/parse should require authentication"
    );
}

#[tokio::test]
async fn test_parse_authorized_access() {
    // This test verifies that providing the correct API key grants access.

    let app = common::create_test_router();

    let request_body = serde_json::json!({
        "message": common::fixtures::MINIMAL_VALID,
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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Correct API key should grant access"
    );
}

#[tokio::test]
async fn test_parse_invalid_api_key() {
    // This test verifies that providing an invalid API key denies access.

    let app = common::create_test_router();

    let request_body = serde_json::json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "wrong-key")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Invalid API key should deny access"
    );
}
