//! Integration tests for the root endpoint.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_root_endpoint_returns_200() {
    let app = common::create_test_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_root_endpoint_returns_json() {
    let app = common::create_test_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());

    assert!(
        content_type.is_some() && content_type.unwrap().contains("application/json"),
        "Response should be JSON"
    );
}

#[tokio::test]
async fn test_root_endpoint_contains_service_info() {
    let app = common::create_test_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("HL7v2 API Server"), "Should contain service name");
    assert!(body_str.contains("endpoints"), "Should list endpoints");
    assert!(body_str.contains("/hl7/parse"), "Should list parse endpoint");
}
