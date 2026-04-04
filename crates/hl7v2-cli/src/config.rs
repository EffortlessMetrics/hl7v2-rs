//! Configuration file loading for hl7v2-cli and hl7v2-server.
//!
//! This module provides configuration file parsing with support for:
//! - TOML and YAML formats
//! - Layered configuration (CLI > Env > Config file > Defaults)
//! - Config validation and error handling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

/// The main configuration structure for HL7v2 applications.
///
/// This struct defines all configurable settings organized by section.
/// It supports deserialization from TOML and YAML config files.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    /// Server settings (host, port, timeouts, etc.)
    #[serde(default)]
    pub server: ServerConfig,

    /// HL7 message processing settings
    #[serde(default)]
    pub hl7: Hl7Config,

    /// Security settings (TLS, API keys, CORS)
    #[serde(default)]
    pub security: SecurityConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Performance settings
    #[serde(default)]
    pub performance: PerformanceConfig,

    /// Feature flags
    #[serde(default)]
    pub features: FeatureConfig,

    /// Unknown sections are collected here for forward compatibility
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Server configuration section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    /// Host address to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
            max_body_size: default_max_body_size(),
            request_timeout: default_request_timeout(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_body_size() -> usize {
    10485760 // 10MB
}

fn default_request_timeout() -> u64 {
    30
}

/// HL7 message configuration section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hl7Config {
    /// HL7 version to use
    #[serde(default = "default_hl7_version")]
    pub version: String,

    /// Strict parsing mode
    #[serde(default = "default_strict_parsing")]
    pub strict_parsing: bool,

    /// Default character encoding
    #[serde(default = "default_encoding")]
    pub default_encoding: String,

    /// ACK generation mode: "original" or "enhanced"
    #[serde(default = "default_ack_mode")]
    pub acknowledgment_mode: String,
}

impl Default for Hl7Config {
    fn default() -> Self {
        Self {
            version: default_hl7_version(),
            strict_parsing: default_strict_parsing(),
            default_encoding: default_encoding(),
            acknowledgment_mode: default_ack_mode(),
        }
    }
}

fn default_hl7_version() -> String {
    "2.5.1".to_string()
}

fn default_strict_parsing() -> bool {
    true
}

fn default_encoding() -> String {
    "UTF-8".to_string()
}

fn default_ack_mode() -> String {
    "enhanced".to_string()
}

/// Security configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SecurityConfig {
    /// API key for authentication (optional)
    pub api_key: Option<String>,

    /// Enable TLS/HTTPS
    #[serde(default)]
    pub tls_enabled: bool,

    /// Path to TLS certificate
    pub cert_path: Option<String>,

    /// Path to TLS private key
    pub key_path: Option<String>,

    /// Allowed CORS origins
    #[serde(default)]
    pub cors_origins: Vec<String>,

    /// Rate limit per minute per client
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

fn default_rate_limit() -> u32 {
    100
}

/// Logging configuration section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format: pretty, json, compact
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Log output: stdout, stderr, or file path
    #[serde(default = "default_log_output")]
    pub output: String,

    /// Log file path (when output is "file")
    pub file_path: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: default_log_output(),
            file_path: None,
        }
    }
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

/// Performance configuration section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceConfig {
    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Worker thread count (0 = number of CPUs)
    #[serde(default)]
    pub worker_threads: usize,

    /// Connection keep-alive timeout in seconds
    #[serde(default = "default_keep_alive")]
    pub keep_alive_timeout: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            worker_threads: 0,
            keep_alive_timeout: default_keep_alive(),
        }
    }
}

fn default_max_connections() -> usize {
    1000
}

fn default_keep_alive() -> u64 {
    60
}

/// Feature flags configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FeatureConfig {
    /// Enable gRPC server
    #[serde(default)]
    pub grpc_enabled: bool,

    /// Enable WebSocket support
    #[serde(default)]
    pub websockets_enabled: bool,

    /// Enable metrics endpoint
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
}

fn default_metrics_enabled() -> bool {
    true
}

/// Error type for configuration loading failures
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// IO error when reading config file
    #[error("Failed to read config file '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// TOML parsing error
    #[error("Failed to parse config file '{path}': {message}")]
    TomlParse { path: String, message: String },

    /// YAML parsing error
    #[error("Failed to parse config file '{path}': {message}")]
    YamlParse { path: String, message: String },

    /// Unsupported config file format
    #[error("Unsupported config file format: {path}. Use .toml, .yaml, or .yml extension")]
    UnsupportedFormat { path: String },

    /// Config file not found
    #[error("Config file not found: {path}")]
    NotFound { path: String },

    /// Config file is a directory
    #[error("Config path is a directory, not a file: {path}")]
    IsDirectory { path: String },
}

/// Load configuration from a file.
///
/// Supports TOML (.toml) and YAML (.yaml, .yml) formats.
/// The format is detected from the file extension.
///
/// # Arguments
///
/// * `path` - Path to the configuration file
///
/// # Returns
///
/// Returns the loaded `Config` or a `ConfigError`.
///
/// # Example
///
/// ```rust,no_run
/// use hl7v2_cli::config::load_config;
///
/// let config = load_config("/etc/hl7v2/config.toml").unwrap();
/// println!("Server port: {}", config.server.port);
/// ```
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy().to_string();

    // Check if path exists
    if !path.exists() {
        return Err(ConfigError::NotFound { path: path_str });
    }

    // Check if path is a directory
    if path.is_dir() {
        return Err(ConfigError::IsDirectory { path: path_str });
    }

    // Read file content
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path_str.clone(),
        source: e,
    })?;

    // Parse based on extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "toml" => {
            let config: Config = toml::from_str(&content).map_err(|e| ConfigError::TomlParse {
                path: path_str,
                message: e.to_string(),
            })?;
            Ok(config)
        }
        "yaml" | "yml" => {
            let config: Config =
                serde_yaml::from_str(&content).map_err(|e| ConfigError::YamlParse {
                    path: path_str,
                    message: e.to_string(),
                })?;
            Ok(config)
        }
        _ => Err(ConfigError::UnsupportedFormat { path: path_str }),
    }
}

/// Resolve a config file path, expanding `~` to the home directory.
///
/// # Arguments
///
/// * `path` - The path to resolve
///
/// # Returns
///
/// The resolved `PathBuf`
pub fn resolve_config_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();

    // Handle tilde expansion
    if let Some(path_str) = path.to_str()
        && let Some(rest) = path_str.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }

    path.to_path_buf()
}

/// Load configuration with environment variable overrides.
///
/// This loads the config file and then applies environment variable overrides
/// following the precedence: Env vars > Config file > Defaults
///
/// # Arguments
///
/// * `path` - Path to the configuration file
///
/// # Returns
///
/// Returns the loaded and merged `Config` or a `ConfigError`.
pub fn load_config_with_env<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
    let mut config = load_config(path)?;

    // Apply environment variable overrides
    apply_env_overrides(&mut config);

    Ok(config)
}

/// Apply environment variable overrides to a config.
///
/// Environment variables take precedence over config file values.
pub fn apply_env_overrides(config: &mut Config) {
    // Server settings
    if let Ok(host) = std::env::var("HL7V2_SERVER_HOST") {
        config.server.host = host;
    }
    if let Ok(port) = std::env::var("HL7V2_SERVER_PORT") {
        if let Ok(port_num) = port.parse() {
            config.server.port = port_num;
        }
    }
    if let Ok(log_level) = std::env::var("HL7V2_LOG_LEVEL") {
        config.server.log_level = log_level.clone();
        config.logging.level = log_level;
    }

    // Security settings
    if let Ok(api_key) = std::env::var("HL7V2_API_KEY") {
        config.security.api_key = Some(api_key);
    }
    if let Ok(tls_enabled) = std::env::var("HL7V2_TLS_ENABLED") {
        config.security.tls_enabled = tls_enabled == "true" || tls_enabled == "1";
    }

    // Legacy BIND_ADDRESS support (overrides host:port)
    if let Ok(bind) = std::env::var("BIND_ADDRESS")
        && let Some((host, port)) = bind.rsplit_once(':')
    {
        config.server.host = host.to_string();
        if let Ok(port_num) = port.parse() {
            config.server.port = port_num;
        }
    }
}

/// Merge two configurations, with `other` taking precedence.
///
/// This is used to merge CLI arguments over config file values.
pub fn merge_config(base: Config, override_config: Config) -> Config {
    Config {
        server: ServerConfig {
            host: if override_config.server.host != default_host() {
                override_config.server.host
            } else {
                base.server.host
            },
            port: if override_config.server.port != default_port() {
                override_config.server.port
            } else {
                base.server.port
            },
            log_level: if override_config.server.log_level != default_log_level() {
                override_config.server.log_level
            } else {
                base.server.log_level
            },
            max_body_size: if override_config.server.max_body_size != default_max_body_size() {
                override_config.server.max_body_size
            } else {
                base.server.max_body_size
            },
            request_timeout: if override_config.server.request_timeout != default_request_timeout()
            {
                override_config.server.request_timeout
            } else {
                base.server.request_timeout
            },
        },
        hl7: Hl7Config {
            version: if override_config.hl7.version != default_hl7_version() {
                override_config.hl7.version
            } else {
                base.hl7.version
            },
            strict_parsing: override_config.hl7.strict_parsing || base.hl7.strict_parsing,
            default_encoding: if override_config.hl7.default_encoding != default_encoding() {
                override_config.hl7.default_encoding
            } else {
                base.hl7.default_encoding
            },
            acknowledgment_mode: if override_config.hl7.acknowledgment_mode != default_ack_mode() {
                override_config.hl7.acknowledgment_mode
            } else {
                base.hl7.acknowledgment_mode
            },
        },
        security: SecurityConfig {
            api_key: override_config.security.api_key.or(base.security.api_key),
            tls_enabled: override_config.security.tls_enabled || base.security.tls_enabled,
            cert_path: override_config
                .security
                .cert_path
                .or(base.security.cert_path),
            key_path: override_config.security.key_path.or(base.security.key_path),
            cors_origins: if !override_config.security.cors_origins.is_empty() {
                override_config.security.cors_origins
            } else {
                base.security.cors_origins
            },
            rate_limit: if override_config.security.rate_limit != default_rate_limit() {
                override_config.security.rate_limit
            } else {
                base.security.rate_limit
            },
        },
        logging: LoggingConfig {
            level: if override_config.logging.level != default_log_level() {
                override_config.logging.level
            } else {
                base.logging.level
            },
            format: if override_config.logging.format != default_log_format() {
                override_config.logging.format
            } else {
                base.logging.format
            },
            output: if override_config.logging.output != default_log_output() {
                override_config.logging.output
            } else {
                base.logging.output
            },
            file_path: override_config.logging.file_path.or(base.logging.file_path),
        },
        performance: PerformanceConfig {
            max_connections: if override_config.performance.max_connections
                != default_max_connections()
            {
                override_config.performance.max_connections
            } else {
                base.performance.max_connections
            },
            worker_threads: if override_config.performance.worker_threads != 0 {
                override_config.performance.worker_threads
            } else {
                base.performance.worker_threads
            },
            keep_alive_timeout: if override_config.performance.keep_alive_timeout
                != default_keep_alive()
            {
                override_config.performance.keep_alive_timeout
            } else {
                base.performance.keep_alive_timeout
            },
        },
        features: FeatureConfig {
            grpc_enabled: override_config.features.grpc_enabled || base.features.grpc_enabled,
            websockets_enabled: override_config.features.websockets_enabled
                || base.features.websockets_enabled,
            metrics_enabled: override_config.features.metrics_enabled,
        },
        extra: base.extra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Create a test TOML config file
    fn create_test_config_toml() -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(
            file,
            r#"
[server]
host = "127.0.0.1"
port = 9999
log_level = "debug"

[hl7]
version = "2.3"
strict_parsing = false

[security]
api_key = "test-api-key"
tls_enabled = true
rate_limit = 50
"#
        )
        .unwrap();
        file
    }

    /// Create a test YAML config file
    fn create_test_config_yaml() -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        write!(
            file,
            r#"
server:
  host: "127.0.0.1"
  port: 8888
  log_level: "warn"

hl7:
  version: "2.4"
  strict_parsing: true
"#
        )
        .unwrap();
        file
    }

    #[test]
    fn test_load_toml_config() {
        let file = create_test_config_toml();
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9999);
        assert_eq!(config.server.log_level, "debug");
        assert_eq!(config.hl7.version, "2.3");
        assert!(!config.hl7.strict_parsing);
        assert_eq!(config.security.api_key, Some("test-api-key".to_string()));
        assert!(config.security.tls_enabled);
        assert_eq!(config.security.rate_limit, 50);
    }

    #[test]
    fn test_load_yaml_config() {
        let file = create_test_config_yaml();
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8888);
        assert_eq!(config.server.log_level, "warn");
        assert_eq!(config.hl7.version, "2.4");
        assert!(config.hl7.strict_parsing);
    }

    #[test]
    fn test_load_yml_config() {
        let mut file = NamedTempFile::with_suffix(".yml").unwrap();
        write!(
            file,
            r#"
server:
  port: 7777
"#
        )
        .unwrap();

        let config = load_config(file.path()).unwrap();
        assert_eq!(config.server.port, 7777);
    }

    #[test]
    fn test_load_missing_config() {
        let result = load_config("/nonexistent/path/config.toml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Config file not found"));
    }

    #[test]
    fn test_load_directory() {
        let result = load_config("/tmp");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("directory"));
    }

    #[test]
    fn test_load_invalid_toml() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(file, "[invalid toml").unwrap();

        let result = load_config(file.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to parse"));
    }

    #[test]
    fn test_load_invalid_yaml() {
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        write!(file, "{{invalid yaml: []").unwrap();

        let result = load_config(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_format() {
        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        write!(file, r#"{{"server": {{"port": 8080}}}}"#).unwrap();

        let result = load_config(file.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unsupported config file format"));
    }

    #[test]
    fn test_resolve_config_path_no_tilde() {
        let path = resolve_config_path("/etc/hl7v2/config.toml");
        assert_eq!(path, PathBuf::from("/etc/hl7v2/config.toml"));
    }

    #[test]
    fn test_load_empty_config() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(file, "").unwrap();

        let config = load_config(file.path()).unwrap();
        // Should use all defaults
        assert_eq!(config.server.host, default_host());
        assert_eq!(config.server.port, default_port());
        assert_eq!(config.server.log_level, default_log_level());
    }

    #[test]
    fn test_load_unknown_sections_ignored() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(
            file,
            r#"
[server]
port = 9000

[future_section]
unknown_field = "value"
"#
        )
        .unwrap();

        let config = load_config(file.path()).unwrap();
        assert_eq!(config.server.port, 9000);
        // Unknown section should be in extra
        assert!(config.extra.contains_key("future_section"));
    }

    #[test]
    fn test_load_unknown_fields_ignored() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(
            file,
            r#"
[server]
port = 9000
unknown_field = "value"
"#
        )
        .unwrap();

        let config = load_config(file.path()).unwrap();
        assert_eq!(config.server.port, 9000);
        // Should not fail on unknown fields
    }

    #[test]
    fn test_merge_config() {
        let mut base = Config::default();
        base.server.port = 8080;
        base.server.host = "0.0.0.0".to_string();

        let mut override_config = Config::default();
        override_config.server.port = 9090;
        // host stays at default

        let merged = merge_config(base, override_config);

        // Port should be overridden
        assert_eq!(merged.server.port, 9090);
        // Host should stay from base (override has default)
        assert_eq!(merged.server.host, "0.0.0.0");
    }

    #[test]
    fn test_apply_env_overrides() {
        let mut config = Config::default();

        // Set env vars (unsafe required in Rust 2024 edition)
        unsafe {
            std::env::set_var("HL7V2_SERVER_HOST", "192.168.1.1");
            std::env::set_var("HL7V2_SERVER_PORT", "9090");
            std::env::set_var("HL7V2_LOG_LEVEL", "debug");
        }

        apply_env_overrides(&mut config);

        assert_eq!(config.server.host, "192.168.1.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.log_level, "debug");
        assert_eq!(config.logging.level, "debug");

        // Clean up (unsafe required in Rust 2024 edition)
        unsafe {
            std::env::remove_var("HL7V2_SERVER_HOST");
            std::env::remove_var("HL7V2_SERVER_PORT");
            std::env::remove_var("HL7V2_LOG_LEVEL");
        }
    }

    #[test]
    fn test_legacy_bind_address() {
        let mut config = Config::default();

        unsafe {
            std::env::set_var("BIND_ADDRESS", "10.0.0.1:7070");
        }

        apply_env_overrides(&mut config);

        assert_eq!(config.server.host, "10.0.0.1");
        assert_eq!(config.server.port, 7070);

        unsafe {
            std::env::remove_var("BIND_ADDRESS");
        }
    }
}
