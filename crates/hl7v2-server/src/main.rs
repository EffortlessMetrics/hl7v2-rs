//! HL7v2 HTTP/REST API server binary.

use hl7v2_server::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hl7v2_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get bind address from environment or use default
    let bind_address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // Get API key from environment
    let api_key = std::env::var("HL7V2_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        tracing::warn!("HL7V2_API_KEY environment variable not set or empty. Authentication may fail.");
    }

    tracing::info!("Starting HL7v2 HTTP server");
    tracing::info!("Bind address: {}", bind_address);

    // Create and run server
    let server = Server::builder()
        .bind(bind_address)
        .api_key(api_key)
        .build();

    server.serve().await?;

    Ok(())
}
