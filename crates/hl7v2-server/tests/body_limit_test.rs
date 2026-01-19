use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_request_body_too_large() {
    // Limit body to 10 bytes
    let app = common::create_limited_test_router(10);

    // Create a body larger than 10 bytes
    let large_body = "This string is definitely longer than 10 bytes.";
    let request_body = serde_json::json!({
        "message": large_body,
    });
    let body_str = serde_json::to_string(&request_body).unwrap();

    // verify our assumption about size
    assert!(body_str.len() > 10);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/hl7/parse")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_request_body_within_limit() {
    // Limit body to 1000 bytes (plenty for this message)
    let app = common::create_limited_test_router(1000);

    let small_body = "Small body";
    let request_body = serde_json::json!({
        "message": small_body,
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

    // Should NOT be 413. It might be 400 (if parsing fails) or 200, but definitely not 413.
    assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
