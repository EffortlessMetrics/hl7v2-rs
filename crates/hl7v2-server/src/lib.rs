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

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    reason = "pre-existing server runtime lint debt is tracked in policy/clippy-debt.toml"
)]

mod audit;

pub mod evidence;
pub mod grpc;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod redaction;
pub mod routes;
pub mod server;

pub(crate) const PROFILE_LOAD_SAFE_MESSAGE: &str =
    "profile could not be loaded; run profile lint for details";

pub use models::{
    AckPolicyAcceptOn, AckPolicyConfig, AckPolicyMode, AckPolicyRejectCondition, QuarantineConfig,
};
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
    Parse(#[from] hl7v2::Error),

    /// Validation failed for message contents.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Internal server processing error.
    #[error("Internal server error: {0}")]
    Internal(String),
}

/// Result type for server operations
pub type Result<T> = std::result::Result<T, Error>;
