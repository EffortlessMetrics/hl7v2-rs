//! Configuration file loading for hl7v2-cli and hl7v2-server.
//!
//! This module provides configuration file parsing with support for:
//! - TOML and YAML formats
//! - Layered configuration (CLI > Env > Config file > Defaults)
//! - Environment variable overrides

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "configuration loader is staged for CLI config integration"
    )
)]
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
#[expect(
    dead_code,
    reason = "configuration loader is staged for CLI config integration"
)]
pub fn apply_env_overrides(config: &mut Config) {
    if let Ok(host) = std::env::var("HL7_HOST") {
        config.server.host = host;
    }
    if let Ok(port_str) = std::env::var("HL7_PORT")
        && let Ok(port) = port_str.parse::<u16>()
    {
        config.server.port = port;
    }
    if let Ok(api_key) = std::env::var("HL7_API_KEY") {
        config.server.api_key = Some(api_key);
    }
    if let Ok(log_level) = std::env::var("HL7_LOG_LEVEL") {
        config.logging.level = log_level;
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "legacy config tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]
mod tests {
    use super::{Config, load_config};
    use std::fs;

    #[test]
    fn config_example_matches_loader_shape() {
        let config: Config = toml::from_str(include_str!("../../../config.example.toml"))
            .expect("config.example.toml should match Config");

        assert_example_config(config);
    }

    #[test]
    fn load_config_reads_toml_file() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("hl7v2.toml");
        fs::write(&path, include_str!("../../../config.example.toml"))
            .expect("config fixture should be written");

        let config = load_config(&path).expect("TOML config should load");

        assert_example_config(config);
    }

    #[test]
    fn load_config_reads_yaml_file() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("hl7v2.yaml");
        fs::write(
            &path,
            "server:\n  host: 0.0.0.0\n  port: 8080\ncli:\n  default_version: 2.5.1\n  output_format: text\nlogging:\n  level: info\n  log_to_file: false\n",
        )
        .expect("config fixture should be written");

        let config = load_config(&path).expect("YAML config should load");

        assert_example_config(config);
    }

    fn assert_example_config(config: Config) {
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.cli.default_version, "2.5.1");
        assert_eq!(config.cli.output_format, "text");
        assert_eq!(config.logging.level, "info");
        assert!(!config.logging.log_to_file);
    }
}
