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
    use hl7v2_server::grpc::Hl7ServiceImpl;
    use hl7v2_server::grpc::proto::hl7_service_client::Hl7ServiceClient;
    use hl7v2_server::grpc::proto::hl7_service_server::Hl7Service;
    use hl7v2_server::grpc::proto::{
        GenerateAckRequest, HealthCheckRequest, NormalizeOptions, NormalizeRequest, ParseRequest,
        ParseStreamRequest, ParseStreamResponse, ValidateRedactedRequest, ValidateRequest,
        generate_ack_request, health_check_response, validation_issue,
    };
    use hl7v2_server::server::{AppState, ServerConfig};
    use hl7v2_test_utils::{
        PHI_LEAK_SENTINEL_MESSAGE as PHI_MESSAGE, PHI_LEAK_SENTINEL_POLICY as REDACTION_POLICY,
        assert_no_phi_leak_sentinels,
    };
    use http_body_util::Full;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use prost::Message as ProstMessage;
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;
    use tokio::net::TcpListener;
    use tokio::time::sleep;
    use tokio_stream::StreamExt;
    use tonic::codec::{Codec, ProstCodec, Streaming};
    use tonic::codegen::Bytes;
    use tonic::metadata::MetadataValue;
    use tonic::{Code, Request};

    /// Helper to create a mock AppState
    fn mock_state() -> Arc<AppState> {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(handle),
            api_key: None,
            cors_allowed_origins: Default::default(),
            readiness_checks: ServerConfig::default().readiness_checks(),
            bundle_output_root: None,
            ack_policy: Default::default(),
            quarantine: Default::default(),
        })
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

        let request = Request::new(ValidateRequest {
            message: SAMPLE_MSG.to_vec(),
            profile: "invalid: yaml: structure:".to_string(),
            mllp_framed: false,
            options: None,
            report_schema_version: 0,
        });

        let err = service
            .validate(request)
            .await
            .expect_err("Malformed profile should fail the RPC");

        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.message().contains("Failed to load profile"));
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
}
