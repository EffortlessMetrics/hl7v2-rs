//! Configuration file loading for hl7v2-cli and hl7v2-server.
//!
//! This module provides configuration file parsing with support for:
//! - TOML and YAML formats
//! - Layered configuration (CLI > Env > Config file > Defaults)
//! - Environment variable overrides

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Server settings
    #[serde(default)]
    pub server: ServerConfig,
    /// CLI settings
    #[serde(default)]
    pub cli: CliConfig,
    /// Logging settings
    #[serde(default)]
    pub logging: LogConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// API key for authentication
    pub api_key: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            api_key: None,
        }
    }
}

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Default HL7 version
    pub default_version: String,
    /// Default output format (text, json)
    pub output_format: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            default_version: "2.5.1".to_string(),
            output_format: "text".to_string(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,
    /// Whether to log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_path: Option<PathBuf>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_path: None,
        }
    }
}

/// Load configuration from a file
pub fn load_config(path: impl AsRef<Path>) -> Result<Config, Box<dyn std::error::Error>> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref)?;
    let config: Config = if path_ref.extension().and_then(|s| s.to_str()) == Some("yaml") {
        serde_yaml::from_str(&content)?
    } else {
        toml::from_str(&content)?
    };

    Ok(config)
}

/// Apply environment variable overrides to configuration
pub fn apply_env_overrides(config: &mut Config) {
    if let Ok(host) = std::env::var("HL7_HOST") {
        config.server.host = host;
    }
    if let Ok(port_str) = std::env::var("HL7_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            config.server.port = port;
        }
    }
    if let Ok(api_key) = std::env::var("HL7_API_KEY") {
        config.server.api_key = Some(api_key);
    }
    if let Ok(log_level) = std::env::var("HL7_LOG_LEVEL") {
        config.logging.level = log_level;
    }
}
