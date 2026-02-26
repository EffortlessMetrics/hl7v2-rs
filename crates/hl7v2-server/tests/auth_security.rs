use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hl7v2_server::server::AppState;
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

// Helper to create router with a specific API key
fn create_test_router(api_key: Option<String>) -> Router {
    // This is safe to call multiple times due to OnceLock usage in implementation
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();

    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key,
    });

    hl7v2_server::routes::build_router(state)
}

#[tokio::test]
async fn test_auth_missing_header() {
    let app = create_test_router(Some("secret-key".to_string()));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_invalid_key() {
    let app = create_test_router(Some("secret-key".to_string()));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "wrong-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_valid_key() {
    let app = create_test_router(Some("secret-key".to_string()));

    // We expect 400 Bad Request (because body is empty/invalid) but NOT 401 Unauthorized
    // This proves authentication passed
    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "secret-key")
                .body(Body::from("{}")) // Empty JSON object to avoid immediate parse error if possible, or just check it's not 401
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_health_unprotected() {
    let app = create_test_router(Some("secret-key".to_string()));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_no_key_configured_fails_secure() {
    // If no key is configured, it should fail closed (500) for protected endpoints
    let app = create_test_router(None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
