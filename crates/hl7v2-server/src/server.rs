//! HTTP server implementation.

use metrics_exporter_prometheus::PrometheusHandle;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::info;

use crate::Result;
use crate::routes::build_router;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Server start time for uptime calculation
    pub start_time: Instant,
    /// Prometheus metrics handle
    pub metrics_handle: Arc<PrometheusHandle>,
    /// Optional API key for authentication
    pub api_key: Option<String>,
    /// CORS origin policy for browser clients
    pub cors_allowed_origins: CorsAllowedOrigins,
}

/// CORS origin policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CorsAllowedOrigins {
    /// Allow any origin.
    #[default]
    Any,
    /// Allow only the listed origins.
    List(Vec<String>),
}

impl CorsAllowedOrigins {
    /// Build an allow-any origin policy.
    pub fn any() -> Self {
        Self::Any
    }

    /// Build a list-based origin policy.
    pub fn list<I, S>(origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let origins = origins
            .into_iter()
            .map(Into::into)
            .map(|origin: String| origin.trim().to_string())
            .filter(|origin| !origin.is_empty())
            .collect::<Vec<_>>();

        if origins.is_empty() || origins.iter().any(|origin| origin == "*") {
            Self::Any
        } else {
            Self::List(origins)
        }
    }

    /// Parse a comma-separated origin list. Empty values and `*` mean any origin.
    pub fn from_csv(value: &str) -> Self {
        Self::list(value.split(','))
    }
}

/// HTTP server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "0.0.0.0:8080")
    pub bind_address: String,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Optional API key for authentication
    pub api_key: Option<String>,
    /// CORS origin policy
    pub cors_allowed_origins: CorsAllowedOrigins,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".to_string(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            api_key: None,
            cors_allowed_origins: CorsAllowedOrigins::default(),
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
            cors_allowed_origins: config.cors_allowed_origins.clone(),
        });

        Self { config, state }
    }

    /// Create a server builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Run the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the bind address is invalid, the TCP listener cannot
    /// be created, or the Axum server exits with an error.
    pub async fn serve(self) -> Result<()> {
        // Parse bind address
        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| crate::Error::Config(format!("Invalid bind address: {e}")))?;

        // Create TCP listener
        let listener = TcpListener::bind(&addr).await?;
        info!("Server listening on {}", addr);

        // Build router
        let app = build_router(self.state);

        // Serve
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await?;

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

    /// Set the API key for authentication
    pub fn api_key(mut self, api_key: Option<String>) -> Self {
        self.config.api_key = api_key;
        self
    }

    /// Set the CORS allowed origins.
    pub fn cors_allowed_origins(mut self, origins: CorsAllowedOrigins) -> Self {
        self.config.cors_allowed_origins = origins;
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
            .build();

        assert_eq!(server.config.bind_address, "127.0.0.1:8080");
        assert_eq!(server.config.max_body_size, 1024 * 1024);
    }

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:8080");
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
        assert_eq!(config.cors_allowed_origins, CorsAllowedOrigins::Any);
    }

    #[test]
    fn test_cors_allowed_origins_from_csv() {
        assert_eq!(CorsAllowedOrigins::from_csv("*"), CorsAllowedOrigins::Any);
        assert_eq!(CorsAllowedOrigins::from_csv(""), CorsAllowedOrigins::Any);
        assert_eq!(
            CorsAllowedOrigins::from_csv("https://app.example, https://ops.example"),
            CorsAllowedOrigins::List(vec![
                "https://app.example".to_string(),
                "https://ops.example".to_string()
            ])
        );
    }
}
