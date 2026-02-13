#[cfg(test)]
mod tests {
    use hl7v2_server::routes::build_router;
    use hl7v2_server::server::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use std::time::Instant;
    use tower::ServiceExt; // for oneshot

    #[tokio::test]
    async fn test_auth_protection() {
        // Setup app state with a test key
        let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
        let api_key = "test-secret-key".to_string();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: api_key.clone(),
        });

        let app = build_router(state);

        // Case 1: Unauthenticated request (should fail)
        let response_unauth = app.clone()
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

        assert_eq!(response_unauth.status(), StatusCode::UNAUTHORIZED, "Endpoint should be protected");

        // Case 2: Authenticated request (should pass auth check, though body is empty so might be Bad Request)
        let response_auth = app
            .oneshot(
                Request::builder()
                    .uri("/hl7/parse")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .header("X-API-Key", &api_key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // It might be 400 Bad Request because body is empty, but definitely NOT 401 Unauthorized
        assert_ne!(response_auth.status(), StatusCode::UNAUTHORIZED, "Valid API key should be accepted");
    }
}
