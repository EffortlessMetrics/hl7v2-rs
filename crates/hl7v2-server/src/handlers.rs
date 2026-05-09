//! HTTP request handlers for HL7v2 endpoints.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::audit::{self, MessageLogContext};
use crate::models::*;
use crate::redaction::redact_message;
use crate::server::AppState;

/// Handler for GET /health
pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();

    let response = HealthResponse {
        status: HealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    };

    (StatusCode::OK, Json(response))
}

/// Handler for GET /ready
pub async fn ready_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = state.ready_response();
    let status = if response.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(response))
}

/// Handler for POST /hl7/parse
pub async fn parse_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ParseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message(request.message.as_bytes(), request.mllp_framed)?;

    // Extract metadata
    let metadata = extract_metadata(&message)?;
    let log_context = MessageLogContext::from_message(&message);

    // Optionally convert to JSON
    let message_json = if request.options.include_json {
        Some(hl7v2::to_json(&message))
    } else {
        None
    };

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_PARSE,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        segment_count = metadata.segment_count,
        include_json = request.options.include_json,
        "parsed HL7 message"
    );

    let response = ParseResponse {
        message: message_json,
        metadata,
        warnings: Vec::new(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/validate
pub async fn validate_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ValidateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version = requested_report_schema_version(request.report_schema_version)?;
    let message = parse_request_message(request.message.as_bytes(), request.mllp_framed)?;

    // Extract metadata
    let metadata = extract_metadata(&message)?;
    let log_context = MessageLogContext::from_message(&message);

    // Load the profile before validation. Profile load failures are client
    // errors, not successful validation results.
    let profile = hl7v2::load_profile_checked(&request.profile)
        .map_err(|e| AppError::ProfileLoad(e.to_string()))?;

    // Perform validation using the profile
    let issues = hl7v2::validate(&message, &profile);
    let report = hl7v2::ValidationReport::from_issues(
        &message,
        Some(profile.message_structure.clone()),
        issues.clone(),
    );
    let validation_report_v2 = (report_schema_version == 2)
        .then(|| validation_report_v2_for_server(&report, &request.profile, &profile));

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_VALIDATE,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        profile = %profile.message_structure,
        validation_status = audit::validation_status(report.valid),
        valid = report.valid,
        issue_count = report.issue_count,
        report_schema_version,
        "validated HL7 message"
    );

    // Preserve legacy error/warning arrays while exposing the shared report issues.
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for issue in issues {
        let severity = match issue.severity {
            hl7v2::Severity::Error => ErrorSeverity::Error,
            hl7v2::Severity::Warning => ErrorSeverity::Warning,
        };

        let validation_item = ValidationError {
            code: issue.code,
            message: issue.detail,
            location: issue.path,
            severity,
        };

        match issue.severity {
            hl7v2::Severity::Error => errors.push(validation_item),
            hl7v2::Severity::Warning => {
                warnings.push(ValidationWarning {
                    code: validation_item.code,
                    message: validation_item.message,
                    location: validation_item.location,
                });
            }
        }
    }

    let response = ValidateResponse {
        valid: report.valid,
        message_type: report.message_type,
        profile: report.profile,
        segment_count: report.segment_count,
        issue_count: report.issue_count,
        issues: report.issues,
        validation_report_v2,
        errors,
        warnings,
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/validate-redacted
pub async fn validate_redacted_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ValidateRedactedRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version = requested_report_schema_version(request.report_schema_version)?;
    let redaction_receipt_schema_version =
        requested_redaction_receipt_schema_version(request.redaction_receipt_schema_version)?;
    let quarantine_schema_version =
        requested_quarantine_schema_version(request.quarantine_schema_version)?;
    let raw_input = request.message.into_bytes();
    let mut message = parse_request_message(&raw_input, request.mllp_framed)?;
    let log_context = MessageLogContext::from_message(&message);
    let receipt =
        redact_message(&mut message, &request.redaction_policy).map_err(AppError::Redaction)?;
    let redacted_hl7 = String::from_utf8(hl7v2::write(&message))
        .map_err(|error| AppError::Internal(format!("redacted message was not UTF-8: {error}")))?;

    let profile = hl7v2::load_profile_checked(&request.profile)
        .map_err(|e| AppError::ProfileLoad(e.to_string()))?;
    let issues = hl7v2::validate(&message, &profile);
    let validation_report = hl7v2::ValidationReport::from_issues(
        &message,
        Some(profile.message_structure.clone()),
        issues,
    );
    let validation_report_v2 = (report_schema_version == 2)
        .then(|| validation_report_v2_for_server(&validation_report, &request.profile, &profile));
    let redaction_receipt_v2 = (redaction_receipt_schema_version == 2).then(|| receipt.to_v2());
    let quarantine = maybe_write_redacted_quarantine(RedactedQuarantineContext {
        state: &state,
        raw_input: &raw_input,
        profile_yaml: &request.profile,
        policy_text: &request.redaction_policy,
        redacted_message: &message,
        redacted_hl7: &redacted_hl7,
        redaction_receipt: &receipt,
        validation_report: &validation_report,
    })?;
    let quarantine_v2 = if quarantine_schema_version == 2 {
        quarantine
            .as_ref()
            .map(|summary| summary.to_v2("hl7v2-server", env!("CARGO_PKG_VERSION")))
    } else {
        None
    };
    let quarantine_output_id = quarantine
        .as_ref()
        .map_or("none", |summary| summary.output_dir.as_str());

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_VALIDATE_REDACTED,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        profile = %profile.message_structure,
        validation_status = audit::validation_status(validation_report.valid),
        valid = validation_report.valid,
        issue_count = validation_report.issue_count,
        redaction_status = audit::redaction_status(receipt.phi_removed),
        redaction_phi_removed = receipt.phi_removed,
        quarantine_output_id,
        include_redacted_hl7 = request.include_redacted_hl7,
        report_schema_version,
        redaction_receipt_schema_version,
        quarantine_schema_version,
        "validated redacted HL7 message"
    );

    let response = ValidateRedactedResponse {
        validation_report,
        validation_report_v2,
        redaction_receipt: receipt,
        redaction_receipt_v2,
        quarantine,
        quarantine_v2,
        redacted_hl7: request.include_redacted_hl7.then_some(redacted_hl7),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/bundle
pub async fn bundle_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BundleRequest>,
) -> Result<impl IntoResponse, AppError> {
    let artifact_schema_version =
        requested_bundle_artifact_schema_version(request.bundle_artifact_schema_version)?;
    let bundle_output_root = state
        .bundle_output_root
        .as_deref()
        .ok_or(AppError::BundleOutputNotConfigured)?;

    let raw_input = request.message.into_bytes();
    let mut message = parse_request_message(&raw_input, request.mllp_framed)?;
    let log_context = MessageLogContext::from_message(&message);
    let receipt =
        redact_message(&mut message, &request.redaction_policy).map_err(AppError::Redaction)?;
    let redacted_hl7 = String::from_utf8(hl7v2::write(&message))
        .map_err(|error| AppError::Internal(format!("redacted message was not UTF-8: {error}")))?;

    let profile = hl7v2::load_profile_checked(&request.profile)
        .map_err(|e| AppError::ProfileLoad(e.to_string()))?;
    let issues = hl7v2::validate(&message, &profile);
    let validation_report =
        hl7v2::ValidationReport::from_issues(&message, Some("profile.yaml".to_string()), issues);

    let summary =
        crate::evidence::write_evidence_bundle(crate::evidence::EvidenceBundleWriteRequest {
            root: bundle_output_root,
            bundle_id: &request.bundle_id,
            raw_input: &raw_input,
            profile_yaml: &request.profile,
            policy_text: &request.redaction_policy,
            redacted_message: &message,
            redacted_hl7: &redacted_hl7,
            redaction_receipt: &receipt,
            validation_report: &validation_report,
            artifact_schema_version,
        })
        .map_err(AppError::from)?;

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_BUNDLE,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        profile = %profile.message_structure,
        validation_status = audit::validation_status(summary.validation_valid),
        valid = summary.validation_valid,
        issue_count = summary.validation_issue_count,
        redaction_status = audit::redaction_status(summary.redaction_phi_removed),
        redaction_phi_removed = summary.redaction_phi_removed,
        bundle_id_hash = %audit::hash_identifier(&summary.output_dir),
        artifact_count = summary.artifacts.len(),
        artifact_schema_version,
        "wrote redacted evidence bundle"
    );

    Ok((StatusCode::CREATED, Json(summary)))
}

/// Handler for POST /hl7/replay
pub async fn replay_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReplayRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version =
        requested_replay_report_schema_version(request.replay_report_schema_version)?;
    let bundle_output_root = state
        .bundle_output_root
        .as_deref()
        .ok_or(AppError::BundleOutputNotConfigured)?;
    let bundle_dir = crate::evidence::bundle_path_for_id(bundle_output_root, &request.bundle_id)
        .map_err(AppError::from)?;

    if !bundle_dir.is_dir() {
        return Err(AppError::BundleNotFound(format!(
            "bundle_id '{}' was not found",
            request.bundle_id
        )));
    }

    let report = hl7v2::evidence::replay_evidence_bundle(&bundle_dir, "hl7v2-server");
    let message_type = report.message_type.as_deref().unwrap_or("unknown");
    let validation_status = report
        .validation_valid
        .map_or("not_available", audit::validation_status);
    let validation_issue_count = report.validation_issue_count.unwrap_or(0);

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_REPLAY,
        message_type,
        bundle_id_hash = %audit::hash_identifier(&request.bundle_id),
        reproduced = report.reproduced,
        validation_status,
        issue_count = validation_issue_count,
        check_count = report.checks.len(),
        replay_report_schema_version = report_schema_version,
        "replayed evidence bundle"
    );

    let response = if report_schema_version == 2 {
        serde_json::to_value(report.to_v2())
    } else {
        serde_json::to_value(report)
    }
    .map_err(|error| AppError::Internal(format!("could not serialize replay report: {error}")))?;

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/corpus/summarize
pub async fn corpus_summarize_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CorpusSummaryRequest>,
) -> Result<impl IntoResponse, AppError> {
    let schema_version = requested_corpus_summary_schema_version(request.summary_schema_version)?;
    let ids = validated_corpus_message_ids(&request.messages, "messages", "message")?;
    let messages = corpus_message_refs(&request.messages, &ids);
    let summary = hl7v2::synthetic::corpus::summarize_corpus_messages("<inline-corpus>", &messages);
    let response = if schema_version == 2 {
        serde_json::to_value(summary.to_v2("hl7v2-server", env!("CARGO_PKG_VERSION")))
    } else {
        serde_json::to_value(summary)
    }
    .map_err(|error| AppError::Internal(format!("could not serialize corpus summary: {error}")))?;

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/corpus/fingerprint
pub async fn corpus_fingerprint_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CorpusFingerprintRequest>,
) -> Result<impl IntoResponse, AppError> {
    let schema_version =
        requested_corpus_fingerprint_schema_version(request.fingerprint_schema_version)?;
    let ids = validated_corpus_message_ids(&request.messages, "messages", "message")?;
    let messages = corpus_message_refs(&request.messages, &ids);
    let mut fingerprint =
        hl7v2::synthetic::corpus::fingerprint_corpus_messages("<inline-corpus>", &messages);

    if let Some(profile_yaml) = request.profile.as_deref() {
        attach_profile_to_fingerprint(&mut fingerprint, profile_yaml, &request.messages)?;
    }

    let response = if schema_version == 2 {
        serde_json::to_value(fingerprint.to_v2("hl7v2-server"))
    } else {
        serde_json::to_value(fingerprint)
    }
    .map_err(|error| {
        AppError::Internal(format!("could not serialize corpus fingerprint: {error}"))
    })?;

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/corpus/diff
pub async fn corpus_diff_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CorpusDiffRequest>,
) -> Result<impl IntoResponse, AppError> {
    let schema_version = requested_corpus_diff_schema_version(request.diff_schema_version)?;
    let before_ids = validated_corpus_message_ids(&request.before, "before", "before")?;
    let after_ids = validated_corpus_message_ids(&request.after, "after", "after")?;
    let before_messages = corpus_message_refs(&request.before, &before_ids);
    let after_messages = corpus_message_refs(&request.after, &after_ids);
    let mut before_fingerprint =
        hl7v2::synthetic::corpus::fingerprint_corpus_messages("<inline-before>", &before_messages);
    let mut after_fingerprint =
        hl7v2::synthetic::corpus::fingerprint_corpus_messages("<inline-after>", &after_messages);

    if let Some(profile_yaml) = request.profile.as_deref() {
        let profile_metadata =
            attach_profile_to_fingerprint(&mut before_fingerprint, profile_yaml, &request.before)?;
        after_fingerprint.profile = Some(profile_metadata);
        after_fingerprint.validation_issue_code_counts =
            validation_issue_counts_for_messages(&request.after, profile_yaml)?;
    }

    let diff =
        hl7v2::synthetic::corpus::diff_corpus_fingerprints(&before_fingerprint, &after_fingerprint);
    let response = if schema_version == 2 {
        serde_json::to_value(diff.to_v2("hl7v2-server"))
    } else {
        serde_json::to_value(diff)
    }
    .map_err(|error| AppError::Internal(format!("could not serialize corpus diff: {error}")))?;

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/ack
pub async fn ack_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<AckRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message(request.message.as_bytes(), request.mllp_framed)?;
    let log_context = MessageLogContext::from_message(&message);
    let ack_code = map_ack_code(request.code);

    let ack_message = if let Some(error_message) = request.error_message.as_deref() {
        hl7v2::ack_with_error(&message, ack_code, Some(error_message))
    } else {
        hl7v2::ack(&message, ack_code)
    }
    .map_err(|e| AppError::Internal(format!("Failed to generate ACK: {}", e)))?;

    let metadata = extract_metadata(&ack_message)?;
    let ack_bytes = hl7v2::write(&ack_message);
    let ack_bytes = if request.mllp_frame {
        hl7v2::wrap_mllp(&ack_bytes)
    } else {
        ack_bytes
    };

    let response = AckResponse {
        ack_message: String::from_utf8(ack_bytes)
            .map_err(|e| AppError::Internal(format!("ACK was not UTF-8: {}", e)))?,
        ack_code: request.code.as_str().to_string(),
        metadata,
    };

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_ACK,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        ack_code = request.code.as_str(),
        mllp_frame = request.mllp_frame,
        "generated ACK"
    );

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/ack-policy
pub async fn ack_policy_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AckPolicyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let raw_input = request.message.into_bytes();
    let policy = &state.ack_policy;

    let (message, validation_report, decision) =
        match parse_request_message(&raw_input, request.mllp_framed) {
            Ok(message) => {
                let profile = hl7v2::load_profile_checked(&request.profile)
                    .map_err(|e| AppError::ProfileLoad(e.to_string()))?;
                let issues = hl7v2::validate(&message, &profile);
                let report = hl7v2::ValidationReport::from_issues(
                    &message,
                    Some(profile.message_structure.clone()),
                    issues,
                );
                let decision = ack_policy_decision_for_validation(policy, &report)?;
                (message, Some(report), decision)
            }
            Err(error) if policy.rejects(AckPolicyRejectCondition::ParseError) => {
                let message = parse_msh_for_ack_policy(&raw_input, request.mllp_framed)
                    .map_err(|_fallback_error| error)?;
                let decision = ack_policy_reject_decision(policy, AckPolicyReason::ParseError, 0);
                (message, None, decision)
            }
            Err(error) => return Err(error),
        };
    let log_context = MessageLogContext::from_message(&message);

    let ack_code = ack_code_from_policy_decision(&decision)?;
    let ack_message = if let Some(error_text) = decision.error_text.as_deref() {
        hl7v2::ack_with_error(&message, ack_code, Some(error_text))
    } else {
        hl7v2::ack(&message, ack_code)
    }
    .map_err(|e| AppError::Internal(format!("Failed to generate policy ACK: {}", e)))?;

    let metadata = extract_metadata(&ack_message)?;
    let ack_bytes = hl7v2::write(&ack_message);
    let ack_bytes = if request.mllp_frame {
        hl7v2::wrap_mllp(&ack_bytes)
    } else {
        ack_bytes
    };

    let validation_status = validation_report.as_ref().map_or("parse_error", |report| {
        audit::validation_status(report.valid)
    });
    let issue_count = validation_report
        .as_ref()
        .map_or(0, |report| report.issue_count);

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_ACK_POLICY,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        validation_status,
        issue_count,
        ack_outcome = audit::ack_outcome_label(decision.outcome),
        ack_reason = audit::ack_reason_label(decision.reason),
        ack_code = %decision.ack_code,
        include_error_text = decision.include_error_text,
        mllp_frame = request.mllp_frame,
        "generated policy ACK decision"
    );

    let response = AckPolicyResponse {
        ack_message: String::from_utf8(ack_bytes)
            .map_err(|e| AppError::Internal(format!("ACK was not UTF-8: {}", e)))?,
        ack_code: decision.ack_code.clone(),
        decision,
        validation_report,
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/normalize
pub async fn normalize_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<NormalizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message_bytes = request.message.as_bytes();
    let input = if request.mllp_framed {
        hl7v2::unwrap_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))?
    } else {
        message_bytes
    };

    let normalized_bytes = hl7v2::normalize(input, request.options.canonical_delimiters)
        .map_err(|e| AppError::Parse(format!("Normalize error: {}", e)))?;
    let normalized_message = hl7v2::parse(&normalized_bytes)
        .map_err(|e| AppError::Parse(format!("Normalized message parse error: {}", e)))?;
    let metadata = extract_metadata(&normalized_message)?;
    let log_context = MessageLogContext::from_message(&normalized_message);

    let response_bytes = if request.options.mllp_frame {
        hl7v2::wrap_mllp(&normalized_bytes)
    } else {
        normalized_bytes
    };

    let response = NormalizeResponse {
        normalized_message: String::from_utf8(response_bytes)
            .map_err(|e| AppError::Internal(format!("Normalized message was not UTF-8: {}", e)))?,
        metadata,
    };

    tracing::info!(
        target: "hl7v2_server::evidence",
        event = audit::EVENT_NORMALIZE,
        message_type = %log_context.message_type,
        message_control_id_hash = %log_context.message_control_id_hash,
        correlation_id = %log_context.correlation_id,
        canonical_delimiters = request.options.canonical_delimiters,
        mllp_frame = request.options.mllp_frame,
        "normalized HL7 message"
    );

    Ok((StatusCode::OK, Json(response)))
}

fn parse_request_message(
    message_bytes: &[u8],
    mllp_framed: bool,
) -> Result<hl7v2::Message, AppError> {
    if mllp_framed {
        hl7v2::parse_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))
    } else {
        hl7v2::parse(message_bytes).map_err(|e| AppError::Parse(format!("Parse error: {}", e)))
    }
}

fn requested_report_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported validation report schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_redaction_receipt_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported redaction receipt schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_quarantine_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported quarantine output schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_bundle_artifact_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported bundle artifact schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_replay_report_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported replay report schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_corpus_summary_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported corpus summary schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_corpus_fingerprint_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported corpus fingerprint schema version {other}; expected 1 or 2"
        ))),
    }
}

fn requested_corpus_diff_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported corpus diff schema version {other}; expected 1 or 2"
        ))),
    }
}

fn validated_corpus_message_ids(
    messages: &[CorpusMessageInput],
    field_name: &str,
    default_prefix: &str,
) -> Result<Vec<String>, AppError> {
    if messages.is_empty() {
        return Err(AppError::Validation(format!(
            "{field_name} must contain at least one message"
        )));
    }

    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let label = message
                .id
                .clone()
                .unwrap_or_else(|| format!("{default_prefix}-{}", index.saturating_add(1)));
            validate_corpus_message_id(&label)?;
            Ok(label)
        })
        .collect()
}

fn validate_corpus_message_id(label: &str) -> Result<(), AppError> {
    if label.is_empty() || label == "." || label == ".." || label.len() > 128 {
        return Err(AppError::Validation(
            "corpus message id must be 1-128 characters and cannot be '.' or '..'".to_string(),
        ));
    }

    if !label
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(AppError::Validation(
            "corpus message id must use only ASCII letters, numbers, '.', '_' or '-'".to_string(),
        ));
    }

    Ok(())
}

fn corpus_message_refs<'a>(
    messages: &'a [CorpusMessageInput],
    ids: &'a [String],
) -> Vec<hl7v2::synthetic::corpus::CorpusMessageRef<'a>> {
    messages
        .iter()
        .zip(ids.iter())
        .map(|(message, id)| {
            hl7v2::synthetic::corpus::CorpusMessageRef::new(id.as_str(), message.message.as_bytes())
        })
        .collect()
}

fn attach_profile_to_fingerprint(
    fingerprint: &mut hl7v2::synthetic::corpus::CorpusFingerprint,
    profile_yaml: &str,
    messages: &[CorpusMessageInput],
) -> Result<hl7v2::synthetic::corpus::CorpusFingerprintProfile, AppError> {
    let profile = hl7v2::load_profile_checked(profile_yaml)
        .map_err(|error| AppError::ProfileLoad(error.to_string()))?;
    let metadata = hl7v2::synthetic::corpus::CorpusFingerprintProfile {
        path: "<inline-profile>".to_string(),
        sha256: compute_sha256(profile_yaml),
        version: profile.version.clone(),
        message_structure: profile.message_structure.clone(),
    };
    fingerprint.profile = Some(metadata.clone());
    fingerprint.validation_issue_code_counts =
        validation_issue_counts_for_loaded_profile(messages, &profile);
    Ok(metadata)
}

fn validation_issue_counts_for_messages(
    messages: &[CorpusMessageInput],
    profile_yaml: &str,
) -> Result<Vec<hl7v2::synthetic::corpus::CorpusCount>, AppError> {
    let profile = hl7v2::load_profile_checked(profile_yaml)
        .map_err(|error| AppError::ProfileLoad(error.to_string()))?;
    Ok(validation_issue_counts_for_loaded_profile(
        messages, &profile,
    ))
}

fn validation_issue_counts_for_loaded_profile(
    messages: &[CorpusMessageInput],
    profile: &hl7v2::Profile,
) -> Vec<hl7v2::synthetic::corpus::CorpusCount> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for message in messages {
        let parsed = if hl7v2::is_mllp_framed(message.message.as_bytes()) {
            hl7v2::parse_mllp(message.message.as_bytes())
        } else {
            hl7v2::parse(message.message.as_bytes())
        };
        let Ok(parsed) = parsed else {
            continue;
        };

        let issues = hl7v2::validate(&parsed, profile);
        let report = hl7v2::ValidationReport::from_issues(
            &parsed,
            Some(profile.message_structure.clone()),
            issues,
        );
        for issue in report.issues {
            let count = counts.entry(issue.code).or_insert(0);
            *count = count.saturating_add(1);
        }
    }

    counts
        .into_iter()
        .map(|(value, count)| hl7v2::synthetic::corpus::CorpusCount { value, count })
        .collect()
}

fn validation_report_v2_for_server(
    report: &hl7v2::ValidationReport,
    profile_yaml: &str,
    profile: &hl7v2::Profile,
) -> hl7v2::ValidationReportV2 {
    report.to_v2(
        "hl7v2-server",
        env!("CARGO_PKG_VERSION"),
        Some(hl7v2::ValidationReportProfileIdentity {
            label: profile.message_structure.clone(),
            message_structure: Some(profile.message_structure.clone()),
            version: Some(profile.version.clone()),
            sha256: Some(compute_sha256(profile_yaml)),
        }),
    )
}

fn compute_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn parse_msh_for_ack_policy(
    message_bytes: &[u8],
    mllp_framed: bool,
) -> Result<hl7v2::Message, AppError> {
    let input = if mllp_framed {
        hl7v2::unwrap_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))?
    } else {
        message_bytes
    };

    let first_segment_end = input
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .unwrap_or(input.len());
    let msh = &input[..first_segment_end];
    if !msh.starts_with(b"MSH") {
        return Err(AppError::Parse(
            "Parse error: message did not contain a usable MSH segment".to_string(),
        ));
    }

    let mut buffer = msh.to_vec();
    buffer.push(b'\r');
    hl7v2::parse(&buffer)
        .map_err(|e| AppError::Parse(format!("MSH parse error for ACK policy: {}", e)))
}

fn map_ack_code(code: AckRequestCode) -> hl7v2::AckCode {
    match code {
        AckRequestCode::Aa => hl7v2::AckCode::AA,
        AckRequestCode::Ae => hl7v2::AckCode::AE,
        AckRequestCode::Ar => hl7v2::AckCode::AR,
        AckRequestCode::Ca => hl7v2::AckCode::CA,
        AckRequestCode::Ce => hl7v2::AckCode::CE,
        AckRequestCode::Cr => hl7v2::AckCode::CR,
    }
}

fn ack_policy_decision_for_validation(
    policy: &AckPolicyConfig,
    report: &hl7v2::ValidationReport,
) -> Result<AckPolicyDecision, AppError> {
    if report.valid && policy.accept_on == AckPolicyAcceptOn::Valid {
        let ack_code = match policy.mode {
            AckPolicyMode::Original => AckRequestCode::Aa,
            AckPolicyMode::Enhanced => AckRequestCode::Ca,
        };
        return Ok(AckPolicyDecision {
            mode: policy.mode,
            outcome: AckPolicyOutcome::Accepted,
            reason: AckPolicyReason::Valid,
            ack_code: ack_code.as_str().to_string(),
            include_error_text: false,
            error_text: None,
        });
    }

    if policy.rejects(AckPolicyRejectCondition::ValidationError) {
        return Ok(ack_policy_reject_decision(
            policy,
            AckPolicyReason::ValidationError,
            report.issue_count,
        ));
    }

    Err(AppError::Validation(
        "ACK policy did not define a decision for validation failure".to_string(),
    ))
}

fn ack_policy_reject_decision(
    policy: &AckPolicyConfig,
    reason: AckPolicyReason,
    issue_count: usize,
) -> AckPolicyDecision {
    let ack_code = match policy.mode {
        AckPolicyMode::Original => AckRequestCode::Ar,
        AckPolicyMode::Enhanced => AckRequestCode::Cr,
    };
    let error_text = policy
        .include_error_text
        .then(|| ack_policy_error_text(reason, issue_count));

    AckPolicyDecision {
        mode: policy.mode,
        outcome: AckPolicyOutcome::Rejected,
        reason,
        ack_code: ack_code.as_str().to_string(),
        include_error_text: policy.include_error_text,
        error_text,
    }
}

fn ack_policy_error_text(reason: AckPolicyReason, issue_count: usize) -> String {
    match reason {
        AckPolicyReason::Valid => "message accepted".to_string(),
        AckPolicyReason::ParseError => "message parsing failed".to_string(),
        AckPolicyReason::ValidationError => {
            format!("message validation failed with {issue_count} issue(s)")
        }
    }
}

fn ack_code_from_policy_decision(decision: &AckPolicyDecision) -> Result<hl7v2::AckCode, AppError> {
    match decision.ack_code.as_str() {
        "AA" => Ok(hl7v2::AckCode::AA),
        "AR" => Ok(hl7v2::AckCode::AR),
        "CA" => Ok(hl7v2::AckCode::CA),
        "CR" => Ok(hl7v2::AckCode::CR),
        code => Err(AppError::Internal(format!(
            "ACK policy produced unsupported ACK code: {code}"
        ))),
    }
}

struct RedactedQuarantineContext<'a> {
    state: &'a AppState,
    raw_input: &'a [u8],
    profile_yaml: &'a str,
    policy_text: &'a str,
    redacted_message: &'a hl7v2::Message,
    redacted_hl7: &'a str,
    redaction_receipt: &'a RedactionReceipt,
    validation_report: &'a hl7v2::ValidationReport,
}

fn maybe_write_redacted_quarantine(
    context: RedactedQuarantineContext<'_>,
) -> Result<Option<QuarantineOutputSummary>, AppError> {
    let RedactedQuarantineContext {
        state,
        raw_input,
        profile_yaml,
        policy_text,
        redacted_message,
        redacted_hl7,
        redaction_receipt,
        validation_report,
    } = context;

    if validation_report.valid || !state.quarantine.enabled {
        return Ok(None);
    }

    let root = state
        .quarantine
        .path
        .as_deref()
        .ok_or(AppError::QuarantineOutputNotConfigured)?;
    let output_id = generated_quarantine_id();
    let summary =
        crate::evidence::write_quarantine_output(crate::evidence::QuarantineOutputWriteRequest {
            root,
            output_id: &output_id,
            config: &state.quarantine,
            raw_input,
            profile_yaml,
            policy_text,
            redacted_message,
            redacted_hl7,
            redaction_receipt,
            validation_report,
        })
        .map_err(AppError::quarantine_from_evidence_error)?;

    Ok(Some(summary))
}

fn generated_quarantine_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("quarantine-{}-{nanos}", std::process::id())
}

/// Extract message metadata from parsed message
fn extract_metadata(message: &hl7v2::Message) -> Result<MessageMetadata, AppError> {
    // Find MSH segment
    let msh = message
        .segments
        .first()
        .ok_or_else(|| AppError::Parse("Missing MSH segment".to_string()))?;

    if &msh.id != b"MSH" {
        return Err(AppError::Parse("First segment must be MSH".to_string()));
    }

    // Extract MSH fields
    let message_type = joined_components(message, "MSH.9").unwrap_or_else(|| "UNKNOWN".to_string());

    let version = hl7v2::get(message, "MSH.12").unwrap_or("2.5").to_string();

    let sending_application = hl7v2::get(message, "MSH.3").unwrap_or("").to_string();

    let sending_facility = hl7v2::get(message, "MSH.4").unwrap_or("").to_string();

    let message_control_id = hl7v2::get(message, "MSH.10").unwrap_or("").to_string();

    Ok(MessageMetadata {
        message_type,
        version,
        sending_application,
        sending_facility,
        message_control_id,
        segment_count: message.segments.len(),
        charsets: message.charsets.clone(),
    })
}

fn joined_components(message: &hl7v2::Message, path: &str) -> Option<String> {
    let mut components = Vec::new();

    for index in 1.. {
        let component_path = format!("{}.{}", path, index);
        match hl7v2::get(message, &component_path) {
            Some(value) if !value.is_empty() => components.push(value.to_string()),
            Some(_) => {}
            None => break,
        }
    }

    if components.is_empty() {
        hl7v2::get(message, path).map(str::to_string)
    } else {
        Some(components.join("^"))
    }
}

/// Application error type with specific error variants.
///
/// This enum provides detailed error information for different failure modes,
/// making it easier to diagnose issues and provide meaningful error responses.
#[derive(Debug)]
pub enum AppError {
    /// Message parsing error (malformed HL7, invalid structure, etc.)
    Parse(String),

    /// Profile loading error (YAML syntax, missing fields, etc.)
    ProfileLoad(String),

    /// Validation error (message does not conform to profile)
    Validation(String),

    /// Redaction policy or redaction application error
    Redaction(String),

    /// Bundle output is not configured on the server
    BundleOutputNotConfigured,

    /// Bundle output root is configured but not writable or available
    BundleOutputNotReady(String),

    /// Bundle request or write error
    Bundle(String),

    /// Bundle output already exists
    Conflict(String),

    /// Requested bundle id was not found under the configured output root
    BundleNotFound(String),

    /// Quarantine output is enabled but no path is configured
    QuarantineOutputNotConfigured,

    /// Quarantine output root is configured but not writable or available
    QuarantineOutputNotReady(String),

    /// Quarantine output write error
    Quarantine(String),

    /// Quarantine output already exists
    QuarantineConflict(String),

    /// Internal server error (unexpected failures)
    Internal(String),
}

impl From<crate::evidence::EvidenceBundleError> for AppError {
    fn from(error: crate::evidence::EvidenceBundleError) -> Self {
        match error {
            crate::evidence::EvidenceBundleError::InvalidRequest(message) => Self::Bundle(message),
            crate::evidence::EvidenceBundleError::Conflict(message) => Self::Conflict(message),
            crate::evidence::EvidenceBundleError::Io(message) => {
                Self::BundleOutputNotReady(message)
            }
        }
    }
}

impl AppError {
    fn quarantine_from_evidence_error(error: crate::evidence::EvidenceBundleError) -> Self {
        match error {
            crate::evidence::EvidenceBundleError::InvalidRequest(message) => {
                Self::Quarantine(message)
            }
            crate::evidence::EvidenceBundleError::Conflict(message) => {
                Self::QuarantineConflict(message)
            }
            crate::evidence::EvidenceBundleError::Io(message) => {
                Self::QuarantineOutputNotReady(message)
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Parse(msg) => (StatusCode::BAD_REQUEST, "PARSE_ERROR", msg),
            // Profile load error is a client error since the profile is provided in the request
            AppError::ProfileLoad(msg) => (StatusCode::BAD_REQUEST, "PROFILE_LOAD_ERROR", msg),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            AppError::Redaction(msg) => (StatusCode::BAD_REQUEST, "REDACTION_ERROR", msg),
            AppError::BundleOutputNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "BUNDLE_OUTPUT_NOT_CONFIGURED",
                "server bundle output root is not configured".to_string(),
            ),
            AppError::BundleOutputNotReady(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "BUNDLE_OUTPUT_NOT_READY",
                msg,
            ),
            AppError::Bundle(msg) => (StatusCode::BAD_REQUEST, "BUNDLE_ERROR", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "BUNDLE_EXISTS", msg),
            AppError::BundleNotFound(msg) => (StatusCode::NOT_FOUND, "BUNDLE_NOT_FOUND", msg),
            AppError::QuarantineOutputNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "QUARANTINE_OUTPUT_NOT_CONFIGURED",
                "server quarantine output is enabled but no path is configured".to_string(),
            ),
            AppError::QuarantineOutputNotReady(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "QUARANTINE_OUTPUT_NOT_READY",
                msg,
            ),
            AppError::Quarantine(msg) => (StatusCode::BAD_REQUEST, "QUARANTINE_ERROR", msg),
            AppError::QuarantineConflict(msg) => (StatusCode::CONFLICT, "QUARANTINE_EXISTS", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        tracing::warn!(
            target: "hl7v2_server::evidence",
            event = audit::EVENT_ERROR,
            status = status.as_u16(),
            error_code = code,
            "request failed"
        );

        let error = ErrorResponse::new(code, message);
        (status, Json(error)).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Parse(msg) => write!(f, "Parse error: {}", msg),
            AppError::ProfileLoad(msg) => write!(f, "Profile load error: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::Redaction(msg) => write!(f, "Redaction error: {}", msg),
            AppError::BundleOutputNotConfigured => {
                write!(f, "Bundle output root is not configured")
            }
            AppError::BundleOutputNotReady(msg) => {
                write!(f, "Bundle output root is not ready: {}", msg)
            }
            AppError::Bundle(msg) => write!(f, "Bundle error: {}", msg),
            AppError::Conflict(msg) => write!(f, "Bundle conflict: {}", msg),
            AppError::BundleNotFound(msg) => write!(f, "Bundle not found: {}", msg),
            AppError::QuarantineOutputNotConfigured => {
                write!(f, "Quarantine output path is not configured")
            }
            AppError::QuarantineOutputNotReady(msg) => {
                write!(f, "Quarantine output root is not ready: {}", msg)
            }
            AppError::Quarantine(msg) => write!(f, "Quarantine error: {}", msg),
            AppError::QuarantineConflict(msg) => write!(f, "Quarantine conflict: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<hl7v2::Error> for AppError {
    fn from(err: hl7v2::Error) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<hl7v2::conformance::profile::ProfileLoadError> for AppError {
    fn from(err: hl7v2::conformance::profile::ProfileLoadError) -> Self {
        AppError::ProfileLoad(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    #[test]
    fn test_error_response_creation() {
        let err = ErrorResponse::new("TEST_ERROR", "Test error message");
        assert_eq!(err.code, "TEST_ERROR");
        assert_eq!(err.message, "Test error message");
        assert!(err.details.is_none());
    }

    #[test]
    fn parse_request_message_accepts_plain_and_mllp_facade_paths() {
        let plain = parse_request_message(SAMPLE_MESSAGE.as_bytes(), false)
            .expect("plain message should parse");
        assert_eq!(plain.segments[0].id_str(), "MSH");

        let framed = hl7v2::wrap_mllp(SAMPLE_MESSAGE.as_bytes());
        let mllp = parse_request_message(&framed, true).expect("MLLP message should parse");
        assert_eq!(mllp.segments[0].id_str(), "MSH");
    }

    #[test]
    fn map_ack_code_uses_facade_ack_codes() {
        assert_eq!(map_ack_code(AckRequestCode::Aa), hl7v2::AckCode::AA);
        assert_eq!(map_ack_code(AckRequestCode::Ae), hl7v2::AckCode::AE);
        assert_eq!(map_ack_code(AckRequestCode::Ar), hl7v2::AckCode::AR);
        assert_eq!(map_ack_code(AckRequestCode::Ca), hl7v2::AckCode::CA);
        assert_eq!(map_ack_code(AckRequestCode::Ce), hl7v2::AckCode::CE);
        assert_eq!(map_ack_code(AckRequestCode::Cr), hl7v2::AckCode::CR);
    }

    #[test]
    fn metadata_helpers_use_facade_queries() {
        let message =
            parse_request_message(SAMPLE_MESSAGE.as_bytes(), false).expect("message should parse");

        let metadata = extract_metadata(&message).expect("metadata should extract");
        assert_eq!(metadata.message_type, "ADT^A01");
        assert_eq!(metadata.version, "2.5");
        assert_eq!(metadata.sending_application, "SENDAPP");
        assert_eq!(metadata.sending_facility, "SENDFAC");
        assert_eq!(metadata.message_control_id, "CTRL123");

        assert_eq!(
            joined_components(&message, "MSH.9").as_deref(),
            Some("ADT^A01")
        );
        assert_eq!(
            joined_components(&message, "MSH.3").as_deref(),
            Some("SENDAPP")
        );
    }

    #[test]
    fn app_error_from_facade_errors_preserves_variant() {
        let parse_error: AppError = hl7v2::Error::InvalidSegmentId.into();
        assert!(matches!(parse_error, AppError::Parse(_)));

        let profile_error: AppError =
            hl7v2::conformance::profile::ProfileLoadError::YamlParse("bad yaml".to_string()).into();
        assert!(matches!(profile_error, AppError::ProfileLoad(_)));
    }
}
