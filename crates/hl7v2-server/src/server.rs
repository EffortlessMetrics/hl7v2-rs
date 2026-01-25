//! HTTP server implementation.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::info;
use metrics_exporter_prometheus::PrometheusHandle;

use crate::routes::build_router;
use crate::Result;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Server start time for uptime calculation
    pub start_time: Instant,
    /// Prometheus metrics handle
    pub metrics_handle: Arc<PrometheusHandle>,
    /// API key for authentication
    pub api_key: Option<String>,
}

/// HTTP server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "0.0.0.0:8080")
    pub bind_address: String,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// API key for authentication
    pub api_key: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".to_string(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            // Load from environment variable by default
            api_key: std::env::var("HL7V2_API_KEY").ok(),
        }
    }
}

/// HTTP server
pub struct Server {
    config: ServerConfig,
    state: Arc<AppState>,
}

impl Server {
    /// Create a new server with the given configuration
    pub fn new(config: ServerConfig) -> Self {
        // Initialize Prometheus metrics recorder
        let metrics_handle = crate::metrics::init_metrics_recorder();

        let state = Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: config.api_key.clone(),
        });

        Self { config, state }
    }

    /// Create a server builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Run the server
    pub async fn serve(self) -> Result<()> {
        // Parse bind address
        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| crate::Error::Config(format!("Invalid bind address: {}", e)))?;

        // Create TCP listener
        let listener = TcpListener::bind(&addr).await?;
        info!("Server listening on {}", addr);

        // Build router
        let app = build_router(self.state);

        // Serve
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Server builder for fluent configuration
pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
        }
    }

    /// Set the bind address
    pub fn bind(mut self, address: impl Into<String>) -> Self {
        self.config.bind_address = address.into();
        self
    }

    /// Set the maximum request body size
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.config.max_body_size = size;
        self
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// Disable API key authentication (for testing)
    pub fn no_auth(mut self) -> Self {
        self.config.api_key = None;
        self
    }

    /// Build the server
    pub fn build(self) -> Server {
        Server::new(self.config)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_builder() {
        let server = Server::builder()
            .bind("127.0.0.1:8080")
            .max_body_size(1024 * 1024)
            .api_key("secret")
            .build();

        assert_eq!(server.config.bind_address, "127.0.0.1:8080");
        assert_eq!(server.config.max_body_size, 1024 * 1024);
        assert_eq!(server.config.api_key, Some("secret".to_string()));
    }

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:8080");
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
        // api_key depends on env var, so we don't assert it here
    }
}
