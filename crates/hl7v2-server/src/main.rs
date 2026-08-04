//! HL7v2 HTTP/REST API server binary.

use hl7v2_server::{Server, ServerConfig};
use std::net::SocketAddr;
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

    init_tracing();

    tracing::info!("Starting HL7v2 HTTP server");
    tracing::info!("Bind address: {}", config.bind_address);
    if config.api_key.is_some() {
        tracing::info!("API key authentication enabled");
    } else {
        tracing::warn!(
            bind_address = %config.bind_address,
            "API key authentication is disabled; /hl7 routes are publicly accessible"
        );
    }
    log_bind_security_warning(&config.bind_address);
    tracing::info!("CORS allowed origins: {:?}", config.cors_allowed_origins);

    // Create and run server
    let server = Server::new(config)?;

    server.serve().await?;

    Ok(())
}

fn log_bind_security_warning(bind_address: &str) {
    if bind_address_is_unspecified(bind_address) {
        tracing::warn!(
            bind_address = %bind_address,
            "server is bound to all network interfaces; restrict network exposure and terminate TLS at a trusted proxy"
        );
    }
}

fn bind_address_is_unspecified(bind_address: &str) -> bool {
    bind_address
        .parse::<SocketAddr>()
        .map(|address| address.ip().is_unspecified())
        .unwrap_or(false)
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
    "Usage: hl7v2-server [--print-config]\n\nOptions:\n  --print-config  Print sanitized effective server configuration as JSON and exit\n  -h, --help      Print help\n\nEnvironment:\n  HL7V2_CONFIG                Optional TOML/YAML config file with [server], [ack], and [quarantine] settings\n  BIND_ADDRESS                Override bind address, for example 0.0.0.0:8080\n  HL7V2_API_KEY               API key for protected /hl7/* routes\n  HL7V2_CORS_ALLOWED_ORIGINS  Comma-separated CORS origins, or * for any\n  HL7V2_MAX_MESSAGE_SIZE      Maximum decoded HL7 message size in bytes (default: 50 MiB)\n  HL7V2_PROFILE_PATHS         Profile files that must load before readiness passes\n  HL7V2_BUNDLE_OUTPUT_ROOT    Existing writable directory for server-generated evidence bundles\n  RUST_LOG                    tracing filter, for example hl7v2_server=info,tower_http=debug\n  RUST_LOG_FORMAT             set to json for JSON logs; any other value uses text logs"
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "hl7v2_server=info,tower_http=debug".into());
    let subscriber = tracing_subscriber::registry().with(env_filter);

    match log_format_from_env() {
        LogFormat::Json => subscriber
            .with(tracing_subscriber::fmt::layer().json())
            .init(),
        LogFormat::Text => subscriber.with(tracing_subscriber::fmt::layer()).init(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat {
    Text,
    Json,
}

fn log_format_from_env() -> LogFormat {
    std::env::var("RUST_LOG_FORMAT")
        .ok()
        .and_then(|value| parse_log_format(&value))
        .unwrap_or(LogFormat::Text)
}

fn parse_log_format(value: &str) -> Option<LogFormat> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "text" | "pretty" => Some(LogFormat::Text),
        "json" => Some(LogFormat::Json),
        _ => None,
    }
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

    #[test]
    fn parse_log_format_accepts_json_and_text_values() {
        assert_eq!(parse_log_format("json"), Some(LogFormat::Json));
        assert_eq!(parse_log_format(" JSON "), Some(LogFormat::Json));
        assert_eq!(parse_log_format("text"), Some(LogFormat::Text));
        assert_eq!(parse_log_format("pretty"), Some(LogFormat::Text));
        assert_eq!(parse_log_format(""), Some(LogFormat::Text));
        assert_eq!(parse_log_format("xml"), None);
    }

    #[test]
    fn bind_security_warning_only_matches_unspecified_addresses() {
        assert!(bind_address_is_unspecified("0.0.0.0:8080"));
        assert!(bind_address_is_unspecified("[::]:8080"));
        assert!(!bind_address_is_unspecified("127.0.0.1:8080"));
        assert!(!bind_address_is_unspecified("not-an-address"));
    }
}
