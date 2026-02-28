//! End-to-end tests for the hl7v2-rs workspace.
//!
//! These tests validate the entire system working together, including:
//! - Full message processing pipeline (parse → validate → generate ACK → write)
//! - Network communication with MLLP framing
//! - CLI integration
//! - Server HTTP API

pub mod cli_integration_tests;
pub mod message_pipeline_tests;
pub mod network_tests;
pub mod server_api_tests;

/// Common test utilities for E2E tests
pub mod common {
    use std::net::SocketAddr;
    use std::sync::Once;
    use tokio::net::TcpListener;

    static INIT: Once = Once::new();

    /// Initialize tracing for tests
    pub fn init_tracing() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
                )
                .with_test_writer()
                .init();
        });
    }

    /// Find an available port on localhost
    pub async fn find_available_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to random port");
        let addr = listener.local_addr().expect("Failed to get local address");
        addr.port()
    }

    /// Get a test server address with a random port
    pub async fn test_server_addr() -> SocketAddr {
        let port = find_available_port().await;
        format!("127.0.0.1:{}", port)
            .parse()
            .expect("Failed to parse address")
    }

    /// Wait for a server to be ready
    pub async fn wait_for_server(addr: SocketAddr, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        false
    }
}
