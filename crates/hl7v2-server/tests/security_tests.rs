use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_auth_enforced_missing_header() {
    // This test confirms that authentication is enforced when an API key is configured
    // and the request is missing the X-API-Key header.

    let app = common::create_test_router_with_auth("secret-key-123");

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
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
        StatusCode::UNAUTHORIZED,
        "Should return 401 Unauthorized when API key is missing"
    );
}

#[tokio::test]
async fn test_auth_enforced_invalid_key() {
    // This test confirms that authentication is enforced when an API key is configured
    // and the request provides an invalid key.

    let app = common::create_test_router_with_auth("secret-key-123");

    let request_body = json!({
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
        "Should return 401 Unauthorized when API key is invalid"
    );
}

#[tokio::test]
async fn test_auth_success_valid_key() {
    // This test confirms that requests succeed when the correct API key is provided.

    let app = common::create_test_router_with_auth("secret-key-123");

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "secret-key-123")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should return 200 OK when valid API key is provided"
    );
}

#[tokio::test]
async fn test_no_auth_mode_still_works() {
    // This test confirms that existing tests (which use create_test_router with no auth)
    // still work (i.e., dev mode/testing mode).

    let app = common::create_test_router();

    let request_body = json!({
        "message": common::fixtures::MINIMAL_VALID,
        "mllp_framed": false
    });

    let response = app
        .oneshot(
            Request::builder()
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
        "Should return 200 OK when no auth is configured (dev mode)"
    );
}
