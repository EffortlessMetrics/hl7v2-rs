//! HTTP route definitions.

use axum::{
    Router, middleware,
    routing::{get, post},
};
use std::sync::Arc;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{health_handler, parse_handler, validate_handler};
use crate::metrics::{metrics_handler, middleware::metrics_middleware};
use crate::middleware::{auth_middleware, create_concurrency_limit_layer};
use crate::server::AppState;

/// OpenAPI specification content
const OPENAPI_YAML: &str = include_str!("../../../schemas/openapi/hl7v2-api.yaml");

/// Build the application router
pub fn build_router(state: Arc<AppState>) -> Router {
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
        .route("/validate", post(validate_handler));

    // Apply authentication if API key is configured in state
    if state.api_key.is_some() {
        api_routes = api_routes.layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));
    }

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
        .route("/metrics", get(metrics_handler))
        .nest("/hl7", api_routes)
        .with_state(state)
        // Middleware layers (bottom to top execution order)
        .layer(middleware::from_fn(metrics_middleware))
        .layer(CompressionLayer::new())
        .layer(build_cors_layer())
        .layer(TraceLayer::new_for_http())
        .layer(GovernorLayer::new(governor_conf))
        .layer(create_concurrency_limit_layer()) // Concurrency limiting applied first (last in stack)
}

/// Handler for GET /ready
async fn ready_handler() -> &'static str {
    // Simple readiness check - if we can respond, we're ready
    // In production, you might want to check database connections, etc.
    "{\"ready\":true}"
}

/// Build CORS layer
fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
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

    #[tokio::test]
    async fn test_health_endpoint() {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: None,
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
    async fn test_parse_endpoint() {
        let metrics_handle = crate::metrics::init_metrics_recorder();
        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: None,
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

        assert_eq!(response_data.metadata.message_type, "ADT");
        assert_eq!(response_data.metadata.version, "2.5");
        assert_eq!(response_data.metadata.sending_application, "SendingApp");
        assert!(response_data.message.is_some());
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

        assert_eq!(response_data.metadata.message_type, "ADT");
        assert_eq!(response_data.metadata.version, "2.5");
        assert_eq!(response_data.metadata.sending_application, "SendingApp");
        assert!(response_data.message.is_some());
    }
}
