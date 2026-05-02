//! HTTP middleware components.
//!
//! This module provides middleware for:
//! - Request tracing and logging
//! - Metrics collection (Prometheus)
//! - Authentication and authorization
//! - Rate limiting
//! - Concurrency control

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use http::StatusCode;
use std::sync::Arc;
use tracing::info_span;

use crate::server::AppState;

/// Trace request middleware
///
/// Wraps each request in a tracing span with request metadata.
pub async fn trace_request(request: Request, next: Next) -> Response {
    let span = info_span!(
        "HTTP request",
        method = %request.method(),
        uri = %request.uri(),
    );

    let _enter = span.enter();
    next.run(request).await
}

/// API key authentication middleware
///
/// Validates requests against the HL7 API key configured in state.
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
) -> std::result::Result<Response, StatusCode> {
    const API_KEY_HEADER: &str = "X-API-Key";

    // If no API key is configured, allow all requests (this branch should not
    // be hit if the middleware is applied correctly via build_router)
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
pub fn create_concurrency_limit_layer() -> tower::limit::ConcurrencyLimitLayer {
    tower::limit::ConcurrencyLimitLayer::new(100)
}

/// Create a custom concurrency limiting layer
///
/// # Arguments
///
/// * `max` - Maximum number of concurrent requests
pub fn create_custom_concurrency_limit_layer(max: usize) -> tower::limit::ConcurrencyLimitLayer {
    tower::limit::ConcurrencyLimitLayer::new(max)
}

#[cfg(test)]
mod tests {
    use super::*;

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
