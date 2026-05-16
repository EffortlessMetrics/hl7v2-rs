//! Contract tests for the gRPC service implementation.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::uninlined_format_args,
    reason = "legacy gRPC contract tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]

#[cfg(test)]
mod tests {
    use hl7v2_server::QuarantineConfig;
    use hl7v2_server::grpc::Hl7ServiceImpl;
    use hl7v2_server::grpc::proto::hl7_service_client::Hl7ServiceClient;
    use hl7v2_server::grpc::proto::hl7_service_server::Hl7Service;
    use hl7v2_server::grpc::proto::{
        CorpusDiffRequest, CorpusFingerprintRequest, CorpusMessageInput, CorpusSummarizeRequest,
        CreateEvidenceBundleRequest, GenerateAckRequest, HealthCheckRequest, NormalizeOptions,
        NormalizeRequest, ParseRequest, ParseStreamRequest, ParseStreamResponse,
        ProfileExplainRequest, ProfileLintRequest, ProfileTestFixture, ProfileTestRequest,
        ReplayEvidenceBundleRequest, ValidateRedactedRequest, ValidateRequest,
        evidence_replay_check, generate_ack_request, health_check_response, profile_test_fixture,
        validation_issue,
    };
    use hl7v2_server::server::{AppState, ServerConfig};
    use hl7v2_test_utils::{
        PHI_LEAK_SENTINEL_MESSAGE as PHI_MESSAGE, PHI_LEAK_SENTINEL_POLICY as REDACTION_POLICY,
        SampleMessages, assert_no_phi_leak_sentinels,
    };
    use http_body_util::Full;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use prost::Message as ProstMessage;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::net::TcpListener;
    use tokio::time::sleep;
    use tokio_stream::StreamExt;
    use tonic::codec::{Codec, ProstCodec, Streaming};
    use tonic::codegen::Bytes;
    use tonic::metadata::MetadataValue;
    use tonic::{Code, Request};

    /// Helper to create a mock AppState
    fn mock_state() -> Arc<AppState> {
        mock_state_with_bundle_output_root(None)
    }

    fn mock_state_with_bundle_output_root(bundle_output_root: Option<PathBuf>) -> Arc<AppState> {
        mock_state_with_roots(bundle_output_root, Default::default())
    }

    fn mock_state_with_roots(
        bundle_output_root: Option<PathBuf>,
        quarantine: QuarantineConfig,
    ) -> Arc<AppState> {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(handle),
            api_key: None,
            cors_allowed_origins: Default::default(),
            readiness_checks: ServerConfig::default().readiness_checks(),
            bundle_output_root,
            ack_policy: Default::default(),
            quarantine,
        })
    }

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after UNIX epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hl7v2-server-grpc-bundle-{}-{nonce}-{name}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temp root should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            match fs::remove_dir_all(&self.path) {
                Ok(()) | Err(_) => {}
            }
        }
    }

    const SAMPLE_MSG: &[u8] = b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";
    const CUSTOM_DELIMS_MSG: &[u8] = b"MSH*%$!?*SENDAPP*SENDFAC*RECVAPP*RECVFAC*202605030101**ADT%A01*CTRL123*P*2.5\rPID*1**123456%%%HOSP%MR**Doe%John**19700101*M\r";
    const PROFILE: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
"#;

    const PROFILE_REQUIRES_DROPPED_NAME: &str = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.5"
    required: true
"#;

    fn service() -> Hl7ServiceImpl {
        Hl7ServiceImpl::new(mock_state())
    }

    fn service_with_bundle_output_root(root: PathBuf) -> Hl7ServiceImpl {
        Hl7ServiceImpl::new(mock_state_with_bundle_output_root(Some(root)))
    }

    fn service_with_quarantine(quarantine: QuarantineConfig) -> Hl7ServiceImpl {
        Hl7ServiceImpl::new(mock_state_with_roots(None, quarantine))
    }

    fn dirty_real_world_fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_data/dirty-real-world")
    }

    fn normalize_fixture_segments(bytes: &[u8]) -> Vec<u8> {
        String::from_utf8_lossy(bytes)
            .replace("\r\n", "\n")
            .replace('\n', "\r")
            .into_bytes()
    }

    fn dirty_corpus_messages(category: &str) -> Vec<CorpusMessageInput> {
        let source = dirty_real_world_fixture_root().join(category);
        let mut paths = fs::read_dir(&source)
            .expect("dirty fixture category should be readable")
            .map(|entry| {
                entry
                    .expect("dirty fixture entry should be readable")
                    .path()
            })
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        paths.sort();

        paths
            .into_iter()
            .map(|path| {
                let bytes = fs::read(&path).expect("dirty fixture file should be readable");
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("dirty fixture file should have a UTF-8 name");
                CorpusMessageInput {
                    id: Some(file_name.to_string()),
                    message: normalize_fixture_segments(&bytes),
                }
            })
            .collect()
    }

    fn dirty_after_corpus_messages() -> Vec<CorpusMessageInput> {
        let mut messages = dirty_corpus_messages("after");
        let source = dirty_real_world_fixture_root().join("sources/mllp-source.hl7");
        let bytes = fs::read(&source).expect("MLLP source fixture should be readable");
        let normalized = normalize_fixture_segments(&bytes);
        messages.push(CorpusMessageInput {
            id: Some("mllp-framed.hl7".to_string()),
            message: hl7v2::wrap_mllp(&normalized),
        });
        messages
    }

    fn grpc_request_body<T: ProstMessage>(messages: &[T]) -> Full<Bytes> {
        let mut body = Vec::new();
        for message in messages {
            let encoded = message.encode_to_vec();
            let encoded_len =
                u32::try_from(encoded.len()).expect("gRPC test fixture should fit in u32");
            body.push(0);
            body.extend_from_slice(&encoded_len.to_be_bytes());
            body.extend_from_slice(&encoded);
        }
        Full::new(Bytes::from(body))
    }

    async fn normalize(
        service: &Hl7ServiceImpl,
        message: &[u8],
        canonical_delimiters: bool,
        mllp_frame: bool,
    ) -> Vec<u8> {
        service
            .normalize(Request::new(NormalizeRequest {
                message: message.to_vec(),
                options: Some(NormalizeOptions {
                    canonical_delimiters,
                    mllp_frame,
                    sort_fields: false,
                }),
            }))
            .await
            .expect("RPC should succeed")
            .into_inner()
            .normalized
    }

    #[tokio::test]
    async fn test_grpc_parse_raw_hl7_success() {
        let service = service();
        let request = Request::new(ParseRequest {
            message: SAMPLE_MSG.to_vec(),
            mllp_framed: false,
            options: None,
        });

        let response = service.parse(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.success);
        let message = inner.message.expect("Parsed message should exist");
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "PID");

        let metadata = inner.metadata.expect("Metadata should exist");
        assert_eq!(metadata.message_type, "ADT^A01");
        assert_eq!(metadata.control_id, "CTRL123");
        assert_eq!(metadata.version, "2.5");
        assert_eq!(metadata.sending_facility, "SENDFAC");
        assert_eq!(metadata.receiving_facility, "RECVFAC");
    }

    #[tokio::test]
    async fn test_grpc_parse_mllp_success() {
        let service = service();
        let mllp_msg = hl7v2::wrap_mllp(SAMPLE_MSG);

        let request = Request::new(ParseRequest {
            message: mllp_msg,
            mllp_framed: true,
            options: None,
        });

        let response = service.parse(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.success);
        assert!(inner.message.is_some());
        assert_eq!(
            inner.metadata.expect("Metadata should exist").control_id,
            "CTRL123"
        );
    }

    #[tokio::test]
    async fn test_grpc_parse_invalid_hl7_returns_parse_error() {
        let service = service();
        let request = Request::new(ParseRequest {
            message: b"not an HL7 message".to_vec(),
            mllp_framed: false,
            options: None,
        });

        let response = service.parse(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(!inner.success);
        assert!(inner.message.is_none());
        assert_eq!(inner.errors.len(), 1);
        assert_eq!(inner.errors[0].code, "PARSE_ERROR");
    }

    #[tokio::test]
    async fn test_grpc_generate_ack_maps_codes_and_preserves_control_id() {
        let service = service();

        for (code, expected) in [
            (generate_ack_request::AckCode::Aa, "AA"),
            (generate_ack_request::AckCode::Ae, "AE"),
            (generate_ack_request::AckCode::Ar, "AR"),
        ] {
            let response = service
                .generate_ack(Request::new(GenerateAckRequest {
                    message: SAMPLE_MSG.to_vec(),
                    code: code as i32,
                    error_message: None,
                }))
                .await
                .expect("RPC should succeed");
            let inner = response.into_inner();

            let ack_str = String::from_utf8(inner.ack_message).expect("ACK should be UTF-8");
            assert!(ack_str.starts_with("MSH"));
            assert!(
                ack_str.contains(&format!("MSA|{}|CTRL123", expected)),
                "ACK did not preserve code/control id: {ack_str}"
            );
            assert!(inner.parsed_ack.is_some());
        }
    }

    #[tokio::test]
    async fn test_grpc_normalize_canonical_output_and_idempotence() {
        let service = service();

        let normalized = normalize(&service, CUSTOM_DELIMS_MSG, true, false).await;
        let normalized_str = String::from_utf8(normalized.clone()).expect("HL7 should be UTF-8");

        assert!(normalized_str.starts_with("MSH|^~\\&|"));
        assert!(normalized_str.contains("ADT^A01"));
        assert!(normalized_str.contains("PID|1||123456^^^HOSP^MR||Doe^John||19700101|M"));

        let renormalized = normalize(&service, &normalized, true, false).await;
        assert_eq!(normalized, renormalized);
    }

    #[tokio::test]
    async fn test_grpc_normalize_optional_mllp_framing() {
        let service = service();

        let unframed = normalize(&service, SAMPLE_MSG, true, false).await;
        let framed = normalize(&service, SAMPLE_MSG, true, true).await;

        assert!(framed.starts_with(&[0x0b]));
        assert!(framed.ends_with(&[0x1c, 0x0d]));
        assert_eq!(hl7v2::unwrap_mllp(&framed).unwrap(), unframed.as_slice());
    }

    #[tokio::test]
    async fn test_grpc_validate_valid_profile() {
        let service = service();
        let request = Request::new(ValidateRequest {
            message: SAMPLE_MSG.to_vec(),
            profile: PROFILE.to_string(),
            mllp_framed: false,
            options: None,
            report_schema_version: 0,
        });

        let response = service.validate(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.valid);
        assert!(inner.errors.is_empty());
        assert!(inner.warnings.is_empty());
        let summary = inner.summary.expect("Summary should exist");
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);

        let report = inner
            .validation_report
            .expect("Validation report should exist");
        assert!(report.valid);
        assert_eq!(report.message_type, "ADT^A01");
        assert_eq!(report.profile.as_deref(), Some("ADT_A01"));
        assert_eq!(report.segment_count, 2);
        assert_eq!(report.issue_count, 0);
        assert!(report.issues.is_empty());
        assert!(inner.validation_report_v2.is_none());
    }

    #[tokio::test]
    async fn test_grpc_validate_mllp_framed_message() {
        let service = service();
        let request = Request::new(ValidateRequest {
            message: hl7v2::wrap_mllp(SAMPLE_MSG),
            profile: PROFILE.to_string(),
            mllp_framed: true,
            options: None,
            report_schema_version: 0,
        });

        let response = service.validate(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.valid);
        let report = inner
            .validation_report
            .expect("Validation report should exist");
        assert_eq!(report.message_type, "ADT^A01");
        assert_eq!(report.profile.as_deref(), Some("ADT_A01"));
    }

    #[tokio::test]
    async fn test_grpc_validate_invalid_profile_returns_invalid_argument() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";

        let request = Request::new(ValidateRequest {
            message: SAMPLE_MSG.to_vec(),
            profile: sensitive_profile.to_string(),
            mllp_framed: false,
            options: None,
            report_schema_version: 0,
        });

        let err = service
            .validate(request)
            .await
            .expect_err("Malformed profile should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "profile could not be loaded; run profile lint for details"
        );
        assert!(!err.message().contains("Jane Secret"));
        assert!(!err.message().contains("MRN-SECRET-123"));
        assert!(!err.message().contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_validate_separates_errors_from_warnings() {
        let service = service();
        let profile = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
constraints:
  - path: "PID.99"
    required: true
"#;

        let request = Request::new(ValidateRequest {
            message: SAMPLE_MSG.to_vec(),
            profile: profile.to_string(),
            mllp_framed: false,
            options: None,
            report_schema_version: 2,
        });

        let response = service.validate(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(!inner.valid);
        assert_eq!(inner.errors.len(), 1);
        assert_eq!(inner.errors[0].code, "MISSING_REQUIRED_FIELD");
        assert_eq!(
            inner.errors[0].severity,
            validation_issue::Severity::Error as i32
        );
        assert!(inner.warnings.is_empty());

        let summary = inner.summary.expect("Summary should exist");
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.warning_count, 0);

        let report = inner
            .validation_report
            .expect("Validation report should exist");
        assert!(!report.valid);
        assert_eq!(report.message_type, "ADT^A01");
        assert_eq!(report.profile.as_deref(), Some("ADT_A01"));
        assert_eq!(report.segment_count, 2);
        assert_eq!(report.issue_count, 1);
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].code, "missing_required_field");
        assert_eq!(report.issues[0].severity, "error");
        assert_eq!(report.issues[0].path.as_deref(), Some("PID.99"));
        assert_eq!(
            report.issues[0].rule_id.as_deref(),
            Some("missing_required_field")
        );
        assert_eq!(report.issues[0].segment_index, Some(1));
        assert_eq!(report.issues[0].field_index, Some(99));

        let report_v2 = inner
            .validation_report_v2
            .expect("Validation report v2 should exist");
        assert_eq!(report_v2.schema_version, "2");
        assert_eq!(report_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(report_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert!(!report_v2.valid);
        assert_eq!(report_v2.message_type, "ADT^A01");
        assert_eq!(report_v2.profile.as_deref(), Some("ADT_A01"));
        let identity = report_v2
            .profile_identity
            .expect("Profile identity should exist");
        assert_eq!(identity.label, "ADT_A01");
        assert_eq!(identity.message_structure.as_deref(), Some("ADT_A01"));
        assert_eq!(identity.version.as_deref(), Some("2.5"));
        assert_eq!(report_v2.issues[0].code, "missing_required_field");
    }

    #[tokio::test]
    async fn test_grpc_profile_lint_accepts_valid_profile() {
        let service = service();
        let response = service
            .profile_lint(Request::new(ProfileLintRequest {
                profile: PROFILE.to_string(),
                report_schema_version: 0,
            }))
            .await
            .expect("ProfileLint should succeed")
            .into_inner();

        let report = response
            .profile_lint_report
            .expect("profile lint report should exist");
        assert!(report.valid);
        assert_eq!(report.error_count, 0);
        assert_eq!(report.warning_count, 0);
        assert_eq!(report.issue_count, 0);
        assert!(report.issues.is_empty());
        assert!(response.profile_lint_report_v2.is_none());
    }

    #[tokio::test]
    async fn test_grpc_profile_lint_reports_warnings_and_v2_provenance() {
        let service = service();
        let profile = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
unknown_top_level: "ignored"
"#;

        let response = service
            .profile_lint(Request::new(ProfileLintRequest {
                profile: profile.to_string(),
                report_schema_version: 2,
            }))
            .await
            .expect("ProfileLint should succeed")
            .into_inner();

        let report = response
            .profile_lint_report
            .expect("profile lint report should exist");
        assert!(report.valid);
        assert_eq!(report.error_count, 0);
        assert_eq!(report.warning_count, 1);
        assert_eq!(report.issue_count, 1);
        assert_eq!(report.issues[0].code, "unknown_top_level_key");
        assert_eq!(report.issues[0].severity, "warning");
        assert_eq!(report.issues[0].path.as_deref(), Some("unknown_top_level"));

        let report_v2 = response
            .profile_lint_report_v2
            .expect("profile lint report v2 should exist");
        assert_eq!(report_v2.schema_version, "2");
        assert_eq!(report_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(report_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert!(report_v2.valid);
        assert_eq!(report_v2.warning_count, 1);
        assert_eq!(report_v2.issues[0].code, "unknown_top_level_key");
    }

    #[tokio::test]
    async fn test_grpc_profile_lint_invalid_yaml_does_not_echo_profile_text() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";

        let response = service
            .profile_lint(Request::new(ProfileLintRequest {
                profile: sensitive_profile.to_string(),
                report_schema_version: 0,
            }))
            .await
            .expect("ProfileLint should return a report for malformed YAML")
            .into_inner();
        let response_debug = format!("{response:?}");

        let report = response
            .profile_lint_report
            .expect("profile lint report should exist");
        assert!(!report.valid);
        assert_eq!(report.error_count, 1);
        assert_eq!(report.issue_count, 1);
        assert_eq!(report.issues[0].code, "yaml_parse_error");
        assert!(!response_debug.contains("Jane Secret"));
        assert!(!response_debug.contains("MRN-SECRET-123"));
        assert!(!response_debug.contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_profile_lint_rejects_unsupported_schema_versions() {
        let service = service();
        let err = service
            .profile_lint(Request::new(ProfileLintRequest {
                profile: PROFILE.to_string(),
                report_schema_version: 3,
            }))
            .await
            .expect_err("unsupported profile lint schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported profile lint report schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_profile_explain_reports_contract_shape() {
        let service = service();
        let response = service
            .profile_explain(Request::new(ProfileExplainRequest {
                profile: PROFILE.to_string(),
                report_schema_version: 0,
            }))
            .await
            .expect("ProfileExplain should succeed")
            .into_inner();

        let report = response
            .profile_explain_report
            .expect("profile explain report should exist");
        assert_eq!(report.profile, "<inline-profile>");
        assert_eq!(report.profile_sha256.len(), 64);
        assert_eq!(report.message_structure, "ADT_A01");
        assert_eq!(report.version, "2.5");
        let summary = report.summary.expect("summary should exist");
        assert_eq!(summary.segment_count, 2);
        assert_eq!(summary.required_field_count, 1);
        assert_eq!(summary.field_constraint_count, 1);
        assert_eq!(report.segments[0].id, "MSH");
        assert_eq!(report.segments[1].id, "PID");
        assert_eq!(report.required_fields[0].path, "PID.3");
        assert!(!report.required_fields[0].conditional);
        assert_eq!(report.field_constraints[0].path, "PID.3");
        assert!(report.field_constraints[0].required);
        let lint = report.lint.expect("lint summary should exist");
        assert!(lint.valid);
        assert_eq!(lint.issue_count, 0);
        assert!(response.profile_explain_report_v2.is_none());
    }

    #[tokio::test]
    async fn test_grpc_profile_explain_reports_warnings_and_v2_provenance() {
        let service = service();
        let profile = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
  - id: "PID"
unknown_top_level: "ignored"
"#;

        let response = service
            .profile_explain(Request::new(ProfileExplainRequest {
                profile: profile.to_string(),
                report_schema_version: 2,
            }))
            .await
            .expect("ProfileExplain should succeed")
            .into_inner();

        let report = response
            .profile_explain_report
            .expect("profile explain report should exist");
        let lint = report.lint.expect("lint summary should exist");
        assert!(lint.valid);
        assert_eq!(lint.warning_count, 1);
        assert_eq!(lint.ignored_or_unsupported[0].code, "unknown_top_level_key");
        assert_eq!(
            lint.ignored_or_unsupported[0].path.as_deref(),
            Some("unknown_top_level")
        );

        let report_v2 = response
            .profile_explain_report_v2
            .expect("profile explain report v2 should exist");
        assert_eq!(report_v2.schema_version, "2");
        assert_eq!(report_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(report_v2.tool_version, env!("CARGO_PKG_VERSION"));
        let nested = report_v2
            .report
            .expect("v2 report should contain v1 fields");
        assert_eq!(nested.message_structure, "ADT_A01");
        assert_eq!(nested.version, "2.5");
    }

    #[tokio::test]
    async fn test_grpc_profile_explain_invalid_yaml_does_not_echo_profile_text() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";

        let err = service
            .profile_explain(Request::new(ProfileExplainRequest {
                profile: sensitive_profile.to_string(),
                report_schema_version: 0,
            }))
            .await
            .expect_err("malformed profile should fail ProfileExplain");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "profile could not be loaded; run profile lint for details"
        );
        assert!(!err.message().contains("Jane Secret"));
        assert!(!err.message().contains("MRN-SECRET-123"));
        assert!(!err.message().contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_profile_explain_rejects_unsupported_schema_versions() {
        let service = service();
        let err = service
            .profile_explain(Request::new(ProfileExplainRequest {
                profile: PROFILE.to_string(),
                report_schema_version: 3,
            }))
            .await
            .expect_err("unsupported profile explain schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported profile explain report schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_profile_test_reports_fixture_results_and_v2_provenance() {
        let service = service();
        let invalid_missing_pid3 =
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL124|P|2.5\rPID|1\r";
        let response = service
            .profile_test(Request::new(ProfileTestRequest {
                profile: PROFILE.to_string(),
                fixtures: vec![
                    ProfileTestFixture {
                        name: "valid/adt.hl7".to_string(),
                        message: SAMPLE_MSG.to_vec(),
                        expectation: profile_test_fixture::Expectation::Valid as i32,
                        mllp_framed: false,
                        expected_report_json: None,
                    },
                    ProfileTestFixture {
                        name: "invalid/missing_pid3.hl7".to_string(),
                        message: invalid_missing_pid3.to_vec(),
                        expectation: profile_test_fixture::Expectation::Invalid as i32,
                        mllp_framed: false,
                        expected_report_json: Some(
                            r#"{"valid":false,"issue_count":1,"issues":[{"code":"missing_required_field","path":"PID.3"}]}"#
                                .to_string(),
                        ),
                    },
                ],
                report_schema_version: 2,
            }))
            .await
            .expect("ProfileTest should succeed")
            .into_inner();

        let report = response
            .profile_test_report
            .expect("profile test report should exist");
        assert!(report.valid);
        assert_eq!(report.profile, "<inline-profile>");
        assert_eq!(report.fixtures, "<inline-fixtures>");
        assert_eq!(report.case_count, 2);
        assert_eq!(report.passed_count, 2);
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.cases[0].name, "valid/adt.hl7");
        assert_eq!(report.cases[0].expectation, "valid");
        assert!(report.cases[0].passed);
        assert!(
            report.cases[0]
                .validation_report
                .as_ref()
                .expect("valid fixture should include validation report")
                .valid
        );

        let invalid_case = &report.cases[1];
        assert_eq!(invalid_case.name, "invalid/missing_pid3.hl7");
        assert_eq!(invalid_case.expectation, "invalid");
        assert!(invalid_case.passed);
        let validation_report = invalid_case
            .validation_report
            .as_ref()
            .expect("invalid fixture should include validation report");
        assert!(!validation_report.valid);
        assert_eq!(
            validation_report.profile.as_deref(),
            Some("<inline-profile>")
        );
        assert_eq!(validation_report.issues[0].code, "missing_required_field");
        assert_eq!(validation_report.issues[0].path.as_deref(), Some("PID.3"));
        let expected_report = invalid_case
            .expected_report
            .as_ref()
            .expect("expected report comparison should exist");
        assert!(expected_report.matched);
        assert!(invalid_case.message.contains("expected report matched"));

        let report_v2 = response
            .profile_test_report_v2
            .expect("profile test report v2 should exist");
        assert_eq!(report_v2.schema_version, "2");
        assert_eq!(report_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(report_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            report_v2
                .report
                .expect("v2 report should contain v1 fields")
                .case_count,
            report.case_count
        );
    }

    #[tokio::test]
    async fn test_grpc_profile_test_invalid_yaml_does_not_echo_profile_text() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";

        let err = service
            .profile_test(Request::new(ProfileTestRequest {
                profile: sensitive_profile.to_string(),
                fixtures: vec![ProfileTestFixture {
                    name: "fixture-1".to_string(),
                    message: SAMPLE_MSG.to_vec(),
                    expectation: profile_test_fixture::Expectation::Valid as i32,
                    mllp_framed: false,
                    expected_report_json: None,
                }],
                report_schema_version: 0,
            }))
            .await
            .expect_err("malformed profile should fail ProfileTest");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "profile could not be loaded; run profile lint for details"
        );
        assert!(!err.message().contains("Jane Secret"));
        assert!(!err.message().contains("MRN-SECRET-123"));
        assert!(!err.message().contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_profile_test_rejects_empty_fixtures() {
        let service = service();
        let err = service
            .profile_test(Request::new(ProfileTestRequest {
                profile: PROFILE.to_string(),
                fixtures: Vec::new(),
                report_schema_version: 0,
            }))
            .await
            .expect_err("empty profile test fixtures should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "fixtures must contain at least one fixture");
    }

    #[tokio::test]
    async fn test_grpc_profile_test_rejects_unsupported_schema_versions() {
        let service = service();
        let err = service
            .profile_test(Request::new(ProfileTestRequest {
                profile: PROFILE.to_string(),
                fixtures: vec![ProfileTestFixture {
                    name: "valid/adt.hl7".to_string(),
                    message: SAMPLE_MSG.to_vec(),
                    expectation: profile_test_fixture::Expectation::Valid as i32,
                    mllp_framed: false,
                    expected_report_json: None,
                }],
                report_schema_version: 3,
            }))
            .await
            .expect_err("unsupported profile test schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported profile test report schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_returns_report_receipt_and_redacted_hl7_without_phi() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: true,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let response = service
            .validate_redacted(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();
        let response_debug = format!("{inner:?}");

        let report = inner
            .validation_report
            .expect("Validation report should exist");
        assert!(report.valid);
        assert_eq!(report.message_type, "ADT^A01");
        assert_eq!(report.profile.as_deref(), Some("ADT_A01"));
        assert!(inner.validation_report_v2.is_none());

        let receipt = inner
            .redaction_receipt
            .expect("Redaction receipt should exist");
        assert!(receipt.phi_removed);
        assert_eq!(receipt.hash_algorithm, "sha256");
        assert!(receipt.actions.iter().any(|action| {
            action.path == "PID.3" && action.action == "hash" && action.status == "applied"
        }));
        assert!(inner.redaction_receipt_v2.is_none());

        let redacted_hl7 =
            String::from_utf8(inner.redacted_hl7.expect("Redacted HL7 should be included"))
                .expect("Redacted HL7 should be UTF-8");
        assert!(redacted_hl7.contains("hash:sha256:"));
        assert_no_phi(&response_debug);
        assert_no_phi(&redacted_hl7);
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_omits_redacted_hl7_unless_requested() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let response = service
            .validate_redacted(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.validation_report.expect("report should exist").valid);
        assert!(inner.redacted_hl7.is_none());
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_invalid_profile_does_not_echo_profile_text() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: sensitive_profile.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let err = service
            .validate_redacted(request)
            .await
            .expect_err("Malformed profile should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "profile could not be loaded; run profile lint for details"
        );
        assert!(!err.message().contains("Jane Secret"));
        assert!(!err.message().contains("MRN-SECRET-123"));
        assert!(!err.message().contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_accepts_mllp_framed_message() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: hl7v2::wrap_mllp(PHI_MESSAGE.as_bytes()),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: true,
            include_redacted_hl7: true,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let response = service
            .validate_redacted(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.validation_report.expect("report should exist").valid);
        let redacted_hl7 =
            String::from_utf8(inner.redacted_hl7.expect("Redacted HL7 should be included"))
                .expect("Redacted HL7 should be UTF-8");
        assert!(redacted_hl7.contains("hash:sha256:"));
        assert_no_phi(&redacted_hl7);
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_v2_returns_provenance_without_phi() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE_REQUIRES_DROPPED_NAME.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: true,
            report_schema_version: 2,
            redaction_receipt_schema_version: 2,
            quarantine_schema_version: 0,
        });

        let response = service
            .validate_redacted(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();
        let response_debug = format!("{inner:?}");

        let report = inner
            .validation_report
            .expect("Validation report should exist");
        assert!(!report.valid);
        assert_eq!(report.issues[0].path.as_deref(), Some("PID.5"));

        let report_v2 = inner
            .validation_report_v2
            .expect("Validation report v2 should exist");
        assert_eq!(report_v2.schema_version, "2");
        assert_eq!(report_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(report_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert!(!report_v2.valid);
        let identity = report_v2
            .profile_identity
            .expect("Profile identity should exist");
        assert_eq!(identity.label, "ADT_A01");
        assert_eq!(identity.message_structure.as_deref(), Some("ADT_A01"));
        assert_eq!(identity.version.as_deref(), Some("2.5"));
        assert_eq!(identity.sha256.as_deref().unwrap().len(), 64);

        let receipt_v2 = inner
            .redaction_receipt_v2
            .expect("Redaction receipt v2 should exist");
        assert_eq!(receipt_v2.schema_version, "2");
        assert_eq!(receipt_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(receipt_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert!(receipt_v2.phi_removed);
        assert_eq!(receipt_v2.hash_algorithm, "sha256");
        assert!(receipt_v2.actions.iter().any(|action| {
            action.path == "PID.3" && action.action == "hash" && action.status == "applied"
        }));
        assert_no_phi(&response_debug);
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_fails_closed_when_policy_misses_sensitive_fields() {
        let service = service();
        let incomplete_policy = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "hash patient identifier"
"#;
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: incomplete_policy.to_string(),
            mllp_framed: false,
            include_redacted_hl7: true,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let err = service
            .validate_redacted(request)
            .await
            .expect_err("Incomplete policy should fail closed");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.message().contains("Failed to redact HL7"));
        assert!(err.message().contains("PID.5"));
        assert_no_phi(err.message());
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_rejects_unsupported_schema_versions() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 3,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let err = service
            .validate_redacted(request)
            .await
            .expect_err("Unsupported schema version should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported validation report schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_rejects_unsupported_receipt_schema_versions() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 3,
            quarantine_schema_version: 0,
        });

        let err = service
            .validate_redacted(request)
            .await
            .expect_err("Unsupported schema version should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported redaction receipt schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_writes_quarantine_for_failed_validation() {
        let root = TempRoot::new("quarantine-success");
        let service = service_with_quarantine(QuarantineConfig {
            enabled: true,
            path: Some(root.path().to_path_buf()),
            ..Default::default()
        });
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE_REQUIRES_DROPPED_NAME.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 2,
        });

        let response = service
            .validate_redacted(request)
            .await
            .expect("failed redacted validation should write quarantine output")
            .into_inner();
        let response_debug = format!("{response:?}");

        assert!(
            !response
                .validation_report
                .expect("report should exist")
                .valid
        );
        let quarantine = response
            .quarantine
            .expect("quarantine summary should exist");
        assert_eq!(quarantine.quarantine_version, "1");
        assert!(quarantine.output_dir.starts_with("quarantine-"));
        assert_eq!(quarantine.reason, "validation_error");
        assert_eq!(quarantine.validation_issue_count, 1);
        assert!(
            quarantine
                .artifacts
                .iter()
                .any(|path| path == "manifest.json")
        );

        let quarantine_v2 = response
            .quarantine_v2
            .expect("quarantine v2 summary should exist");
        assert_eq!(quarantine_v2.schema_version, "2");
        assert_eq!(quarantine_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(quarantine_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(quarantine_v2.output_dir, quarantine.output_dir);
        assert_eq!(quarantine_v2.reason, "validation_error");
        assert_no_phi(&response_debug);
        assert!(!response_debug.contains(root.path().to_string_lossy().as_ref()));

        let quarantine_dir = root.path().join(&quarantine.output_dir);
        for artifact in [
            "message.redacted.hl7",
            "validation-report.json",
            "field-paths.json",
            "profile.yaml",
            "redaction-receipt.json",
            "environment.json",
            "replay.sh",
            "replay.ps1",
            "README.md",
            "manifest.json",
        ] {
            let artifact_path = quarantine_dir.join(artifact);
            assert!(
                artifact_path.exists(),
                "missing gRPC quarantine artifact {artifact}"
            );
            let content = fs::read_to_string(artifact_path).expect("artifact should be UTF-8");
            assert_no_phi(&content);
            assert!(!content.contains(root.path().to_string_lossy().as_ref()));
        }
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_omits_quarantine_for_valid_report() {
        let root = TempRoot::new("quarantine-valid");
        let service = service_with_quarantine(QuarantineConfig {
            enabled: true,
            path: Some(root.path().to_path_buf()),
            ..Default::default()
        });
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 2,
        });

        let response = service
            .validate_redacted(request)
            .await
            .expect("valid redacted validation should not write quarantine output")
            .into_inner();

        assert!(
            response
                .validation_report
                .expect("report should exist")
                .valid
        );
        assert!(response.quarantine.is_none());
        assert!(response.quarantine_v2.is_none());
        assert!(fs::read_dir(root.path()).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_quarantine_fails_closed_without_path() {
        let service = service_with_quarantine(QuarantineConfig {
            enabled: true,
            path: None,
            ..Default::default()
        });
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE_REQUIRES_DROPPED_NAME.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 0,
        });

        let err = service
            .validate_redacted(request)
            .await
            .expect_err("enabled quarantine without a root should fail closed");

        assert_eq!(err.code(), Code::FailedPrecondition);
        assert_eq!(
            err.message(),
            "server quarantine output is enabled but no path is configured"
        );
        assert_no_phi(err.message());
    }

    #[tokio::test]
    async fn test_grpc_validate_redacted_rejects_unsupported_quarantine_schema_versions() {
        let service = service();
        let request = Request::new(ValidateRedactedRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            mllp_framed: false,
            include_redacted_hl7: false,
            report_schema_version: 0,
            redaction_receipt_schema_version: 0,
            quarantine_schema_version: 3,
        });

        let err = service
            .validate_redacted(request)
            .await
            .expect_err("Unsupported quarantine schema version should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported quarantine output schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_create_evidence_bundle_writes_redacted_bundle_and_v2_artifacts() {
        let root = TempRoot::new("bundle-success");
        let bundle_id = "MRN-SECRET-123";
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        let request = Request::new(CreateEvidenceBundleRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            bundle_id: bundle_id.to_string(),
            mllp_framed: false,
            bundle_artifact_schema_version: 2,
        });

        let response = service
            .create_evidence_bundle(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();
        let response_debug = format!("{inner:?}");
        let summary = inner.summary.expect("bundle summary should exist");

        assert_eq!(summary.bundle_version, "1");
        assert!(is_sha256_hex(&summary.output_dir));
        assert_ne!(summary.output_dir, bundle_id);
        assert_eq!(summary.message_type, "ADT^A01");
        assert!(summary.validation_valid);
        assert!(summary.redaction_phi_removed);
        assert!(
            summary
                .artifacts
                .iter()
                .any(|artifact| artifact == "manifest.json")
        );
        assert!(!response_debug.contains(root.path().to_string_lossy().as_ref()));
        assert!(!response_debug.contains(bundle_id));
        assert_no_phi(&response_debug);

        let bundle_dir = root.path().join(bundle_id);
        for artifact in [
            "message.redacted.hl7",
            "validation-report.json",
            "field-paths.json",
            "profile.yaml",
            "redaction-receipt.json",
            "environment.json",
            "replay.sh",
            "replay.ps1",
            "README.md",
            "manifest.json",
        ] {
            assert!(
                bundle_dir.join(artifact).exists(),
                "missing bundle artifact {artifact}"
            );
        }

        let redacted_message = fs::read_to_string(bundle_dir.join("message.redacted.hl7"))
            .expect("redacted message should be readable");
        assert!(redacted_message.contains("hash:sha256:"));
        assert_no_phi(&redacted_message);

        for artifact in [
            "manifest.json",
            "field-paths.json",
            "redaction-receipt.json",
            "environment.json",
        ] {
            let content = fs::read_to_string(bundle_dir.join(artifact))
                .expect("v2 bundle artifact should be readable");
            assert_no_phi(&content);
            assert!(!content.contains(root.path().to_string_lossy().as_ref()));
            let value: serde_json::Value =
                serde_json::from_str(&content).expect("artifact should be JSON");
            assert_eq!(value["schema_version"], "2", "{artifact} was not v2");
            assert_eq!(value["tool_name"], "hl7v2-server");
        }
    }

    #[tokio::test]
    async fn test_grpc_create_evidence_bundle_fails_without_configured_output_root() {
        let service = service();
        let request = Request::new(CreateEvidenceBundleRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            bundle_id: "case-001".to_string(),
            mllp_framed: false,
            bundle_artifact_schema_version: 0,
        });

        let err = service
            .create_evidence_bundle(request)
            .await
            .expect_err("missing bundle root should fail closed");

        assert_eq!(err.code(), Code::FailedPrecondition);
        assert_eq!(err.message(), "bundle output root is not configured");
        assert_no_phi(err.message());
    }

    #[tokio::test]
    async fn test_grpc_create_evidence_bundle_invalid_profile_does_not_echo_profile_text() {
        let root = TempRoot::new("invalid-profile");
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        let invalid_profile =
            "patient: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
        let request = Request::new(CreateEvidenceBundleRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: invalid_profile.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            bundle_id: "case-001".to_string(),
            mllp_framed: false,
            bundle_artifact_schema_version: 0,
        });

        let err = service
            .create_evidence_bundle(request)
            .await
            .expect_err("invalid profile should fail safely");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_no_phi(err.message());
        assert!(!err.message().contains(invalid_profile));
        assert!(fs::read_dir(root.path()).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn test_grpc_create_evidence_bundle_rejects_unsafe_bundle_id_without_writing() {
        let root = TempRoot::new("unsafe-bundle-id");
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        let request = Request::new(CreateEvidenceBundleRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            bundle_id: "../escape".to_string(),
            mllp_framed: false,
            bundle_artifact_schema_version: 0,
        });

        let err = service
            .create_evidence_bundle(request)
            .await
            .expect_err("unsafe bundle id should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_no_phi(err.message());
        assert!(fs::read_dir(root.path()).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn test_grpc_create_evidence_bundle_rejects_unsupported_schema_versions() {
        let service = service();
        let request = Request::new(CreateEvidenceBundleRequest {
            message: PHI_MESSAGE.as_bytes().to_vec(),
            profile: PROFILE.to_string(),
            redaction_policy: REDACTION_POLICY.to_string(),
            bundle_id: "case-001".to_string(),
            mllp_framed: false,
            bundle_artifact_schema_version: 3,
        });

        let err = service
            .create_evidence_bundle(request)
            .await
            .expect_err("unsupported bundle artifact schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported bundle artifact schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_replay_evidence_bundle_reproduces_bundle_and_v2_report() {
        let root = TempRoot::new("replay-success");
        let bundle_id = "MRN-SECRET-123";
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        service
            .create_evidence_bundle(Request::new(CreateEvidenceBundleRequest {
                message: PHI_MESSAGE.as_bytes().to_vec(),
                profile: PROFILE.to_string(),
                redaction_policy: REDACTION_POLICY.to_string(),
                bundle_id: bundle_id.to_string(),
                mllp_framed: false,
                bundle_artifact_schema_version: 2,
            }))
            .await
            .expect("bundle creation should succeed");

        let response = service
            .replay_evidence_bundle(Request::new(ReplayEvidenceBundleRequest {
                bundle_id: bundle_id.to_string(),
                replay_report_schema_version: 2,
            }))
            .await
            .expect("replay should succeed")
            .into_inner();
        let response_debug = format!("{response:?}");
        let report = response
            .replay_report
            .as_ref()
            .expect("replay report should exist");

        assert_eq!(report.replay_version, "1");
        assert_eq!(report.bundle_version.as_deref(), Some("1"));
        assert_eq!(report.tool_name, "hl7v2-server-grpc");
        assert_eq!(report.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(report.message_type.as_deref(), Some("ADT^A01"));
        assert!(report.reproduced);
        assert_eq!(report.validation_valid, Some(true));
        assert_eq!(report.validation_issue_count, Some(0));
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.status == evidence_replay_check::Status::Pass as i32)
        );
        assert!(report.validation_report.is_some());

        let report_v2 = response
            .replay_report_v2
            .as_ref()
            .expect("replay report v2 should exist");
        assert_eq!(report_v2.schema_version, "2");
        let nested = report_v2.report.as_ref().expect("v2 report should exist");
        assert_eq!(nested.replay_version, report.replay_version);
        assert!(nested.reproduced);

        assert_no_phi(&response_debug);
        assert!(!response_debug.contains(root.path().to_string_lossy().as_ref()));
        assert!(!response_debug.contains(bundle_id));
    }

    #[tokio::test]
    async fn test_grpc_replay_evidence_bundle_fails_without_configured_output_root() {
        let service = service();
        let err = service
            .replay_evidence_bundle(Request::new(ReplayEvidenceBundleRequest {
                bundle_id: "case-001".to_string(),
                replay_report_schema_version: 0,
            }))
            .await
            .expect_err("missing bundle root should fail closed");

        assert_eq!(err.code(), Code::FailedPrecondition);
        assert_eq!(err.message(), "bundle output root is not configured");
        assert_no_phi(err.message());
    }

    #[tokio::test]
    async fn test_grpc_replay_evidence_bundle_rejects_unsafe_bundle_id() {
        let root = TempRoot::new("replay-unsafe-id");
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        let err = service
            .replay_evidence_bundle(Request::new(ReplayEvidenceBundleRequest {
                bundle_id: "../escape".to_string(),
                replay_report_schema_version: 0,
            }))
            .await
            .expect_err("unsafe bundle id should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_no_phi(err.message());
        assert!(fs::read_dir(root.path()).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn test_grpc_replay_evidence_bundle_returns_not_found_for_missing_bundle_id() {
        let root = TempRoot::new("replay-missing");
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        let err = service
            .replay_evidence_bundle(Request::new(ReplayEvidenceBundleRequest {
                bundle_id: "missing-case".to_string(),
                replay_report_schema_version: 0,
            }))
            .await
            .expect_err("missing bundle should fail");

        assert_eq!(err.code(), Code::NotFound);
        assert_eq!(err.message(), "bundle id was not found");
        assert_no_phi(err.message());
    }

    #[tokio::test]
    async fn test_grpc_replay_evidence_bundle_rejects_unsupported_schema_versions() {
        let root = TempRoot::new("replay-schema-version");
        let service = service_with_bundle_output_root(root.path().to_path_buf());
        let err = service
            .replay_evidence_bundle(Request::new(ReplayEvidenceBundleRequest {
                bundle_id: "case-001".to_string(),
                replay_report_schema_version: 3,
            }))
            .await
            .expect_err("unsupported replay schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported replay report schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_corpus_commands_share_dirty_real_world_fixture_categories() {
        let service = service();

        let summary = service
            .corpus_summarize(Request::new(CorpusSummarizeRequest {
                messages: dirty_after_corpus_messages(),
                summary_schema_version: 2,
            }))
            .await
            .expect("CorpusSummarize should succeed")
            .into_inner();
        let summary_debug = format!("{summary:?}");

        let summary_report = summary.summary.expect("summary should exist");
        assert_eq!(summary_report.root, "<inline-corpus>");
        assert_eq!(summary_report.file_count, 6);
        assert_eq!(summary_report.message_count, 4);
        assert_eq!(summary_report.parse_error_count, 2);
        assert!(
            summary_report
                .message_types
                .iter()
                .any(|entry| entry.value == "ADT^A08" && entry.count == 1)
        );
        assert!(
            summary_report
                .message_types
                .iter()
                .any(|entry| entry.value == "ADT^A04" && entry.count == 1)
        );
        assert!(
            summary_report
                .segments
                .iter()
                .any(|entry| entry.value == "ZPV" && entry.count == 1)
        );
        assert!(
            summary_report
                .parse_errors
                .iter()
                .any(|entry| entry.path == "malformed-delimiters.hl7")
        );
        assert!(
            summary_report
                .parse_errors
                .iter()
                .any(|entry| entry.path == "partial-batch.hl7")
        );
        assert!(!summary_debug.contains("MRN-DIRTY"));

        let fingerprint = service
            .corpus_fingerprint(Request::new(CorpusFingerprintRequest {
                messages: dirty_after_corpus_messages(),
                profile: None,
                fingerprint_schema_version: 2,
            }))
            .await
            .expect("CorpusFingerprint should succeed")
            .into_inner();
        let fingerprint_debug = format!("{fingerprint:?}");

        let fingerprint_report = fingerprint.fingerprint.expect("fingerprint should exist");
        assert_eq!(fingerprint_report.root, "<inline-corpus>");
        assert_eq!(fingerprint_report.file_count, 6);
        assert_eq!(fingerprint_report.message_count, 4);
        assert_eq!(fingerprint_report.parse_error_count, 2);
        assert!(
            fingerprint_report
                .field_cardinality
                .iter()
                .any(|entry| entry.path == "OBX.5"
                    && entry.max_per_message == 20
                    && entry.total_occurrences == 20)
        );
        assert!(
            fingerprint_report
                .field_cardinality
                .iter()
                .any(|entry| entry.path == "ZPV.1" && entry.total_occurrences == 1)
        );
        assert!(!fingerprint_debug.contains("MRN-DIRTY"));

        let diff = service
            .corpus_diff(Request::new(CorpusDiffRequest {
                before: dirty_corpus_messages("before"),
                after: dirty_after_corpus_messages(),
                profile: None,
                diff_schema_version: 2,
            }))
            .await
            .expect("CorpusDiff should succeed")
            .into_inner();
        let diff_debug = format!("{diff:?}");

        let diff_report = diff.diff.expect("diff should exist");
        assert_eq!(
            diff_report
                .file_count
                .expect("file count should exist")
                .delta,
            4
        );
        assert_eq!(
            diff_report
                .message_count
                .expect("message count should exist")
                .delta,
            2
        );
        assert_eq!(
            diff_report
                .parse_error_count
                .expect("parse error count should exist")
                .delta,
            2
        );
        assert!(
            diff_report
                .field_cardinality
                .iter()
                .any(|entry| entry.path == "OBX.5"
                    && entry.max_per_message_delta == 15
                    && entry.total_occurrences_delta == 15)
        );
        assert!(!diff_debug.contains("MRN-DIRTY"));
    }

    #[tokio::test]
    async fn test_grpc_corpus_summarize_reports_counts_and_v2_provenance() {
        let service = service();
        let request = Request::new(CorpusSummarizeRequest {
            messages: vec![
                CorpusMessageInput {
                    id: None,
                    message: SAMPLE_MSG.to_vec(),
                },
                CorpusMessageInput {
                    id: Some("bad-1".to_string()),
                    message: b"not an HL7 message".to_vec(),
                },
            ],
            summary_schema_version: 2,
        });

        let response = service
            .corpus_summarize(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();
        let response_debug = format!("{inner:?}");

        let summary = inner.summary.expect("summary should exist");
        assert_eq!(summary.root, "<inline-corpus>");
        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.message_count, 1);
        assert_eq!(summary.parse_error_count, 1);
        assert!(summary.total_bytes > 0);
        assert!(
            summary
                .message_types
                .iter()
                .any(|count| count.value == "ADT^A01" && count.count == 1)
        );
        assert!(
            summary
                .segments
                .iter()
                .any(|count| count.value == "PID" && count.count == 1)
        );
        assert!(
            summary
                .field_presence
                .iter()
                .any(|field| field.path == "PID.3" && field.message_count == 1)
        );
        assert_eq!(summary.parse_errors[0].path, "bad-1");
        assert!(!summary.parse_errors[0].error.contains("not an HL7 message"));

        let summary_v2 = inner.summary_v2.expect("summary v2 should exist");
        assert_eq!(summary_v2.schema_version, "2");
        assert_eq!(summary_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(summary_v2.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(summary_v2.file_count, summary.file_count);
        assert_eq!(summary_v2.message_count, summary.message_count);
        assert_eq!(summary_v2.parse_error_count, summary.parse_error_count);
        assert_no_phi(&response_debug);
        assert!(!response_debug.contains("not an HL7 message"));
    }

    #[tokio::test]
    async fn test_grpc_corpus_summarize_rejects_empty_input() {
        let service = service();
        let request = Request::new(CorpusSummarizeRequest {
            messages: Vec::new(),
            summary_schema_version: 0,
        });

        let err = service
            .corpus_summarize(request)
            .await
            .expect_err("empty inline corpus should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "messages must contain at least one message");
    }

    #[tokio::test]
    async fn test_grpc_corpus_summarize_rejects_unsupported_schema_versions() {
        let service = service();
        let request = Request::new(CorpusSummarizeRequest {
            messages: vec![CorpusMessageInput {
                id: Some("adt-1".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            summary_schema_version: 3,
        });

        let err = service
            .corpus_summarize(request)
            .await
            .expect_err("unsupported schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported corpus summary schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_corpus_fingerprint_reports_shape_profile_counts_and_v2_provenance() {
        let service = service();
        let profile = r#"
message_structure: "ADT_A01"
version: "2.5"
segments:
  - id: "MSH"
constraints:
  - path: "PID.99"
    required: true
"#;
        let request = Request::new(CorpusFingerprintRequest {
            messages: vec![
                CorpusMessageInput {
                    id: Some("adt-1".to_string()),
                    message: SAMPLE_MSG.to_vec(),
                },
                CorpusMessageInput {
                    id: Some("bad-1".to_string()),
                    message: b"not an HL7 message".to_vec(),
                },
            ],
            profile: Some(profile.to_string()),
            fingerprint_schema_version: 2,
        });

        let response = service
            .corpus_fingerprint(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();
        let response_debug = format!("{inner:?}");

        let fingerprint = inner.fingerprint.expect("fingerprint should exist");
        assert_eq!(fingerprint.fingerprint_version, "1");
        assert_eq!(fingerprint.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(fingerprint.root, "<inline-corpus>");
        assert_eq!(fingerprint.file_count, 2);
        assert_eq!(fingerprint.message_count, 1);
        assert_eq!(fingerprint.parse_error_count, 1);
        assert!(
            fingerprint
                .message_type_counts
                .iter()
                .any(|count| count.value == "ADT^A01" && count.count == 1)
        );
        assert!(
            fingerprint
                .field_cardinality
                .iter()
                .any(|field| field.path == "PID.3"
                    && field.message_count == 1
                    && field.total_occurrences == 1)
        );
        assert!(
            fingerprint
                .validation_issue_code_counts
                .iter()
                .any(|count| count.value == "missing_required_field" && count.count == 1)
        );
        let profile = fingerprint
            .profile
            .as_ref()
            .expect("profile metadata should exist");
        assert_eq!(profile.path, "<inline-profile>");
        assert_eq!(profile.message_structure, "ADT_A01");
        assert_eq!(profile.version, "2.5");
        assert_eq!(profile.sha256.len(), 64);

        let fingerprint_v2 = inner.fingerprint_v2.expect("fingerprint v2 should exist");
        assert_eq!(fingerprint_v2.schema_version, "2");
        assert_eq!(fingerprint_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(
            fingerprint_v2.fingerprint_version,
            fingerprint.fingerprint_version
        );
        assert_eq!(fingerprint_v2.tool_version, fingerprint.tool_version);
        assert_eq!(fingerprint_v2.file_count, fingerprint.file_count);
        assert_no_phi(&response_debug);
        assert!(!response_debug.contains("not an HL7 message"));
    }

    #[tokio::test]
    async fn test_grpc_corpus_fingerprint_rejects_unsupported_schema_versions() {
        let service = service();
        let request = Request::new(CorpusFingerprintRequest {
            messages: vec![CorpusMessageInput {
                id: Some("adt-1".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            profile: None,
            fingerprint_schema_version: 3,
        });

        let err = service
            .corpus_fingerprint(request)
            .await
            .expect_err("unsupported schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported corpus fingerprint schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_corpus_fingerprint_invalid_profile_does_not_echo_profile_text() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
        let request = Request::new(CorpusFingerprintRequest {
            messages: vec![CorpusMessageInput {
                id: Some("adt-1".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            profile: Some(sensitive_profile.to_string()),
            fingerprint_schema_version: 0,
        });

        let err = service
            .corpus_fingerprint(request)
            .await
            .expect_err("malformed profile should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "profile could not be loaded; run profile lint for details"
        );
        assert!(!err.message().contains("Jane Secret"));
        assert!(!err.message().contains("MRN-SECRET-123"));
        assert!(!err.message().contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_corpus_diff_reports_inline_deltas_and_v2_provenance() {
        let service = service();
        let request = Request::new(CorpusDiffRequest {
            before: vec![CorpusMessageInput {
                id: Some("before-adt".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            after: vec![
                CorpusMessageInput {
                    id: Some("after-adt".to_string()),
                    message: SAMPLE_MSG.to_vec(),
                },
                CorpusMessageInput {
                    id: Some("after-oru".to_string()),
                    message: SampleMessages::oru_r01().as_bytes().to_vec(),
                },
            ],
            profile: None,
            diff_schema_version: 2,
        });

        let response = service
            .corpus_diff(request)
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();
        let response_debug = format!("{inner:?}");

        let diff = inner.diff.expect("diff should exist");
        assert_eq!(diff.diff_version, "1");
        assert_eq!(diff.tool_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(diff.before_root, "<inline-before>");
        assert_eq!(diff.after_root, "<inline-after>");
        assert_eq!(
            diff.message_count
                .expect("message count should exist")
                .delta,
            1
        );
        assert_eq!(diff.new_message_types, vec!["ORU^R01"]);
        assert!(
            diff.field_presence
                .iter()
                .any(|field| field.path == "OBX.5" && field.message_count_delta == 1)
        );

        let diff_v2 = inner.diff_v2.expect("diff v2 should exist");
        assert_eq!(diff_v2.schema_version, "2");
        assert_eq!(diff_v2.tool_name, "hl7v2-server-grpc");
        assert_eq!(diff_v2.diff_version, diff.diff_version);
        assert_eq!(diff_v2.tool_version, diff.tool_version);
        assert_eq!(diff_v2.before_root, diff.before_root);
        assert_eq!(diff_v2.after_root, diff.after_root);
        assert_no_phi(&response_debug);
        assert!(!response_debug.contains("Patient^Test"));
        assert!(!response_debug.contains("MRN789"));
    }

    #[tokio::test]
    async fn test_grpc_corpus_diff_rejects_unsupported_schema_versions() {
        let service = service();
        let request = Request::new(CorpusDiffRequest {
            before: vec![CorpusMessageInput {
                id: Some("before-adt".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            after: vec![CorpusMessageInput {
                id: Some("after-adt".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            profile: None,
            diff_schema_version: 3,
        });

        let err = service
            .corpus_diff(request)
            .await
            .expect_err("unsupported schema version should fail");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "unsupported corpus diff schema version 3; expected 1 or 2"
        );
    }

    #[tokio::test]
    async fn test_grpc_corpus_diff_invalid_profile_does_not_echo_profile_text() {
        let service = service();
        let sensitive_profile =
            "patient_name: Jane Secret\nmrn: MRN-SECRET-123\ninvalid: yaml: structure:";
        let request = Request::new(CorpusDiffRequest {
            before: vec![CorpusMessageInput {
                id: Some("before-adt".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            after: vec![CorpusMessageInput {
                id: Some("after-adt".to_string()),
                message: SAMPLE_MSG.to_vec(),
            }],
            profile: Some(sensitive_profile.to_string()),
            diff_schema_version: 0,
        });

        let err = service
            .corpus_diff(request)
            .await
            .expect_err("malformed profile should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(
            err.message(),
            "profile could not be loaded; run profile lint for details"
        );
        assert!(!err.message().contains("Jane Secret"));
        assert!(!err.message().contains("MRN-SECRET-123"));
        assert!(!err.message().contains(sensitive_profile));
    }

    #[tokio::test]
    async fn test_grpc_health_check_reports_serving_version() {
        let service = service();

        let response = service
            .health_check(Request::new(HealthCheckRequest {}))
            .await
            .expect("RPC should succeed");
        let inner = response.into_inner();

        assert_eq!(
            inner.status,
            health_check_response::ServingStatus::Serving as i32
        );
        assert_eq!(inner.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_grpc_transport_server_serves_health_check() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should have a local address");
        let server = hl7v2_server::Server::builder()
            .bind(addr.to_string())
            .build();
        let server_task =
            tokio::spawn(async move { server.serve_grpc_with_listener(listener).await });

        let endpoint = format!("http://{addr}");
        let mut client = None;
        for _ in 0..20 {
            match Hl7ServiceClient::connect(endpoint.clone()).await {
                Ok(value) => {
                    client = Some(value);
                    break;
                }
                Err(_) => sleep(Duration::from_millis(25)).await,
            }
        }
        let mut client = client.expect("gRPC transport server should accept connections");

        let response = client
            .health_check(Request::new(HealthCheckRequest {}))
            .await
            .expect("HealthCheck should succeed")
            .into_inner();

        assert_eq!(
            response.status,
            health_check_response::ServingStatus::Serving as i32
        );
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));

        server_task.abort();
    }

    #[tokio::test]
    async fn test_grpc_transport_rejects_missing_api_key_when_configured() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should have a local address");
        let server = hl7v2_server::Server::builder()
            .bind(addr.to_string())
            .api_key(Some("grpc-secret".to_string()))
            .build();
        let server_task =
            tokio::spawn(async move { server.serve_grpc_with_listener(listener).await });

        let endpoint = format!("http://{addr}");
        let mut client = None;
        for _ in 0..20 {
            match Hl7ServiceClient::connect(endpoint.clone()).await {
                Ok(value) => {
                    client = Some(value);
                    break;
                }
                Err(_) => sleep(Duration::from_millis(25)).await,
            }
        }
        let mut client = client.expect("gRPC transport server should accept connections");

        let err = client
            .parse(Request::new(ParseRequest {
                message: SAMPLE_MSG.to_vec(),
                mllp_framed: false,
                options: None,
            }))
            .await
            .expect_err("missing gRPC API key should be rejected");

        assert_eq!(err.code(), Code::Unauthenticated);
        assert!(err.message().contains("API key"));
        assert_no_phi(err.message());

        server_task.abort();
    }

    #[tokio::test]
    async fn test_grpc_transport_accepts_valid_api_key_when_configured() {
        let api_key = "grpc-secret";
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should have a local address");
        let server = hl7v2_server::Server::builder()
            .bind(addr.to_string())
            .api_key(Some(api_key.to_string()))
            .build();
        let server_task =
            tokio::spawn(async move { server.serve_grpc_with_listener(listener).await });

        let endpoint = format!("http://{addr}");
        let mut client = None;
        for _ in 0..20 {
            match Hl7ServiceClient::connect(endpoint.clone()).await {
                Ok(value) => {
                    client = Some(value);
                    break;
                }
                Err(_) => sleep(Duration::from_millis(25)).await,
            }
        }
        let mut client = client.expect("gRPC transport server should accept connections");
        let mut request = Request::new(ParseRequest {
            message: SAMPLE_MSG.to_vec(),
            mllp_framed: false,
            options: None,
        });
        request.metadata_mut().insert(
            "x-api-key",
            MetadataValue::try_from(api_key).expect("test API key metadata should be valid"),
        );

        let response = client
            .parse(request)
            .await
            .expect("valid gRPC API key should be accepted")
            .into_inner();

        assert!(response.success);
        assert_eq!(
            response
                .metadata
                .expect("metadata should exist")
                .message_type,
            "ADT^A01"
        );

        server_task.abort();
    }

    #[tokio::test]
    async fn test_grpc_parse_stream_parses_each_message() {
        let service = service();
        let mut codec = ProstCodec::<ParseStreamResponse, ParseStreamRequest>::default();
        let requests = vec![
            ParseStreamRequest {
                message: SAMPLE_MSG.to_vec(),
                mllp_framed: false,
                options: None,
            },
            ParseStreamRequest {
                message: b"not an HL7 message".to_vec(),
                mllp_framed: false,
                options: None,
            },
            ParseStreamRequest {
                message: SAMPLE_MSG.to_vec(),
                mllp_framed: true,
                options: None,
            },
            ParseStreamRequest {
                message: hl7v2::wrap_mllp(SAMPLE_MSG),
                mllp_framed: true,
                options: None,
            },
        ];
        let stream =
            Streaming::new_request(codec.decoder(), grpc_request_body(&requests), None, None);

        let response = service
            .parse_stream(Request::new(stream))
            .await
            .expect("ParseStream should start");
        let mut output = response.into_inner();

        let first = output
            .next()
            .await
            .expect("first response should exist")
            .expect("first response should be OK");
        assert!(first.success);
        assert_eq!(
            first.metadata.expect("metadata should exist").control_id,
            "CTRL123"
        );

        let second = output
            .next()
            .await
            .expect("second response should exist")
            .expect("second response should be OK");
        assert!(!second.success);
        assert_eq!(second.errors[0].code, "PARSE_ERROR");

        let third = output
            .next()
            .await
            .expect("third response should exist")
            .expect("third response should be OK");
        assert!(!third.success);
        assert_eq!(third.errors[0].code, "MLLP_ERROR");

        let fourth = output
            .next()
            .await
            .expect("fourth response should exist")
            .expect("fourth response should be OK");
        assert!(fourth.success);
        assert_eq!(
            fourth.metadata.expect("metadata should exist").control_id,
            "CTRL123"
        );

        assert!(output.next().await.is_none());
    }

    #[tokio::test]
    async fn test_grpc_parse_stream_reports_malformed_frames_as_status() {
        let service = service();
        let mut codec = ProstCodec::<ParseStreamResponse, ParseStreamRequest>::default();
        let stream = Streaming::new_request(
            codec.decoder(),
            Full::new(Bytes::from_static(&[0, 0, 0, 0, 10, b'x'])),
            None,
            None,
        );

        let response = service
            .parse_stream(Request::new(stream))
            .await
            .expect("ParseStream should start before decode errors");
        let mut output = response.into_inner();

        let err = output
            .next()
            .await
            .expect("malformed frame should emit a status")
            .expect_err("malformed frame should fail stream decoding");
        assert_eq!(err.code(), Code::Internal);

        assert!(output.next().await.is_none());
    }

    fn assert_no_phi(content: &str) {
        assert_no_phi_leak_sentinels("gRPC validate-redacted response", content);
    }

    fn is_sha256_hex(value: &str) -> bool {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }
}
