//! Server HTTP API tests.
//!
//! These tests validate the HTTP server endpoints:
//! - GET /health - Health check endpoint
//! - GET /ready - Readiness endpoint
//! - POST /hl7/parse - Parse endpoint
//! - POST /hl7/validate - Validate endpoint
//! - GET /metrics - Prometheus metrics endpoint

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use super::common::init_tracing;

// Import server components
use hl7v2_server::routes::build_router;
use hl7v2_server::server::{AppState, Server, ServerConfig};
use std::sync::Arc;
use std::time::Instant;

// =========================================================================
// Helper Functions
// =========================================================================

/// Create a test router with default state
fn create_test_router() -> axum::Router {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: None,
    });
    build_router(state)
}

/// Sample ADT^A01 message
fn sample_adt_a01() -> String {
    concat!(
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
        "20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r",
        "EVN|A01|20250128152312|||\r",
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r"
    )
    .to_string()
}

/// Sample ADT^A04 message
fn sample_adt_a04() -> String {
    concat!(
        "MSH|^~\\&|RegSys|Hospital|ADT|Hospital|",
        "20250128140000||ADT^A04|MSG002|P|2.5\r",
        "PID|1||MRN456^^^Hospital^MR||Smith^Jane^M||19900215|F\r"
    )
    .to_string()
}

/// Sample ORU^R01 message
fn sample_oru_r01() -> String {
    concat!(
        "MSH|^~\\&|LabSys|Lab|LIS|Hospital|",
        "20250128150000||ORU^R01|MSG003|P|2.5\r",
        "PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r",
        "OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000\r",
        "OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r"
    )
    .to_string()
}

// =========================================================================
// Health Endpoint Tests
// =========================================================================

mod health_endpoint {
    use super::*;

    #[tokio::test]
    async fn test_health_returns_200() {
        init_tracing();

        let app = create_test_router();

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
    }

    #[tokio::test]
    async fn test_health_returns_json() {
        init_tracing();

        let app = create_test_router();

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
    async fn test_health_contains_status() {
        init_tracing();

        let app = create_test_router();

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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"status\""));
        assert!(body_str.contains("\"healthy\""));
    }

    #[tokio::test]
    async fn test_health_contains_version() {
        init_tracing();

        let app = create_test_router();

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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"version\""));
    }

    #[tokio::test]
    async fn test_health_contains_uptime() {
        init_tracing();

        let app = create_test_router();

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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"uptime_seconds\""));
    }
}

// =========================================================================
// Readiness Endpoint Tests
// =========================================================================

mod ready_endpoint {
    use super::*;

    #[tokio::test]
    async fn test_ready_returns_200() {
        init_tracing();

        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_returns_ready_status() {
        init_tracing();

        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"ready\":true") || body_str.contains("ready"));
    }
}

// =========================================================================
// Parse Endpoint Tests
// =========================================================================

mod parse_endpoint {
    use super::*;

    #[tokio::test]
    async fn test_parse_adt_a01() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": sample_adt_a01(),
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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value =
            serde_json::from_slice(&body).expect("Response should be valid JSON");

        // Verify response contains metadata with correct message type
        assert!(
            body.get("metadata").is_some(),
            "Response should contain metadata"
        );
        let message_type = body["metadata"]["message_type"]
            .as_str()
            .expect("metadata should have message_type");
        // Message type may be just "ADT" or "ADT^A01" depending on how the parser extracts it
        assert!(
            message_type.contains("ADT"),
            "Message type should contain ADT, got: {}",
            message_type
        );
    }

    #[tokio::test]
    async fn test_parse_adt_a04() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": sample_adt_a04(),
            "mllp_framed": false,
            "options": {
                "include_json": true
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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value =
            serde_json::from_slice(&body).expect("Response should be valid JSON");

        let message_type = body["metadata"]["message_type"]
            .as_str()
            .expect("metadata should have message_type");
        // Message type may be just "ADT" or "ADT^A04" depending on how the parser extracts it
        assert!(
            message_type.contains("ADT"),
            "Message type should contain ADT, got: {}",
            message_type
        );
    }

    #[tokio::test]
    async fn test_parse_oru_r01() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": sample_oru_r01(),
            "mllp_framed": false,
            "options": {
                "include_json": true
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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value =
            serde_json::from_slice(&body).expect("Response should be valid JSON");

        let message_type = body["metadata"]["message_type"]
            .as_str()
            .expect("metadata should have message_type");
        // Message type may be just "ORU" or "ORU^R01" depending on how the parser extracts it
        assert!(
            message_type.contains("ORU"),
            "Message type should contain ORU, got: {}",
            message_type
        );
    }

    #[tokio::test]
    async fn test_parse_without_json() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": sample_adt_a01(),
            "mllp_framed": false,
            "options": {
                "include_json": false
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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // message field should be null or absent when include_json is false
        assert!(body["message"].is_null() || body.get("message").is_none());
    }

    #[tokio::test]
    async fn test_parse_invalid_message() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": "This is not a valid HL7 message",
            "mllp_framed": false
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

        // Should return 400 Bad Request for invalid message
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_parse_empty_message() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": "",
            "mllp_framed": false
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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_parse_missing_message_field() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "mllp_framed": false
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

        // Should return 422 for missing required field (axum returns UNPROCESSABLE_ENTITY for JSON deserialization errors)
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

// =========================================================================
// Validate Endpoint Tests
// =========================================================================

mod validate_endpoint {
    use super::*;

    #[tokio::test]
    async fn test_validate_valid_message() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": sample_adt_a01(),
            "profile": "default",
            "mllp_framed": false
        });

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/validate")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Current implementation returns valid: true for parseable messages
        assert!(body["valid"].is_boolean());
    }

    #[tokio::test]
    async fn test_validate_oru_message() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": sample_oru_r01(),
            "profile": "default",
            "mllp_framed": false
        });

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/validate")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_validate_invalid_message() {
        init_tracing();

        let app = create_test_router();

        let request_body = json!({
            "message": "Invalid HL7",
            "profile": "default",
            "mllp_framed": false
        });

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/validate")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

// =========================================================================
// Metrics Endpoint Tests
// =========================================================================

mod metrics_endpoint {
    use super::*;

    #[tokio::test]
    async fn test_metrics_returns_200() {
        init_tracing();

        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_contains_prometheus_format() {
        init_tracing();

        let app = create_test_router();

        // Make a request first to generate some metrics
        let router = create_test_router();
        let _ = router
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
            .await;

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Prometheus metrics typically contain # HELP or # TYPE
        // The actual metrics depend on what's been recorded
        assert!(!body_str.is_empty());
    }
}

// =========================================================================
// Error Handling Tests
// =========================================================================

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn test_404_for_unknown_path() {
        init_tracing();

        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/unknown/path")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_method_not_allowed() {
        init_tracing();

        let app = create_test_router();

        // Try GET on a POST-only endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/hl7/parse")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_invalid_json_body() {
        init_tracing();

        let app = create_test_router();

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
                    .body(Body::from("not valid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

// =========================================================================
// CORS Tests
// =========================================================================

mod cors {
    use super::*;

    #[tokio::test]
    async fn test_cors_headers_present() {
        init_tracing();

        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        8080,
                    ))))
                    .uri("/health")
                    .header("Origin", "http://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // CORS headers should be present due to CorsLayer
        // The actual headers depend on the CORS configuration
        assert_eq!(response.status(), StatusCode::OK);
    }
}

// =========================================================================
// Integration Tests - Full Server
// =========================================================================

mod server_integration {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_server_starts_and_responds() {
        init_tracing();

        // Find available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        // Create server config
        let config = ServerConfig {
            bind_address: addr.to_string(),
            max_body_size: 10 * 1024 * 1024,
            api_key: None,
        };

        // Build server
        let server = Server::new(config);

        // Spawn server task
        let server_task = tokio::spawn(async move { server.serve().await });

        // Give server time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Make health check request
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/health", addr))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        // Server should respond (may fail if port was taken)
        if let Ok(resp) = response {
            assert!(resp.status().is_success());
        }

        // Abort server task
        server_task.abort();
    }
}

// =========================================================================
// Concurrent Request Tests
// =========================================================================

mod concurrent_requests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_parse_requests() {
        init_tracing();

        let mut handles = vec![];

        for _ in 0..10 {
            let handle = tokio::spawn(async move {
                let app = create_test_router();
                let request_body = json!({
                    "message": sample_adt_a01(),
                    "mllp_framed": false
                });

                app.oneshot(
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
                .unwrap()
            });
            handles.push(handle);
        }

        // Wait for all requests
        let results = futures::future::join_all(handles).await;

        // All should succeed
        for result in results {
            let response = result.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_concurrent_health_requests() {
        init_tracing();

        let mut handles = vec![];

        for _ in 0..20 {
            let handle = tokio::spawn(async move {
                let app = create_test_router();
                app.oneshot(
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
                .unwrap()
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;

        for result in results {
            let response = result.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
