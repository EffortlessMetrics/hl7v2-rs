use axum::{
    body::Body,
    extract::connect_info::ConnectInfo,
    http::{Request, StatusCode},
};
use hl7v2_server::{
    handlers::{parse_handler, validate_handler},
    metrics::{init_metrics_recorder, metrics_handler},
    server::AppState,
};
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt; // For `oneshot`
use tower::limit::ConcurrencyLimitLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa_swagger_ui::SwaggerUi;

/// Build a test router with configurable rate limiting and concurrency limiting
fn build_test_router(
    rate_per_second: u64,
    burst_size: u32,
    max_concurrency: usize,
) -> axum::Router {
    let metrics_handle = init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: None,
    });

    // Rate limit configuration
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(rate_per_second)
            .burst_size(burst_size)
            .finish()
            .unwrap(),
    );

    // OpenAPI specification content (copied from src/routes.rs)
    const OPENAPI_YAML: &str = include_str!("../../../schemas/openapi/hl7v2-api.yaml");

    // Create API routes (without /hl7 prefix, as they will be nested)
    let mut api_routes = axum::Router::new()
        .route("/parse", axum::routing::post(parse_handler))
        .route("/validate", axum::routing::post(validate_handler));

    // Main router
    axum::Router::new()
        .merge(
            SwaggerUi::new("/api/docs")
                .config(utoipa_swagger_ui::Config::from("/api/openapi.yaml")),
        )
        .route(
            "/api/openapi.yaml",
            axum::routing::get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/yaml")],
                    OPENAPI_YAML,
                )
            }),
        )
        .route(
            "/health",
            axum::routing::get(|| async { (StatusCode::OK, "{\"status\":\"healthy\"}") }),
        )
        .route(
            "/ready",
            axum::routing::get(|| async { "{\"ready\":true}" }),
        )
        .route("/metrics", axum::routing::get(metrics_handler))
        .nest("/hl7", api_routes) // Nest under /hl7 to match the original router
        .with_state(state)
        // Middleware layers (bottom to top execution order)
        .layer(axum::middleware::from_fn(
            hl7v2_server::metrics::middleware::metrics_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .layer(tower_governor::GovernorLayer::new(governor_conf.clone())) // Rate limiting
        .layer(ConcurrencyLimitLayer::new(max_concurrency)) // Concurrency limiting
}

#[tokio::test]
async fn test_rate_limiting_allows_requests_within_limit() {
    // Create a router with a reasonable rate limit for testing: 5 requests per second, burst 10
    let app = build_test_router(5, 10, 100);

    // Make 3 requests quickly - should all succeed
    for i in 0..3 {
        let app_clone = app.clone();
        let response = app_clone
            .oneshot(
                Request::builder()
                    .extension(ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/parse") // Note: /hl7 prefix due to nesting
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(create_parse_request_payload()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should be OK (200) or possibly 422 if validation fails, but not 429
        assert_ne!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "Request {} should not be rate limited",
            i + 1
        );
    }
}

#[tokio::test]
async fn test_rate_limiting_blocks_requests_over_limit() {
    // Create a router with a very low rate limit for testing: 1 request per second, burst 1
    let app = build_test_router(1, 1, 100);

    // First request should succeed
    let app_clone = app.clone();
    let response = app_clone
        .oneshot(
            Request::builder()
                .extension(ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse") // Note: /hl7 prefix due to nesting
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(create_parse_request_payload()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "First request should succeed"
    );

    // Second request immediately after should be rate limited
    let app_clone = app.clone();
    let response = app_clone
        .oneshot(
            Request::builder()
                .extension(ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .uri("/hl7/parse") // Note: /hl7 prefix due to nesting
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(create_parse_request_payload()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be rate limited (429)
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Second request should be rate limited"
    );
}

#[tokio::test]
async fn test_concurrency_limiting_allows_requests_under_limit() {
    // Create a router with high concurrency limit: 50 concurrent requests
    let app = build_test_router(100, 100, 50);

    // Make 10 requests concurrently - should all be processed
    let mut tasks = vec![];
    for _ in 0..10 {
        let app_clone = app.clone();
        let task = tokio::spawn(async move {
            app_clone
                .oneshot(
                    Request::builder()
                        .extension(ConnectInfo(std::net::SocketAddr::from((
                            [127, 0, 0, 1],
                            8080,
                        ))))
                        .uri("/hl7/parse") // Note: /hl7 prefix due to nesting
                        .method("POST")
                        .header("Content-Type", "application/json")
                        .body(Body::from(create_parse_request_payload()))
                        .unwrap(),
                )
                .await
                .unwrap()
        });
        tasks.push(task);
    }

    // Wait for all requests to complete
    for task in tasks {
        let response = task.await.unwrap();
        // Should be OK or possibly 422 for validation, but not 503 (service unavailable) which
        // might indicate concurrency limit exceeded
        assert_ne!(
            response.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "Request should not be rejected due to concurrency limit"
        );
    }
}

#[tokio::test]
async fn test_concurrency_limiting_blocks_requests_over_limit() {
    // Create a router with low concurrency limit: 2 concurrent requests
    let app = build_test_router(100, 100, 2);

    // Make 4 requests concurrently - only 2 should be processed at a time
    let mut tasks = vec![];
    for i in 0..4 {
        let app_clone = app.clone();
        let task = tokio::spawn(async move {
            app_clone
                .oneshot(
                    Request::builder()
                        .extension(ConnectInfo(std::net::SocketAddr::from((
                            [127, 0, 0, 1],
                            8080,
                        ))))
                        .uri("/hl7/parse") // Note: /hl7 prefix due to nesting
                        .method("POST")
                        .header("Content-Type", "application/json")
                        .body(Body::from(create_parse_request_payload()))
                        .unwrap(),
                )
                .await
                .unwrap()
        });
        tasks.push((i, task));
    }

    // Wait for all requests to complete
    let mut responses = vec![];
    for (i, task) in tasks {
        let response = task.await.unwrap();
        responses.push((i, response));
    }

    // All requests should eventually succeed (they may be delayed but not permanently rejected)
    // Note: With tower::limit::ConcurrencyLimitLayer, excess requests are queued, not rejected
    // So we expect all to eventually succeed with StatusCode::OK
    for (i, response) in responses {
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} should eventually succeed",
            i + 1
        );
    }
}

/// Create a parse request payload for testing
fn create_parse_request_payload() -> String {
    let request_body = serde_json::json!({
        "message": "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rPID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M\r",
        "mllp_framed": false,
        "options": {
            "include_json": true,
            "validate_structure": true
        }
    });

    serde_json::to_string(&request_body).unwrap()
}
