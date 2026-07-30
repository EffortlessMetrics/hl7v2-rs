//! HTTP server implementation.

use crate::models::{
    AckPolicyConfig, PublicQuarantineConfig, QuarantineConfig, ReadinessCheck, ReadyResponse,
};
use crate::{
    Result,
    grpc::{Hl7ServiceImpl, proto::hl7_service_server::Hl7ServiceServer},
    routes::build_router_with_body_limit,
};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server as TonicServer;
use tracing::info;

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
    /// Startup readiness checks.
    pub readiness_checks: Vec<ReadinessCheck>,
    /// Configured root for server-generated evidence bundles.
    pub bundle_output_root: Option<PathBuf>,
    /// Policy used by the policy-driven ACK endpoint.
    pub ack_policy: AckPolicyConfig,
    /// Quarantine output policy for failed redacted validation.
    pub quarantine: QuarantineConfig,
}

impl AppState {
    /// Build the current readiness response.
    pub fn ready_response(&self) -> ReadyResponse {
        ReadyResponse::from_checks(self.readiness_checks.clone())
    }
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
    /// Configured profile files that must load before the server is ready.
    pub profile_paths: Vec<String>,
    /// Optional config file source used to build this configuration.
    pub config_source: Option<String>,
    /// Optional filesystem root for server-generated evidence bundles.
    pub bundle_output_root: Option<PathBuf>,
    /// Policy used by the policy-driven ACK endpoint.
    pub ack_policy: AckPolicyConfig,
    /// Quarantine output policy for failed redacted validation.
    pub quarantine: QuarantineConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".to_string(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            api_key: None,
            cors_allowed_origins: CorsAllowedOrigins::default(),
            profile_paths: Vec::new(),
            config_source: None,
            bundle_output_root: None,
            ack_policy: AckPolicyConfig::default(),
            quarantine: QuarantineConfig::default(),
        }
    }
}

/// Sanitized server configuration suitable for printing and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicServerConfig {
    /// Address the server will bind.
    pub bind_address: String,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Whether an API key is configured without exposing the secret.
    pub api_key_configured: bool,
    /// CORS origin policy.
    pub cors_allowed_origins: PublicCorsAllowedOrigins,
    /// Profile paths checked by readiness.
    pub profile_paths: Vec<String>,
    /// Optional config file source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_source: Option<String>,
    /// Whether server-side evidence bundle output is configured.
    pub bundle_output_root_configured: bool,
    /// Policy used by the policy-driven ACK endpoint.
    pub ack_policy: AckPolicyConfig,
    /// Sanitized quarantine output policy.
    pub quarantine: PublicQuarantineConfig,
}

/// Sanitized CORS origin policy for printed config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicCorsAllowedOrigins {
    /// Policy mode: `any` or `list`.
    pub mode: String,
    /// Configured origins when mode is `list`.
    pub origins: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    server: FileServerConfig,
    #[serde(default)]
    ack: AckPolicyConfig,
    #[serde(default)]
    quarantine: QuarantineConfig,
}

#[derive(Debug, Default, Deserialize)]
struct FileServerConfig {
    host: Option<String>,
    port: Option<u16>,
    api_key: Option<String>,
    bundle_output_root: Option<PathBuf>,
}

impl ServerConfig {
    /// Build server configuration from supported environment variables.
    pub fn from_env() -> Result<Self> {
        let config_path = std::env::var_os("HL7V2_CONFIG").map(PathBuf::from);
        Self::from_sources(
            config_path.as_deref(),
            std::env::var("BIND_ADDRESS").ok(),
            std::env::var("HL7V2_API_KEY").ok(),
            std::env::var("HL7V2_CORS_ALLOWED_ORIGINS").ok(),
            std::env::var_os("HL7V2_PROFILE_PATHS"),
            std::env::var_os("HL7V2_BUNDLE_OUTPUT_ROOT"),
        )
    }

    /// Build server configuration from explicit sources.
    pub fn from_sources(
        config_path: Option<&Path>,
        bind_address: Option<String>,
        api_key: Option<String>,
        cors_allowed_origins: Option<String>,
        profile_paths: Option<OsString>,
        bundle_output_root: Option<OsString>,
    ) -> Result<Self> {
        let mut config = if let Some(path) = config_path {
            let mut config = Self::from_config_file(path)?;
            config.config_source = Some(path.display().to_string());
            config
        } else {
            Self::default()
        };

        if let Some(bind_address) = bind_address {
            config.bind_address = bind_address;
        }

        if let Some(api_key) = api_key {
            config.api_key = Some(api_key);
        }

        if let Some(cors_allowed_origins) = cors_allowed_origins {
            config.cors_allowed_origins = CorsAllowedOrigins::from_csv(&cors_allowed_origins);
        }

        if let Some(profile_paths) = profile_paths {
            config.profile_paths = split_profile_paths(profile_paths);
        }

        if let Some(bundle_output_root) = bundle_output_root {
            config.bundle_output_root = Some(PathBuf::from(bundle_output_root));
        }

        Ok(config)
    }

    fn from_config_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|error| {
            crate::Error::Config(format!("Failed to read config file: {error}"))
        })?;
        let file_config: FileConfig = match path.extension().and_then(|value| value.to_str()) {
            Some("yaml" | "yml") => serde_yaml::from_str(&content).map_err(|error| {
                crate::Error::Config(format!("Failed to parse YAML config file: {error}"))
            })?,
            _ => toml::from_str(&content).map_err(|error| {
                crate::Error::Config(format!("Failed to parse TOML config file: {error}"))
            })?,
        };

        let mut config = Self::default();
        if let Some(host) = file_config.server.host {
            let port = file_config.server.port.unwrap_or(8080);
            config.bind_address = format!("{host}:{port}");
        } else if let Some(port) = file_config.server.port {
            config.bind_address = format!("0.0.0.0:{port}");
        }
        if let Some(api_key) = file_config.server.api_key {
            config.api_key = Some(api_key);
        }
        if let Some(bundle_output_root) = file_config.server.bundle_output_root {
            config.bundle_output_root = Some(bundle_output_root);
        }
        config.ack_policy = file_config.ack;
        config.quarantine = file_config.quarantine;

        Ok(config)
    }

    /// Return a sanitized public view of this configuration.
    pub fn to_public_config(&self) -> PublicServerConfig {
        PublicServerConfig {
            bind_address: self.bind_address.clone(),
            max_body_size: self.max_body_size,
            api_key_configured: self.api_key.is_some(),
            cors_allowed_origins: public_cors_allowed_origins(&self.cors_allowed_origins),
            profile_paths: self.profile_paths.clone(),
            config_source: self.config_source.clone(),
            bundle_output_root_configured: self.bundle_output_root.is_some(),
            ack_policy: self.ack_policy.clone(),
            quarantine: PublicQuarantineConfig {
                enabled: self.quarantine.enabled,
                path_configured: self.quarantine.path.is_some(),
                write_redacted: self.quarantine.write_redacted,
                write_report: self.quarantine.write_report,
                write_bundle: self.quarantine.write_bundle,
            },
        }
    }

    /// Build startup readiness checks for this configuration.
    pub fn readiness_checks(&self) -> Vec<ReadinessCheck> {
        let mut checks = Vec::new();

        checks.push(ReadinessCheck::pass(
            "config",
            "server configuration parsed",
        ));

        match self.bind_address.parse::<SocketAddr>() {
            Ok(_) => checks.push(ReadinessCheck::pass("bind_address", "bind address parsed")),
            Err(error) => checks.push(ReadinessCheck::fail(
                "bind_address",
                format!("bind address did not parse: {error}"),
            )),
        }

        checks.push(profile_readiness_check(&self.profile_paths));
        checks.push(bundle_output_root_readiness_check(
            self.bundle_output_root.as_deref(),
        ));
        checks.push(quarantine_readiness_check(&self.quarantine));
        checks.push(validation_report_readiness_check());

        checks
    }
}

fn split_profile_paths(paths: OsString) -> Vec<String> {
    std::env::split_paths(&paths)
        .map(|path| path.to_string_lossy().trim().to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

fn public_cors_allowed_origins(origins: &CorsAllowedOrigins) -> PublicCorsAllowedOrigins {
    match origins {
        CorsAllowedOrigins::Any => PublicCorsAllowedOrigins {
            mode: "any".to_string(),
            origins: Vec::new(),
        },
        CorsAllowedOrigins::List(origins) => PublicCorsAllowedOrigins {
            mode: "list".to_string(),
            origins: origins.clone(),
        },
    }
}

fn profile_readiness_check(profile_paths: &[String]) -> ReadinessCheck {
    if profile_paths.is_empty() {
        return ReadinessCheck::pass("configured_profiles", "no configured profiles");
    }

    for (index, path) in profile_paths.iter().enumerate() {
        let profile_number = index.saturating_add(1);
        match fs::read_to_string(path) {
            Ok(content) => {
                if let Err(error) = hl7v2::load_profile_checked(&content) {
                    return ReadinessCheck::fail(
                        "configured_profiles",
                        format!("configured profile {profile_number} did not load: {error}"),
                    );
                }
            }
            Err(error) => {
                return ReadinessCheck::fail(
                    "configured_profiles",
                    format!("configured profile {profile_number} could not be read: {error}"),
                );
            }
        }
    }

    ReadinessCheck::pass(
        "configured_profiles",
        format!("loaded {} configured profile(s)", profile_paths.len()),
    )
}

fn bundle_output_root_readiness_check(bundle_output_root: Option<&Path>) -> ReadinessCheck {
    let Some(root) = bundle_output_root else {
        return ReadinessCheck::pass("bundle_output_root", "server bundle output not configured");
    };

    writable_directory_readiness_check(
        "bundle_output_root",
        root,
        "bundle output root is writable",
        "bundle output root",
    )
}

fn quarantine_readiness_check(quarantine: &QuarantineConfig) -> ReadinessCheck {
    if !quarantine.enabled {
        return ReadinessCheck::pass("quarantine_output", "quarantine output not enabled");
    }

    if !quarantine.write_bundle && !quarantine.write_report && !quarantine.write_redacted {
        return ReadinessCheck::fail(
            "quarantine_output",
            "quarantine output has no artifact writers enabled",
        );
    }

    let Some(root) = quarantine.path.as_deref() else {
        return ReadinessCheck::fail(
            "quarantine_output",
            "quarantine output is enabled but no path is configured",
        );
    };

    writable_directory_readiness_check(
        "quarantine_output",
        root,
        "quarantine output root is writable",
        "quarantine output root",
    )
}

fn writable_directory_readiness_check(
    name: &str,
    root: &Path,
    success_message: &str,
    label: &str,
) -> ReadinessCheck {
    match fs::metadata(root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return ReadinessCheck::fail(name, format!("{label} is not a directory"));
        }
        Err(error) => {
            return ReadinessCheck::fail(name, format!("{label} is not readable: {error}"));
        }
    }

    let probe_path = root.join(format!(
        ".hl7v2-server-ready-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));

    match fs::write(&probe_path, b"ready") {
        Ok(()) => match fs::remove_file(&probe_path) {
            Ok(()) => ReadinessCheck::pass(name, success_message),
            Err(error) => {
                ReadinessCheck::fail(name, format!("{label} probe cleanup failed: {error}"))
            }
        },
        Err(error) => ReadinessCheck::fail(name, format!("{label} is not writable: {error}")),
    }
}

fn validation_report_readiness_check() -> ReadinessCheck {
    const SAMPLE_MESSAGE: &str =
        "MSH|^~\\&|HL7V2|READY|OPS|OPS|202605030101||ADT^A01|READY1|P|2.5\r";
    const SAMPLE_PROFILE: &str = r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
    required: true
    max_uses: 1
"#;

    let result = (|| -> std::result::Result<bool, String> {
        let message = hl7v2::parse(SAMPLE_MESSAGE.as_bytes()).map_err(|error| error.to_string())?;
        let profile =
            hl7v2::load_profile_checked(SAMPLE_PROFILE).map_err(|error| error.to_string())?;
        let issues = hl7v2::validate(&message, &profile);
        let report =
            hl7v2::ValidationReport::from_issues(&message, Some(profile.message_structure), issues);

        Ok(!report.message_type.is_empty() && report.profile.is_some())
    })();

    match result {
        Ok(true) => ReadinessCheck::pass(
            "validation_report",
            "validation report self-check produced typed evidence",
        ),
        Ok(false) => ReadinessCheck::fail(
            "validation_report",
            "validation report self-check returned incomplete evidence",
        ),
        Err(error) => ReadinessCheck::fail(
            "validation_report",
            format!("validation report self-check failed: {error}"),
        ),
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
            readiness_checks: config.readiness_checks(),
            bundle_output_root: config.bundle_output_root.clone(),
            ack_policy: config.ack_policy.clone(),
            quarantine: config.quarantine.clone(),
        });

        Self { config, state }
    }

    /// Create a server builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Run the server
    pub async fn serve(self) -> Result<()> {
        // Parse bind address
        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|e| crate::Error::Config(format!("Invalid bind address: {}", e)))?;

        // Create TCP listener
        let listener = TcpListener::bind(&addr).await?;
        info!("Server listening on {}", addr);

        // Build router
        let app = build_router_with_body_limit(self.state, self.config.max_body_size);

        // Serve
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await?;

        Ok(())
    }

    /// Run the gRPC server.
    pub async fn serve_grpc(self) -> Result<()> {
        let addr: SocketAddr = self
            .config
            .bind_address
            .parse()
            .map_err(|error| crate::Error::Config(format!("Invalid bind address: {error}")))?;

        let listener = TcpListener::bind(&addr).await?;
        self.serve_grpc_with_listener(listener).await
    }

    /// Run the gRPC server using an already-bound listener.
    ///
    /// This is primarily useful for tests that need an OS-assigned port.
    pub async fn serve_grpc_with_listener(self, listener: TcpListener) -> Result<()> {
        let addr = listener.local_addr()?;
        info!("gRPC server listening on {}", addr);

        let max_message_size = self.config.max_body_size;
        let api_key = self.state.api_key.clone();
        let service = Hl7ServiceServer::new(Hl7ServiceImpl::new(self.state))
            .max_decoding_message_size(max_message_size)
            .max_encoding_message_size(max_message_size);
        let service = tonic::service::interceptor::InterceptedService::new(
            service,
            GrpcAuthInterceptor {
                expected_key: api_key,
            },
        );

        TonicServer::builder()
            .add_service(service)
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .map_err(|error| crate::Error::Internal(format!("gRPC server error: {error}")))?;

        Ok(())
    }
}

#[derive(Clone)]
struct GrpcAuthInterceptor {
    expected_key: Option<String>,
}

impl tonic::service::Interceptor for GrpcAuthInterceptor {
    fn call(
        &mut self,
        request: tonic::Request<()>,
    ) -> std::result::Result<tonic::Request<()>, tonic::Status> {
        grpc_auth_interceptor(request, self.expected_key.as_deref())
    }
}

#[expect(
    clippy::result_large_err,
    reason = "tonic interceptor callbacks must return tonic::Status on authentication failures"
)]
fn grpc_auth_interceptor(
    request: tonic::Request<()>,
    expected_key: Option<&str>,
) -> std::result::Result<tonic::Request<()>, tonic::Status> {
    const API_KEY_METADATA: &str = "x-api-key";

    let Some(expected_key) = expected_key else {
        return Ok(request);
    };

    let provided_key = request
        .metadata()
        .get(API_KEY_METADATA)
        .and_then(|value| value.to_str().ok());

    match provided_key {
        Some(key) if key.as_bytes().ct_eq(expected_key.as_bytes()).into() => Ok(request),
        Some(_) => {
            tracing::warn!("Invalid gRPC API key provided");
            Err(tonic::Status::unauthenticated("valid API key required"))
        }
        None => {
            tracing::warn!("No gRPC API key provided");
            Err(tonic::Status::unauthenticated("valid API key required"))
        }
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

    /// Set profile files that must load before readiness passes.
    pub fn profile_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.profile_paths = paths.into_iter().map(Into::into).collect();
        self
    }

    /// Set the filesystem root for server-generated evidence bundles.
    pub fn bundle_output_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.bundle_output_root = Some(path.into());
        self
    }

    /// Set the policy used by the policy-driven ACK endpoint.
    pub fn ack_policy(mut self, policy: AckPolicyConfig) -> Self {
        self.config.ack_policy = policy;
        self
    }

    /// Set the quarantine output policy.
    pub fn quarantine(mut self, quarantine: QuarantineConfig) -> Self {
        self.config.quarantine = quarantine;
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
        assert!(config.profile_paths.is_empty());
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

    #[test]
    fn server_config_loads_config_file_and_env_style_overrides() {
        let path = temp_file_path("server-config.toml");
        fs::write(
            &path,
            r#"
[server]
host = "127.0.0.1"
port = 18080
api_key = "file-secret"

[ack]
mode = "enhanced"
accept_on = "valid"
reject_on = ["validation_error"]
include_error_text = false

[quarantine]
enabled = true
path = "quarantine"
write_redacted = true
write_report = true
write_bundle = false
"#,
        )
        .expect("config fixture should be written");

        let config = ServerConfig::from_sources(
            Some(&path),
            Some("0.0.0.0:19090".to_string()),
            Some("env-secret".to_string()),
            Some("https://app.example,https://ops.example".to_string()),
            None,
            Some(OsString::from("bundles")),
        )
        .expect("config should load");

        assert_eq!(config.bind_address, "0.0.0.0:19090");
        assert_eq!(config.api_key.as_deref(), Some("env-secret"));
        assert_eq!(config.bundle_output_root, Some(PathBuf::from("bundles")));
        assert_eq!(
            config.ack_policy.mode,
            crate::models::AckPolicyMode::Enhanced
        );
        assert_eq!(
            config.ack_policy.reject_on,
            vec![crate::models::AckPolicyRejectCondition::ValidationError]
        );
        assert!(!config.ack_policy.include_error_text);
        assert!(config.quarantine.enabled);
        assert_eq!(config.quarantine.path, Some(PathBuf::from("quarantine")));
        assert!(config.quarantine.write_redacted);
        assert!(config.quarantine.write_report);
        assert!(!config.quarantine.write_bundle);
        assert_eq!(
            config.cors_allowed_origins,
            CorsAllowedOrigins::List(vec![
                "https://app.example".to_string(),
                "https://ops.example".to_string()
            ])
        );
        assert_eq!(config.config_source, Some(path.display().to_string()));

        fs::remove_file(path).expect("config fixture should be removed");
    }

    #[test]
    fn public_config_redacts_api_key_value() {
        let config = ServerConfig {
            api_key: Some("super-secret".to_string()),
            ..ServerConfig::default()
        };

        let printed = serde_json::to_string(&config.to_public_config())
            .expect("public config should serialize");

        assert!(printed.contains("\"api_key_configured\":true"));
        assert!(!printed.contains("super-secret"));
    }

    #[test]
    fn public_config_includes_ack_policy_without_secret_material() {
        let printed = serde_json::to_string(&ServerConfig::default().to_public_config())
            .expect("public config should serialize");

        assert!(printed.contains("\"ack_policy\""));
        assert!(printed.contains("\"mode\":\"original\""));
        assert!(printed.contains("\"reject_on\":[\"parse_error\",\"validation_error\"]"));
    }

    #[test]
    fn public_config_reports_quarantine_without_exposing_path() {
        let config = ServerConfig {
            quarantine: crate::models::QuarantineConfig {
                enabled: true,
                path: Some(PathBuf::from("sensitive-quarantine-path")),
                write_redacted: true,
                write_report: true,
                write_bundle: true,
            },
            ..ServerConfig::default()
        };

        let printed = serde_json::to_string(&config.to_public_config())
            .expect("public config should serialize");

        assert!(printed.contains("\"quarantine\""));
        assert!(printed.contains("\"enabled\":true"));
        assert!(printed.contains("\"path_configured\":true"));
        assert!(!printed.contains("sensitive-quarantine-path"));
    }

    #[test]
    fn readiness_checks_load_configured_profiles() {
        let path = temp_file_path("ready-profile.yaml");
        fs::write(
            &path,
            r#"
message_structure: ADT_A01
version: "2.5"
segments:
  - id: MSH
"#,
        )
        .expect("profile fixture should be written");

        let config = ServerConfig::from_sources(
            None,
            None,
            None,
            None,
            Some(OsString::from(path.as_os_str())),
            None,
        )
        .expect("config should build");
        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "configured_profiles")
            .expect("configured profile check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Pass);
        assert!(check.message.contains("loaded 1 configured profile"));

        fs::remove_file(path).expect("profile fixture should be removed");
    }

    #[test]
    fn readiness_checks_fail_missing_configured_profile() {
        let config = ServerConfig {
            profile_paths: vec!["missing-profile.yaml".to_string()],
            ..ServerConfig::default()
        };
        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "configured_profiles")
            .expect("configured profile check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Fail);
        assert!(check.message.contains("configured profile 1"));
        assert!(!check.message.contains("missing-profile.yaml"));
    }

    #[test]
    fn readiness_checks_fail_invalid_configured_profile_without_exposing_path() {
        let path = temp_file_path("invalid-ready-profile.yaml");
        fs::write(&path, "message_structure: [").expect("profile fixture should be written");
        let path_text = path.display().to_string();
        let config = ServerConfig {
            profile_paths: vec![path_text.clone()],
            ..ServerConfig::default()
        };
        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "configured_profiles")
            .expect("configured profile check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Fail);
        assert!(check.message.contains("configured profile 1"));
        assert!(!check.message.contains(&path_text));
        assert!(!check.message.contains("invalid-ready-profile.yaml"));

        fs::remove_file(path).expect("profile fixture should be removed");
    }

    #[test]
    fn public_config_reports_bundle_output_root_without_exposing_path() {
        let config = ServerConfig {
            bundle_output_root: Some(PathBuf::from("sensitive-local-path")),
            ..ServerConfig::default()
        };

        let printed = serde_json::to_string(&config.to_public_config())
            .expect("public config should serialize");

        assert!(printed.contains("\"bundle_output_root_configured\":true"));
        assert!(!printed.contains("sensitive-local-path"));
    }

    #[test]
    fn readiness_checks_configured_bundle_output_root_writable() {
        let root = temp_dir_path("bundle-root");
        fs::create_dir(&root).expect("bundle root should be created");
        let config = ServerConfig {
            bundle_output_root: Some(root.clone()),
            ..ServerConfig::default()
        };

        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "bundle_output_root")
            .expect("bundle root readiness check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Pass);

        fs::remove_dir(root).expect("bundle root should be removed");
    }

    #[test]
    fn readiness_checks_fail_missing_bundle_output_root() {
        let root = temp_dir_path("missing-bundle-root");
        let config = ServerConfig {
            bundle_output_root: Some(root),
            ..ServerConfig::default()
        };
        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "bundle_output_root")
            .expect("bundle root readiness check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Fail);
    }

    #[test]
    fn readiness_checks_configured_quarantine_output_writable() {
        let root = temp_dir_path("quarantine-root");
        fs::create_dir(&root).expect("quarantine root should be created");
        let config = ServerConfig {
            quarantine: crate::models::QuarantineConfig {
                enabled: true,
                path: Some(root.clone()),
                ..Default::default()
            },
            ..ServerConfig::default()
        };

        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "quarantine_output")
            .expect("quarantine readiness check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Pass);

        fs::remove_dir(root).expect("quarantine root should be removed");
    }

    #[test]
    fn readiness_checks_fail_enabled_quarantine_without_path() {
        let config = ServerConfig {
            quarantine: crate::models::QuarantineConfig {
                enabled: true,
                path: None,
                ..Default::default()
            },
            ..ServerConfig::default()
        };
        let check = config
            .readiness_checks()
            .into_iter()
            .find(|check| check.name == "quarantine_output")
            .expect("quarantine readiness check should exist");

        assert_eq!(check.status, crate::models::ReadinessCheckStatus::Fail);
    }

    fn temp_file_path(name: &str) -> PathBuf {
        temp_dir_path(name)
    }

    fn temp_dir_path(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "hl7v2-server-{}-{nonce}-{name}",
            std::process::id()
        ))
    }
}
