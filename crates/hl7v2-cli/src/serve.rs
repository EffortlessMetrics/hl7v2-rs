//! Server mode implementation for the HL7 v2 CLI.
//!
//! This module provides the `hl7v2 serve` subcommand functionality, supporting:
//! - HTTP REST API server using Axum
//! - gRPC server (optional, behind feature flag)
//! - Graceful shutdown via Ctrl+C

use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::{error, info};

use hl7v2_server::{Server, ServerConfig};

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
        ServerMode::Grpc => run_grpc_server(&bind_address, max_body_size).await,
    }
}

/// Run the HTTP REST API server.
async fn run_http_server(
    bind_address: &str,
    max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting HL7 v2 HTTP server on {}", bind_address);

    // Create shutdown signal
    let shutdown = setup_shutdown_signal();

    let server = build_server_from_cli_args(bind_address, max_body_size)?;

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
async fn run_grpc_server(
    bind_address: &str,
    max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting HL7 v2 gRPC server on {}", bind_address);

    let shutdown = setup_shutdown_signal();
    let server = build_server_from_cli_args(bind_address, max_body_size)?;

    info!("Server configuration:");
    info!("  Bind address: {}", bind_address);
    info!("  Transport: gRPC");
    info!("  RPCs:");
    info!("    Parse, ParseStream, Validate, ProfileLint, ProfileExplain, ProfileTest");
    info!("    ValidateRedacted (redaction receipt and configured quarantine output)");
    info!("    CreateEvidenceBundle, ReplayEvidenceBundle");
    info!("    CorpusSummarize, CorpusFingerprint, CorpusDiff");
    info!("    GenerateAck, Normalize, HealthCheck");
    info!("");
    info!("Press Ctrl+C to shutdown gracefully");

    tokio::select! {
        result = server.serve_grpc() => {
            match result {
                Ok(()) => info!("gRPC server shutdown normally"),
                Err(e) => {
                    error!("gRPC server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown => {
            info!("Shutdown signal received, stopping gRPC server...");
        }
    }

    info!("gRPC server stopped");
    Ok(())
}

fn build_server_from_cli_args(
    bind_address: &str,
    max_body_size: usize,
) -> Result<Server, Box<dyn std::error::Error>> {
    let config = server_config_from_cli_args(bind_address, max_body_size)?;
    Ok(Server::new(config))
}

fn server_config_from_cli_args(
    bind_address: &str,
    max_body_size: usize,
) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let config_path = std::env::var_os("HL7V2_CONFIG").map(std::path::PathBuf::from);
    server_config_from_cli_sources(
        CliServerConfigSources {
            config_path: config_path.as_deref(),
            api_key: std::env::var("HL7V2_API_KEY").ok(),
            cors_allowed_origins: std::env::var("HL7V2_CORS_ALLOWED_ORIGINS").ok(),
            profile_paths: std::env::var_os("HL7V2_PROFILE_PATHS"),
            bundle_output_root: std::env::var_os("HL7V2_BUNDLE_OUTPUT_ROOT"),
        },
        bind_address,
        max_body_size,
    )
}

struct CliServerConfigSources<'a> {
    config_path: Option<&'a Path>,
    api_key: Option<String>,
    cors_allowed_origins: Option<String>,
    profile_paths: Option<OsString>,
    bundle_output_root: Option<OsString>,
}

fn server_config_from_cli_sources(
    sources: CliServerConfigSources<'_>,
    bind_address: &str,
    max_body_size: usize,
) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let config = ServerConfig::from_sources(
        sources.config_path,
        None,
        sources.api_key,
        sources.cors_allowed_origins,
        sources.profile_paths,
        sources.bundle_output_root,
    )?;
    Ok(apply_cli_server_args(config, bind_address, max_body_size))
}

fn apply_cli_server_args(
    mut config: ServerConfig,
    bind_address: &str,
    max_body_size: usize,
) -> ServerConfig {
    config.bind_address = bind_address.to_string();
    config.max_body_size = max_body_size;
    config
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
    async fn test_grpc_invalid_bind_address_fails_before_serving()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = run_grpc_server("not-a-bind-address", 1024).await;
        let Err(error) = result else {
            return Err("invalid bind address should fail before serving".into());
        };
        if error.to_string().contains("Invalid bind address") {
            Ok(())
        } else {
            Err(format!("expected invalid bind address error, got: {error}").into())
        }
    }

    #[test]
    fn test_cli_server_args_preserve_security_and_evidence_config() {
        let config = ServerConfig {
            api_key: Some("grpc-secret".to_string()),
            bundle_output_root: Some(std::path::PathBuf::from("bundle-root")),
            quarantine: hl7v2_server::models::QuarantineConfig {
                enabled: true,
                path: Some(std::path::PathBuf::from("quarantine-root")),
                write_redacted: true,
                write_report: true,
                write_bundle: true,
            },
            ..ServerConfig::default()
        };

        let config = apply_cli_server_args(config, "127.0.0.1:50051", 65_536);

        assert_eq!(config.bind_address, "127.0.0.1:50051");
        assert_eq!(config.max_body_size, 65_536);
        assert_eq!(config.api_key.as_deref(), Some("grpc-secret"));
        assert_eq!(
            config.bundle_output_root,
            Some(std::path::PathBuf::from("bundle-root"))
        );
        assert!(config.quarantine.enabled);
        assert_eq!(
            config.quarantine.path,
            Some(std::path::PathBuf::from("quarantine-root"))
        );
    }

    #[test]
    fn test_cli_server_sources_load_security_and_evidence_config_before_cli_bind_override() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let config_path = dir.path().join("server.toml");
        std::fs::write(
            &config_path,
            r#"
[server]
host = "0.0.0.0"
port = 18080
api_key = "file-secret"
bundle_output_root = "file-bundles"

[quarantine]
enabled = true
path = "file-quarantine"
write_redacted = true
write_report = true
write_bundle = true
"#,
        )
        .expect("config should be written");

        let config = server_config_from_cli_sources(
            CliServerConfigSources {
                config_path: Some(&config_path),
                api_key: Some("env-secret".to_string()),
                cors_allowed_origins: Some("https://example.test".to_string()),
                profile_paths: None,
                bundle_output_root: Some(OsString::from("env-bundles")),
            },
            "127.0.0.1:50051",
            65_536,
        )
        .expect("config sources should load");

        assert_eq!(config.bind_address, "127.0.0.1:50051");
        assert_eq!(config.max_body_size, 65_536);
        assert_eq!(config.api_key.as_deref(), Some("env-secret"));
        assert_eq!(
            config.bundle_output_root,
            Some(std::path::PathBuf::from("env-bundles"))
        );
        assert!(config.quarantine.enabled);
        assert_eq!(
            config.quarantine.path,
            Some(std::path::PathBuf::from("file-quarantine"))
        );
        assert_eq!(
            config.cors_allowed_origins,
            hl7v2_server::server::CorsAllowedOrigins::list(["https://example.test"])
        );
    }
}
