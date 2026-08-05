//! HTTP route definitions.

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::HeaderValue,
    middleware,
    routing::{get, post},
};
use std::sync::Arc;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{
    ack_handler, ack_policy_handler, bundle_handler, corpus_diff_handler,
    corpus_fingerprint_handler, corpus_summarize_handler, health_handler, normalize_handler,
    parse_handler, profile_explain_handler, profile_lint_handler, profile_test_handler,
    ready_handler, replay_handler, validate_handler, validate_redacted_handler,
};
use crate::metrics::{metrics_handler, middleware::metrics_middleware};
use crate::middleware::{auth_middleware, create_concurrency_limit_layer, trace_request};
use crate::server::{AppState, CorsAllowedOrigins, ServerConfig};

/// OpenAPI specification content
const OPENAPI_YAML: &str = include_str!(env!("HL7V2_OPENAPI_YAML"));

/// Build the application router
pub fn build_router(state: Arc<AppState>) -> Router {
    build_router_with_body_limit(state, ServerConfig::default().max_body_size)
}

/// Build the application router with an explicit HTTP request-body limit.
pub(crate) fn build_router_with_body_limit(state: Arc<AppState>, max_body_size: usize) -> Router {
    // Rate limit configuration: 100 requests per minute per IP
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2) // approximately 120 per minute
            .burst_size(20)
            .finish()
            .unwrap(),
    );

    // Create API routes
    let mut api_routes = Router::new()
        .route("/parse", post(parse_handler))
        .route("/validate", post(validate_handler))
        .route("/validate-redacted", post(validate_redacted_handler))
        .route("/bundle", post(bundle_handler))
        .route("/replay", post(replay_handler))
        .route("/corpus/summarize", post(corpus_summarize_handler))
        .route("/corpus/fingerprint", post(corpus_fingerprint_handler))
        .route("/corpus/diff", post(corpus_diff_handler))
        .route("/profile/lint", post(profile_lint_handler))
        .route("/profile/explain", post(profile_explain_handler))
        .route("/profile/test", post(profile_test_handler))
        .route("/ack", post(ack_handler))
        .route("/ack-policy", post(ack_policy_handler))
        .route("/normalize", post(normalize_handler));

    // Apply authentication if API key is configured in state
    if state.api_key.is_some() {
        api_routes = api_routes.layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));
    }

    // Metrics can expose operational details, so apply the same API-key
    // protection when authentication is configured. Health and readiness
    // remain public for load-balancer and orchestration probes.
    let mut metrics_routes = Router::new().route("/metrics", get(metrics_handler));
    if state.api_key.is_some() {
        metrics_routes = metrics_routes.route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));
    }

    let cors_layer = build_cors_layer(&state.cors_allowed_origins);

    // Main router
    Router::new()
        .merge(
            SwaggerUi::new("/api/docs")
                .config(utoipa_swagger_ui::Config::from("/api/openapi.yaml")),
        )
        .route(
            "/api/openapi.yaml",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/yaml")],
                    OPENAPI_YAML,
                )
            }),
        )
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .merge(metrics_routes)
        .nest("/hl7", api_routes)
        // Keep the extractor limit for its 413 rejection behavior and also
        // wrap every request body so raw-body consumers cannot bypass it.
        .layer(RequestBodyLimitLayer::new(max_body_size))
        .layer(DefaultBodyLimit::max(max_body_size))
        .with_state(state)
        // Middleware layers (bottom to top execution order)
        .layer(middleware::from_fn(metrics_middleware))
        .layer(CompressionLayer::new())
        .layer(cors_layer)
        .layer(GovernorLayer::new(governor_conf))
        .layer(create_concurrency_limit_layer()) // Concurrency limiting applied first
        .layer(middleware::from_fn(trace_request)) // Request tracing remains outermost
}

/// Build CORS layer
fn build_cors_layer(origins: &CorsAllowedOrigins) -> CorsLayer {
    let layer = CorsLayer::new().allow_methods(Any).allow_headers(Any);

    match origins {
        CorsAllowedOrigins::Any => layer.allow_origin(Any),
        CorsAllowedOrigins::List(origins) => {
            let parsed_origins = origins
                .iter()
                .map(|origin| HeaderValue::from_str(origin))
                .collect::<Result<Vec<_>, _>>();
            match parsed_origins {
                Ok(origins) => layer.allow_origin(AllowOrigin::list(origins)),
                Err(_) => {
                    tracing::error!(
                        "CORS allowlist contains an invalid origin; refusing to enable CORS"
                    );
                    CorsLayer::new()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use hl7v2_test_utils::deterministic_api_key;
    use http_body_util::BodyExt;
    use std::time::Instant;
    use tower::ServiceExt; // For `oneshot`

    fn build_test_router_with_api_key(seed: &str) -> (Router, String) {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let api_key = deterministic_api_key(seed);
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: Some(api_key.clone()),
            max_message_size: crate::server::ServerConfig::default().max_message_size,
            cors_allowed_origins: CorsAllowedOrigins::default(),
            readiness_checks: crate::server::ServerConfig::default().readiness_checks(),
            bundle_output_root: None,
            ack_policy: Default::default(),
            quarantine: Default::default(),
        });
        (build_router(state), api_key)
    }

    fn parse_request_payload() -> String {
        let request_body = serde_json::json!({
            "message": "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|123456|P|2.5\rPID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M\r",
            "mllp_framed": false,
            "options": {
                "include_json": true,
                "validate_structure": true
            }
        });

        serde_json::to_string(&request_body).unwrap()
    }

    async fn request_parse(app: Router, api_key: Option<&str>) -> (StatusCode, Vec<u8>) {
        let mut request = Request::builder()
            .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .uri("/hl7/parse")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(parse_request_payload()))
            .unwrap();

        if let Some(key) = api_key {
            request
                .headers_mut()
                .insert("X-API-Key", axum::http::HeaderValue::from_str(key).unwrap());
        }

        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, body.to_vec())
    }

    async fn request_metrics(app: Router, api_key: Option<&str>) -> StatusCode {
        let mut request = Request::builder()
            .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();

        if let Some(key) = api_key {
            request
                .headers_mut()
                .insert("X-API-Key", axum::http::HeaderValue::from_str(key).unwrap());
        }

        app.oneshot(request).await.unwrap().status()
    }

    async fn request_public_probe(app: Router, uri: &str) -> StatusCode {
        let request = Request::builder()
            .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        app.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: None,
            max_message_size: crate::server::ServerConfig::default().max_message_size,
            cors_allowed_origins: CorsAllowedOrigins::default(),
            readiness_checks: crate::server::ServerConfig::default().readiness_checks(),
            bundle_output_root: None,
            ack_policy: Default::default(),
            quarantine: Default::default(),
        });

        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"status\":\"healthy\""));
    }

    #[tokio::test]
    async fn test_configured_http_body_limit_applies_before_body_consumers() {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: None,
            max_message_size: crate::server::ServerConfig::default().max_message_size,
            cors_allowed_origins: CorsAllowedOrigins::default(),
            readiness_checks: crate::server::ServerConfig::default().readiness_checks(),
            bundle_output_root: None,
            ack_policy: Default::default(),
            quarantine: Default::default(),
        });

        let app = build_router_with_body_limit(state, 64);
        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/health")
                    .method("GET")
                    .header("Content-Length", "65")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_parse_endpoint() {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: None,
            max_message_size: crate::server::ServerConfig::default().max_message_size,
            cors_allowed_origins: CorsAllowedOrigins::default(),
            readiness_checks: crate::server::ServerConfig::default().readiness_checks(),
            bundle_output_root: None,
            ack_policy: Default::default(),
            quarantine: Default::default(),
        });

        let app = build_router(state);

        // Create a proper HL7 message with correct delimiters
        let hl7_message = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|123456|P|2.5\rPID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M\r";

        let request_body = serde_json::json!({
            "message": hl7_message,
            "mllp_framed": false,
            "options": {
                "include_json": true,
                "validate_structure": true
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/parse")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let response_data: crate::models::ParseResponse =
            serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(response_data.metadata.message_type, "ADT^A01");
        assert_eq!(response_data.metadata.version, "2.5");
        assert_eq!(response_data.metadata.sending_application, "SendingApp");
        assert!(response_data.message.is_some());
    }

    #[tokio::test]
    async fn test_configured_http_body_limit_rejects_oversized_request() {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: None,
            max_message_size: crate::server::ServerConfig::default().max_message_size,
            cors_allowed_origins: CorsAllowedOrigins::default(),
            readiness_checks: crate::server::ServerConfig::default().readiness_checks(),
            bundle_output_root: None,
            ack_policy: Default::default(),
            quarantine: Default::default(),
        });

        let app = build_router_with_body_limit(state, 64);
        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/parse")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(parse_request_payload()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_parse_endpoint_rejects_missing_api_key() {
        let key_seed = "server::api-auth::missing-key";
        let (app, _) = build_test_router_with_api_key(key_seed);
        let (status, _) = request_parse(app, None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_parse_endpoint_accepts_valid_deterministic_api_key() {
        let key_seed = "server::api-auth::valid-key";
        let (app, key) = build_test_router_with_api_key(key_seed);
        let (status, body_bytes) = request_parse(app, Some(&key)).await;

        assert_eq!(status, StatusCode::OK);
        let response_data: crate::models::ParseResponse =
            serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(response_data.metadata.message_type, "ADT^A01");
        assert_eq!(response_data.metadata.version, "2.5");
        assert_eq!(response_data.metadata.sending_application, "SendingApp");
        assert!(response_data.message.is_some());
    }

    #[tokio::test]
    async fn test_metrics_endpoint_rejects_missing_api_key() {
        let (app, _) = build_test_router_with_api_key("server::metrics-auth::missing-key");

        assert_eq!(request_metrics(app, None).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_endpoint_rejects_invalid_api_key() {
        let (app, _) = build_test_router_with_api_key("server::metrics-auth::invalid-key");

        assert_eq!(
            request_metrics(app, Some("not-the-configured-key")).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn test_metrics_endpoint_accepts_valid_deterministic_api_key() {
        let (app, key) = build_test_router_with_api_key("server::metrics-auth::valid-key");

        assert_eq!(request_metrics(app, Some(&key)).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_and_ready_remain_public_when_api_key_is_configured() {
        let (app, _) = build_test_router_with_api_key("server::metrics-auth::public-probes");

        assert_eq!(
            request_public_probe(app.clone(), "/health").await,
            StatusCode::OK
        );
        assert_eq!(request_public_probe(app, "/ready").await, StatusCode::OK);
    }
}
