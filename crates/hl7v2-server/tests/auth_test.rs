use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_parse_without_api_key_returns_401() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}")) // Body doesn't matter much if auth fails first
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request without API key should return 401"
    );
}

#[tokio::test]
async fn test_parse_with_wrong_api_key_returns_401() {
    let app = common::create_test_router();

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

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request with wrong API key should return 401"
    );
}

#[tokio::test]
async fn test_parse_with_correct_api_key_allows_access() {
    let app = common::create_test_router();

    // We expect 400 Bad Request (or similar) because the body is empty JSON {}, which is invalid for parse
    // But getting anything other than 401 means we passed authentication check.

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-API-Key", "test-key")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request with correct API key should NOT return 401"
    );
}
