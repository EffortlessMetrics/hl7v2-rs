//! gRPC service implementation for HL7v2.

use crate::server::AppState;
use hl7v2_model::{
    Comp as RustComp, Field as RustField, Message as RustMessage, Rep as RustRep,
    Segment as RustSegment,
};
use hl7v2_parser::parse as rust_parse;
use std::sync::Arc;
use tonic::{Request, Response, Status};

// Include the generated gRPC code
/// Generated gRPC protocol code (protobuf messages and service traits).
#[allow(missing_docs)]
pub mod proto {
    tonic::include_proto!("hl7v2.v1");
}

use proto::hl7_service_server::Hl7Service;
use proto::*;

/// Implementation of the HL7Service gRPC trait
pub struct Hl7ServiceImpl {
    #[allow(dead_code)]
    state: Arc<AppState>,
}

impl Hl7ServiceImpl {
    /// Create a new gRPC service instance
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Hl7Service for Hl7ServiceImpl {
    async fn parse(
        &self,
        request: Request<ParseRequest>,
    ) -> Result<Response<ParseResponse>, Status> {
        let req = request.into_inner();

        let parse_result = if req.mllp_framed {
            match hl7v2_mllp::unwrap_mllp(&req.message) {
                Ok(hl7) => rust_parse(hl7),
                Err(e) => {
                    return Ok(Response::new(ParseResponse {
                        success: false,
                        message: None,
                        errors: vec![Error {
                            code: "MLLP_ERROR".to_string(),
                            message: format!("Failed to unwrap MLLP: {}", e),
                            details: std::collections::HashMap::new(),
                            trace_id: String::new(),
                        }],
                        metadata: None,
                    }));
                }
            }
        } else {
            rust_parse(&req.message)
        };

        match parse_result {
            Ok(msg) => {
                let metadata = MessageMetadata {
                    message_type: msg
                        .segments
                        .iter()
                        .find(|s| &s.id == b"MSH")
                        .and_then(|s: &RustSegment| s.fields.get(7))
                        .and_then(|f: &RustField| f.first_text())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    version: msg
                        .segments
                        .iter()
                        .find(|s| &s.id == b"MSH")
                        .and_then(|s: &RustSegment| s.fields.get(10))
                        .and_then(|f: &RustField| f.first_text())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    control_id: msg
                        .segments
                        .iter()
                        .find(|s| &s.id == b"MSH")
                        .and_then(|s: &RustSegment| s.fields.get(8))
                        .and_then(|f: &RustField| f.first_text())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    sending_facility: String::new(),
                    receiving_facility: String::new(),
                };

                let proto_msg = proto::Message::from(msg);

                Ok(Response::new(ParseResponse {
                    success: true,
                    message: Some(proto_msg),
                    errors: Vec::new(),
                    metadata: Some(metadata),
                }))
            }
            Err(e) => Ok(Response::new(ParseResponse {
                success: false,
                message: None,
                errors: vec![Error {
                    code: "PARSE_ERROR".to_string(),
                    message: format!("Failed to parse HL7: {}", e),
                    details: std::collections::HashMap::new(),
                    trace_id: String::new(),
                }],
                metadata: None,
            })),
        }
    }

    type ParseStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<ParseStreamResponse, Status>>;

    async fn parse_stream(
        &self,
        _request: Request<tonic::Streaming<ParseStreamRequest>>,
    ) -> Result<Response<Self::ParseStreamStream>, Status> {
        Err(Status::unimplemented("Streaming parse not yet implemented"))
    }

    async fn validate(
        &self,
        request: Request<ValidateRequest>,
    ) -> Result<Response<ValidateResponse>, Status> {
        let req = request.into_inner();

        let message = rust_parse(&req.message)
            .map_err(|e| Status::invalid_argument(format!("Failed to parse HL7: {}", e)))?;

        let profile = hl7v2_prof::load_profile(&req.profile)
            .map_err(|e| Status::invalid_argument(format!("Failed to load profile: {}", e)))?;

        let issues = hl7v2_prof::validate(&message, &profile);
        let valid = issues
            .iter()
            .all(|i| i.severity != hl7v2_prof::Severity::Error);

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for issue in issues {
            let location = issue.path.map(|p| {
                let mut loc = Location::default();
                let parts: Vec<&str> = p.split('.').collect();
                if !parts.is_empty() {
                    loc.segment = parts[0].to_string();
                }
                if parts.len() > 1 {
                    loc.field = parts[1].parse().unwrap_or(0);
                }
                if parts.len() > 2 {
                    loc.component = parts[2].parse().unwrap_or(0);
                }
                loc
            });

            let proto_issue = ValidationIssue {
                code: issue.code,
                message: issue.detail,
                severity: match issue.severity {
                    hl7v2_prof::Severity::Error => validation_issue::Severity::Error as i32,
                    hl7v2_prof::Severity::Warning => validation_issue::Severity::Warning as i32,
                },
                location,
                advice: String::new(),
            };

            if issue.severity == hl7v2_prof::Severity::Error {
                errors.push(proto_issue);
            } else {
                warnings.push(proto_issue);
            }
        }

        let summary = Some(ValidationSummary {
            error_count: errors.len() as i32,
            warning_count: warnings.len() as i32,
            info_count: 0,
        });

        Ok(Response::new(ValidateResponse {
            valid,
            errors,
            warnings,
            summary,
        }))
    }

    async fn generate_ack(
        &self,
        request: Request<GenerateAckRequest>,
    ) -> Result<Response<GenerateAckResponse>, Status> {
        let req = request.into_inner();

        let message = rust_parse(&req.message)
            .map_err(|e| Status::invalid_argument(format!("Failed to parse HL7: {}", e)))?;

        let ack_code = match req.code() {
            generate_ack_request::AckCode::Aa => hl7v2_gen::AckCode::AA,
            generate_ack_request::AckCode::Ae => hl7v2_gen::AckCode::AE,
            generate_ack_request::AckCode::Ar => hl7v2_gen::AckCode::AR,
            _ => hl7v2_gen::AckCode::AA,
        };

        let ack_msg = hl7v2_gen::ack(&message, ack_code)
            .map_err(|e| Status::internal(format!("Failed to generate ACK: {}", e)))?;
        let ack_bytes = hl7v2_writer::write(&ack_msg);
        let proto_ack = proto::Message::from(ack_msg);

        Ok(Response::new(GenerateAckResponse {
            ack_message: ack_bytes,
            parsed_ack: Some(proto_ack),
        }))
    }

    async fn normalize(
        &self,
        request: Request<NormalizeRequest>,
    ) -> Result<Response<NormalizeResponse>, Status> {
        let req = request.into_inner();
        let canonical = req
            .options
            .as_ref()
            .map(|o| o.canonical_delimiters)
            .unwrap_or(true);

        let normalized_bytes = hl7v2_normalize::normalize(&req.message, canonical)
            .map_err(|e| Status::invalid_argument(format!("Failed to normalize HL7: {}", e)))?;

        let mut final_bytes = normalized_bytes;
        if let Some(options) = req.options
            && options.mllp_frame
        {
            final_bytes = hl7v2_mllp::wrap_mllp(&final_bytes);
        }

        Ok(Response::new(NormalizeResponse {
            normalized: final_bytes,
        }))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = HealthCheckResponse {
            status: health_check_response::ServingStatus::Serving as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0,
        };
        Ok(Response::new(response))
    }
}

// ============================================================================
// Conversions
// ============================================================================

impl From<RustMessage> for proto::Message {
    fn from(msg: RustMessage) -> Self {
        proto::Message {
            delimiters: Some(proto::Delimiters {
                field: msg.delims.field.to_string(),
                component: msg.delims.comp.to_string(),
                repetition: msg.delims.rep.to_string(),
                escape: msg.delims.esc.to_string(),
                subcomponent: msg.delims.sub.to_string(),
            }),
            segments: msg.segments.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RustSegment> for proto::Segment {
    fn from(seg: RustSegment) -> Self {
        proto::Segment {
            id: String::from_utf8_lossy(&seg.id).to_string(),
            fields: seg.fields.into_iter().map(Into::into).collect(),
            sequence: 0,
        }
    }
}

impl From<RustField> for proto::Field {
    fn from(f: RustField) -> Self {
        let presence = if f.reps.is_empty() {
            proto::field::Presence::Missing as i32
        } else {
            proto::field::Presence::Value as i32
        };

        proto::Field {
            presence,
            value: f.first_text().map(String::from),
            repetitions: f.reps.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RustRep> for proto::Repetition {
    fn from(r: RustRep) -> Self {
        proto::Repetition {
            value: r.first_text().map(String::from),
            components: r.comps.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RustComp> for proto::Component {
    fn from(c: RustComp) -> Self {
        proto::Component {
            value: c.first_text().map(String::from),
            subcomponents: c
                .subs
                .into_iter()
                .filter_map(|a| match a {
                    hl7v2_model::Atom::Text(t) => Some(t),
                    _ => None,
                })
                .collect(),
        }
    }
}
