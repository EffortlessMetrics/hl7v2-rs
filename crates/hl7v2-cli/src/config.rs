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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn apply_env_overrides(config: &mut Config) {
    apply_env_overrides_with(config, |key| std::env::var(key));
}

fn apply_env_overrides_with(
    config: &mut Config,
    mut var: impl FnMut(&str) -> Result<String, std::env::VarError>,
) {
    if let Ok(host) = var("HL7_HOST") {
        config.server.host = host;
    }
    if let Ok(port_str) = var("HL7_PORT")
        && let Ok(port) = port_str.parse::<u16>()
    {
        config.server.port = port;
    }
    if let Ok(api_key) = var("HL7_API_KEY") {
        config.server.api_key = Some(api_key);
    }
    if let Ok(log_level) = var("HL7_LOG_LEVEL") {
        config.logging.level = log_level;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CliConfig, Config, LogConfig, ServerConfig, apply_env_overrides, apply_env_overrides_with,
        load_config,
    };
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

    #[test]
    fn config_default_has_expected_subsection_values() {
        let config = Config::default();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert!(config.server.api_key.is_none());

        assert_eq!(config.cli.default_version, "2.5.1");
        assert_eq!(config.cli.output_format, "text");

        assert_eq!(config.logging.level, "info");
        assert!(!config.logging.log_to_file);
        assert!(config.logging.log_path.is_none());
    }

    #[test]
    fn server_config_default_matches_documented_defaults() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 8080);
        assert!(server.api_key.is_none());
    }

    #[test]
    fn cli_config_default_matches_documented_defaults() {
        let cli = CliConfig::default();
        assert_eq!(cli.default_version, "2.5.1");
        assert_eq!(cli.output_format, "text");
    }

    #[test]
    fn log_config_default_matches_documented_defaults() {
        let logging = LogConfig::default();
        assert_eq!(logging.level, "info");
        assert!(!logging.log_to_file);
        assert!(logging.log_path.is_none());
    }

    #[test]
    fn load_config_returns_err_for_missing_path() {
        let _err = load_config("/no/such/path/definitely-missing.toml")
            .expect_err("missing path should fail to load");
    }

    #[test]
    fn load_config_returns_err_for_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("invalid.toml");
        fs::write(&path, "@@ this is not valid toml @@@\n!!!!!")
            .expect("invalid TOML fixture should be written");

        let _err = load_config(&path).expect_err("invalid TOML should fail to load");
    }

    #[test]
    fn load_config_returns_err_for_invalid_yaml() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("invalid.yaml");
        fs::write(
            &path,
            "server:\n  host: [unterminated\n  port: not-a-number",
        )
        .expect("invalid YAML fixture should be written");

        let _err = load_config(&path).expect_err("invalid YAML should fail to load");
    }

    #[test]
    fn load_config_empty_toml_yields_default_config() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("empty.toml");
        fs::write(&path, "").expect("empty TOML fixture should be written");

        let config = load_config(&path).expect("empty TOML should load as defaults");
        let defaults = Config::default();

        assert_eq!(config.server.host, defaults.server.host);
        assert_eq!(config.server.port, defaults.server.port);
        assert_eq!(config.cli.default_version, defaults.cli.default_version);
        assert_eq!(config.cli.output_format, defaults.cli.output_format);
        assert_eq!(config.logging.level, defaults.logging.level);
        assert_eq!(config.logging.log_to_file, defaults.logging.log_to_file);
    }

    #[test]
    fn apply_env_overrides_sets_known_values() {
        let mut config = Config::default();

        apply_env_overrides_with(&mut config, |key| match key {
            "HL7_HOST" => Ok("0.0.0.0".to_string()),
            "HL7_PORT" => Ok("9090".to_string()),
            "HL7_API_KEY" => Ok("secret".to_string()),
            "HL7_LOG_LEVEL" => Ok("debug".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.api_key.as_deref(), Some("secret"));
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn apply_env_overrides_ignores_absent_or_invalid_values() {
        let mut config = Config::default();
        let port_before = config.server.port;
        let host_before = config.server.host.clone();
        let level_before = config.logging.level.clone();
        apply_env_overrides_with(&mut config, |key| match key {
            "HL7_PORT" => Ok("not-a-port".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert_eq!(config.server.host, host_before);
        assert_eq!(config.server.port, port_before);
        assert!(config.server.api_key.is_none());
        assert_eq!(config.logging.level, level_before);
    }

    #[test]
    fn apply_env_overrides_ambient_path_is_callable() {
        let mut config = Config::default();
        apply_env_overrides(&mut config);
    }
}
