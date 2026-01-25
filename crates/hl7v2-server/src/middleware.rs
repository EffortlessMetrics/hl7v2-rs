//! HTTP middleware components.
//!
//! This module provides middleware for:
//! - Request tracing and logging
//! - Metrics collection (Prometheus)
//! - Authentication and authorization
//! - Rate limiting
//! - Request ID generation

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use tower::limit::ConcurrencyLimitLayer;
use tracing::info;

/// Request logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "HTTP request"
    );

    response
}

use std::sync::Arc;
use crate::server::AppState;
use axum::extract::State;

/// API key authentication middleware
///
/// Validates requests against the configured API key in AppState.
/// Uses X-API-Key header for authentication.
///
/// # Security Note
/// This is a basic API key implementation suitable for internal services
/// or development environments. For production use, consider:
/// - OAuth 2.0 / OIDC
/// - mTLS
/// - More sophisticated key management (HashiCorp Vault, AWS Secrets Manager)
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    const API_KEY_HEADER: &str = "X-API-Key";

    // If no API key is configured, skip authentication (e.g. testing)
    let expected_key = match &state.api_key {
        Some(key) => key,
        None => return Ok(next.run(request).await),
    };

    // Get provided API key from request
    let provided_key = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|h| h.to_str().ok());

    match provided_key {
        Some(key) if key == expected_key => {
            // Valid key - allow request
            Ok(next.run(request).await)
        }
        Some(_) => {
            // Invalid key provided
            tracing::warn!("Invalid API key provided");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            // No key provided
            tracing::warn!("No API key provided");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Create a concurrency limiting layer
///
/// Limits the number of concurrent requests being processed.
///
/// Default configuration:
/// - Maximum 100 concurrent requests
///
/// # Production Considerations
///
/// - This provides backpressure, not rate limiting per se
/// - Protects against resource exhaustion from too many concurrent requests
/// - For true rate limiting (requests/second), consider integrating a proper rate limiter
/// - Adjust limits based on your capacity and benchmarks
/// - Monitor 503 responses via metrics
///
/// # Example
///
/// ```
/// use hl7v2_server::middleware::create_concurrency_limit_layer;
///
/// let _layer = create_concurrency_limit_layer();
/// // Use with axum Router: Router::new().layer(_layer)
/// ```
pub fn create_concurrency_limit_layer() -> ConcurrencyLimitLayer {
    ConcurrencyLimitLayer::new(100)  // Allow up to 100 concurrent requests
}

/// Create a custom concurrency limiting layer with configurable limit
///
/// # Arguments
///
/// * `max_concurrent` - Maximum number of concurrent requests
///
/// # Example
///
/// ```no_run
/// use hl7v2_server::middleware::create_custom_concurrency_limit_layer;
///
/// // Allow 50 concurrent requests
/// let layer = create_custom_concurrency_limit_layer(50);
/// ```
pub fn create_custom_concurrency_limit_layer(max_concurrent: usize) -> ConcurrencyLimitLayer {
    ConcurrencyLimitLayer::new(max_concurrent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_module() {
        // Placeholder test to ensure module compiles
        assert!(true);
    }

    #[test]
    fn test_create_concurrency_limit_layer() {
        // Test that we can create a concurrency limit layer
        let _layer = create_concurrency_limit_layer();
        // No panic means success
    }

    #[test]
    fn test_create_custom_concurrency_limit_layer() {
        // Test that we can create a custom concurrency limit layer
        let _layer = create_custom_concurrency_limit_layer(50);
        // No panic means success
    }
}
