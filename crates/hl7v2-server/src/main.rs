//! HL7v2 HTTP/REST API server binary.

use hl7v2_server::{Server, ServerConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = parse_args(std::env::args().skip(1))
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    if command == ServerCommand::Help {
        println!("{}", usage());
        return Ok(());
    }

    let config = ServerConfig::from_env()?;
    if command == ServerCommand::PrintConfig {
        println!(
            "{}",
            serde_json::to_string_pretty(&config.to_public_config())?
        );
        return Ok(());
    }

    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hl7v2_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting HL7v2 HTTP server");
    tracing::info!("Bind address: {}", config.bind_address);
    if config.api_key.is_some() {
        tracing::info!("API key authentication enabled");
    } else {
        tracing::warn!("API key authentication disabled (public access enabled)");
    }
    tracing::info!("CORS allowed origins: {:?}", config.cors_allowed_origins);

    // Create and run server
    let server = Server::new(config);

    server.serve().await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerCommand {
    Serve,
    PrintConfig,
    Help,
}

fn parse_args<I, S>(args: I) -> Result<ServerCommand, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut command = ServerCommand::Serve;

    for arg in args {
        match arg.as_ref() {
            "--print-config" => command = ServerCommand::PrintConfig,
            "-h" | "--help" => command = ServerCommand::Help,
            unknown => {
                return Err(format!(
                    "unknown argument '{unknown}'. Run hl7v2-server --help for usage."
                ));
            }
        }
    }

    Ok(command)
}

fn usage() -> &'static str {
    "Usage: hl7v2-server [--print-config]\n\nOptions:\n  --print-config  Print sanitized effective server configuration as JSON and exit\n  -h, --help      Print help\n\nEnvironment:\n  HL7V2_CONFIG                Optional TOML/YAML config file with [server] and [ack] settings\n  BIND_ADDRESS                Override bind address, for example 0.0.0.0:8080\n  HL7V2_API_KEY               API key for protected /hl7/* routes\n  HL7V2_CORS_ALLOWED_ORIGINS  Comma-separated CORS origins, or * for any\n  HL7V2_PROFILE_PATHS         Profile files that must load before readiness passes\n  HL7V2_BUNDLE_OUTPUT_ROOT    Existing writable directory for server-generated evidence bundles"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_serve() {
        assert_eq!(
            parse_args(std::iter::empty::<&str>()),
            Ok(ServerCommand::Serve)
        );
    }

    #[test]
    fn parse_args_accepts_print_config() {
        assert_eq!(
            parse_args(["--print-config"]),
            Ok(ServerCommand::PrintConfig)
        );
    }

    #[test]
    fn parse_args_accepts_help() {
        assert_eq!(parse_args(["--help"]), Ok(ServerCommand::Help));
        assert_eq!(parse_args(["-h"]), Ok(ServerCommand::Help));
    }

    #[test]
    fn parse_args_rejects_unknown_arguments() {
        assert!(matches!(
            parse_args(["--unknown"]),
            Err(error) if error.contains("unknown argument")
        ));
    }
}
