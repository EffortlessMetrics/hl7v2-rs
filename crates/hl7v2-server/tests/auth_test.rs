//! Integration tests for authentication.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for oneshot

mod common;

#[tokio::test]
async fn test_unauthorized_access() {
    // Create router with AUTH enabled
    let app = common::create_test_router_with_auth("secret-key");

    // Request WITHOUT key
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "Endpoint should be protected");
}

#[tokio::test]
async fn test_authorized_access() {
    // Create router with AUTH enabled
    let app = common::create_test_router_with_auth("secret-key");

    // Request WITH key
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "secret-key")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should NOT be UNAUTHORIZED.
    // It might be 400 Bad Request because body is empty JSON, but that proves auth passed.
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED, "Valid key should pass auth");
    // Ideally check for 400 or 422
    assert!(response.status().is_client_error(), "Should return client error for empty body, but auth passed");
}

#[tokio::test]
async fn test_wrong_key_access() {
    // Create router with AUTH enabled
    let app = common::create_test_router_with_auth("secret-key");

    // Request WITH WRONG key
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "wrong-key")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "Wrong key should fail auth");
}
