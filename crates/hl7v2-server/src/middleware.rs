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

/// API key authentication middleware
///
/// Validates requests against the HL7V2_API_KEY environment variable.
/// Uses X-API-Key header for authentication.
///
/// # Security Note
/// This is a basic API key implementation suitable for internal services
/// or development environments. For production use, consider:
/// - OAuth 2.0 / OIDC
/// - mTLS
/// - More sophisticated key management (HashiCorp Vault, AWS Secrets Manager)
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    const API_KEY_HEADER: &str = "X-API-Key";

    // Load expected API key from environment
    let expected_key = match std::env::var("HL7V2_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        Ok(_) => {
            // Empty key configured - fail closed
            tracing::error!("HL7V2_API_KEY environment variable is empty");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(_) => {
            // No key configured - fail closed
            tracing::error!("HL7V2_API_KEY environment variable not set");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_module() {
        // Placeholder test to ensure module compiles
        assert!(true);
    }
}
