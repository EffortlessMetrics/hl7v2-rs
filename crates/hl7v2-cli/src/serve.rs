//! Server mode implementation for the HL7 v2 CLI.
//!
//! This module provides the `hl7v2 serve` subcommand functionality, supporting:
//! - HTTP REST API server using Axum
//! - gRPC server (optional, behind feature flag)
//! - Graceful shutdown via Ctrl+C

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tracing::{info, error, warn};

use hl7v2_server::Server;

/// Server mode from CLI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServerMode {
    /// HTTP REST API server
    Http,
    /// gRPC server
    Grpc,
}

impl From<crate::ServerMode> for ServerMode {
    fn from(mode: crate::ServerMode) -> Self {
        match mode {
            crate::ServerMode::Http => ServerMode::Http,
            crate::ServerMode::Grpc => ServerMode::Grpc,
        }
    }
}

/// Run the server with the given configuration.
///
/// This function starts the HTTP or gRPC server and handles graceful shutdown
/// when Ctrl+C is pressed.
pub async fn run_server(
    mode: &crate::ServerMode,
    port: u16,
    host: &str,
    max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let server_mode = ServerMode::from(*mode);
    let bind_address = format!("{}:{}", host, port);
    
    match server_mode {
        ServerMode::Http => run_http_server(&bind_address, max_body_size).await,
        ServerMode::Grpc => run_grpc_server(&bind_address).await,
    }
}

/// Run the HTTP REST API server.
async fn run_http_server(bind_address: &str, max_body_size: usize) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting HL7 v2 HTTP server on {}", bind_address);
    
    // Create shutdown signal
    let shutdown = setup_shutdown_signal();
    
    // Build server configuration
    let server = Server::builder()
        .bind(bind_address)
        .max_body_size(max_body_size)
        .build();
    
    info!("Server configuration:");
    info!("  Bind address: {}", bind_address);
    info!("  Max body size: {} bytes", max_body_size);
    info!("  Endpoints:");
    info!("    GET  /health  - Health check");
    info!("    GET  /ready   - Readiness check");
    info!("    GET  /metrics - Prometheus metrics");
    info!("    POST /hl7/parse   - Parse HL7 message");
    info!("    POST /hl7/validate - Validate HL7 message");
    info!("");
    info!("Press Ctrl+C to shutdown gracefully");
    
    // Run server with shutdown signal
    tokio::select! {
        result = server.serve() => {
            match result {
                Ok(()) => info!("Server shutdown normally"),
                Err(e) => {
                    error!("Server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown => {
            info!("Shutdown signal received, stopping server...");
        }
    }
    
    info!("Server stopped");
    Ok(())
}

/// Run the gRPC server.
async fn run_grpc_server(bind_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    warn!("gRPC server mode is not yet implemented");
    info!("gRPC server would bind to: {}", bind_address);
    
    // TODO: Implement gRPC server when tonic integration is ready
    // This would use the hl7v2-server crate's gRPC functionality
    
    Err("gRPC server mode is not yet implemented. Use --mode http for now.".into())
}

/// Setup Ctrl+C shutdown signal handler.
fn setup_shutdown_signal() -> impl std::future::Future<Output = ()> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    
    // Set up Ctrl+C handler
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Ctrl+C received, initiating graceful shutdown...");
                shutdown_clone.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                error!("Failed to listen for Ctrl+C: {}", e);
            }
        }
    });
    
    // Return future that completes when shutdown is triggered
    async move {
        while !shutdown.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_mode_conversion() {
        assert_eq!(ServerMode::from(crate::ServerMode::Http), ServerMode::Http);
        assert_eq!(ServerMode::from(crate::ServerMode::Grpc), ServerMode::Grpc);
    }

    #[test]
    fn test_bind_address_format() {
        let host = "127.0.0.1";
        let port = 8080;
        let bind_address = format!("{}:{}", host, port);
        assert_eq!(bind_address, "127.0.0.1:8080");
    }

    #[tokio::test]
    async fn test_grpc_not_implemented() {
        let result = run_grpc_server("127.0.0.1:50051").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not yet implemented"));
    }
}
