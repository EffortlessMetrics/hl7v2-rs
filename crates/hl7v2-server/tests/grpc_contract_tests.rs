//! Contract tests for the gRPC service implementation.

#[cfg(test)]
mod tests {
    use hl7v2_server::grpc::Hl7ServiceImpl;
    use hl7v2_server::grpc::proto::hl7_service_server::Hl7Service;
    use hl7v2_server::grpc::proto::{
        GenerateAckRequest, HealthCheckRequest, NormalizeOptions, NormalizeRequest, ParseRequest,
        ParseStreamRequest, ParseStreamResponse, ValidateRequest, generate_ack_request,
        health_check_response, validation_issue,
    };
    use hl7v2_server::server::AppState;
    use http_body_util::Empty;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use std::sync::Arc;
    use std::time::Instant;
    use tonic::codec::{Codec, ProstCodec, Streaming};
    use tonic::codegen::Bytes;
    use tonic::{Code, Request};

    /// Helper to create a mock AppState
    fn mock_state() -> Arc<AppState> {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        Arc::new(AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(handle),
            api_key: None,
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

    fn service() -> Hl7ServiceImpl {
        Hl7ServiceImpl::new(mock_state())
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
        let mllp_msg = hl7v2_mllp::wrap_mllp(SAMPLE_MSG);

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
        assert_eq!(
            hl7v2_mllp::unwrap_mllp(&framed).unwrap(),
            unframed.as_slice()
        );
    }

    #[tokio::test]
    async fn test_grpc_validate_valid_profile() {
        let service = service();
        let request = Request::new(ValidateRequest {
            message: SAMPLE_MSG.to_vec(),
            profile: PROFILE.to_string(),
            mllp_framed: false,
            options: None,
        });

        let response = service.validate(request).await.expect("RPC should succeed");
        let inner = response.into_inner();

        assert!(inner.valid);
        assert!(inner.errors.is_empty());
        assert!(inner.warnings.is_empty());
        let summary = inner.summary.expect("Summary should exist");
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
    }

    #[tokio::test]
    async fn test_grpc_validate_invalid_profile_returns_invalid_argument() {
        let service = service();

        let request = Request::new(ValidateRequest {
            message: SAMPLE_MSG.to_vec(),
            profile: "invalid: yaml: structure:".to_string(),
            mllp_framed: false,
            options: None,
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
    async fn test_grpc_parse_stream_is_explicitly_unsupported() {
        let service = service();
        let mut codec = ProstCodec::<ParseStreamResponse, ParseStreamRequest>::default();
        let stream = Streaming::new_request(codec.decoder(), Empty::<Bytes>::new(), None, None);

        let err = service
            .parse_stream(Request::new(stream))
            .await
            .expect_err("ParseStream should be explicitly unsupported");

        assert_eq!(err.code(), Code::Unimplemented);
        assert_eq!(err.message(), "Streaming parse not yet implemented");
    }
}
