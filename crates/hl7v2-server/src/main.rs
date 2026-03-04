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

    tracing::info!("Starting HL7v2 HTTP server");
    tracing::info!("Bind address: {}", bind_address);

    // Get API key from environment if set
    let api_key = std::env::var("HL7V2_API_KEY").ok();
    if api_key.is_some() {
        tracing::info!("API key authentication enabled");
    } else {
        tracing::warn!("API key authentication disabled (public access enabled)");
    }

    // Create and run server
    let server = Server::builder()
        .bind(bind_address)
        .api_key(api_key)
        .build();

    server.serve().await?;

    Ok(())
}
