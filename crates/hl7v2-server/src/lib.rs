//! HTTP/REST API server for HL7v2 message processing.
//!
//! This crate provides a production-ready HTTP server built with Axum that exposes
//! REST endpoints for:
//! - Parsing HL7 messages
//! - Validating messages against profiles
//! - Generating ACK messages
//! - Normalizing HL7 messages
//! - Health checks
//!
//! # Features
//!
//! - **High Performance**: Built on Axum + Tokio for async performance
//! - **Observability**: Structured logging with tracing, Prometheus metrics
//! - **Security**: Authentication, authorization, rate limiting
//! - **Standards**: OpenAPI/Swagger documentation, JSON:API responses
//!
//! # Example
//!
//! ```no_run
//! use hl7v2_server::Server;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = Server::builder()
//!         .bind("0.0.0.0:8080")
//!         .build();
//!
//!     server.serve().await?;
//!     Ok(())
//! }
//! ```

pub mod grpc;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod server;

pub use routes::build_router;
pub use server::{AppState, CorsAllowedOrigins, Server, ServerBuilder, ServerConfig};

/// Server error types
#[derive(Debug, thiserror::Error)]
#[expect(
    clippy::error_impl_error,
    reason = "public server API intentionally exposes Error as the crate result error type"
)]
pub enum Error {
    /// Failed to bind the underlying server transport.
    #[error("Bind error: {0}")]
    Bind(#[from] std::io::Error),

    /// Configuration failed validation.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Failed to parse the incoming HL7 message.
    #[error("Parse error: {0}")]
    Parse(#[from] hl7v2_core::Error),

    /// Validation failed for message contents.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Internal server processing error.
    #[error("Internal server error: {0}")]
    Internal(String),
}

/// Result type for server operations
pub type Result<T> = std::result::Result<T, Error>;
