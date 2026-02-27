use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_access_without_api_key_is_denied() {
    // This test confirms that accessing protected endpoints without an API key
    // returns 401 Unauthorized.

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
                // No X-API-Key header
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Expect 401 Unauthorized now that auth middleware is applied
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Endpoint should NOT be accessible without API Key"
    );
}
