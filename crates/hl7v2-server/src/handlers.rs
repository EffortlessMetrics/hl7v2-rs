//! HTTP request handlers for HL7v2 endpoints.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

use crate::audit::{self, MessageLogContext};
use crate::models::*;
use crate::server::AppState;

mod ack_policy;
mod corpus;
mod error;
mod metadata;
mod parsing;
mod profile;
mod quarantine;
mod schema_versions;
mod validation;

use ack_policy::*;
use corpus::*;
pub use error::AppError;
use metadata::extract_metadata;
#[cfg(test)]
use metadata::joined_components;
use parsing::*;
pub(crate) use profile::*;
use quarantine::*;
use schema_versions::*;
use validation::*;

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
    State(state): State<Arc<AppState>>,
    Json(request): Json<ParseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message_with_metrics(
        request.message.as_bytes(),
        request.mllp_framed,
        crate::metrics::operation::PARSE,
        state.max_message_size,
    )?;

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
    State(state): State<Arc<AppState>>,
    Json(request): Json<ValidateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let report_schema_version = requested_report_schema_version(request.report_schema_version)?;
    let message = parse_request_message_with_metrics(
        request.message.as_bytes(),
        request.mllp_framed,
        crate::metrics::operation::VALIDATE,
        state.max_message_size,
    )?;

    // Extract metadata
    let metadata = extract_metadata(&message)?;
    let log_context = MessageLogContext::from_message(&message);

    // Load the profile before validation. Profile load failures are client
    // errors, not successful validation results.
    let validation = validate_message_with_profile(
        &message,
        &request.profile,
        crate::metrics::operation::VALIDATE,
        |profile| Some(profile.message_structure.clone()),
    )?;
    let profile = validation.profile;
    let report = validation.report;
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
    let (errors, warnings) = legacy_validation_items(validation.issues);

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
    let mut message = parse_request_message_with_metrics(
        &raw_input,
        request.mllp_framed,
        crate::metrics::operation::VALIDATE_REDACTED,
        state.max_message_size,
    )?;
    let log_context = MessageLogContext::from_message(&message);
    let receipt = redact_message_with_metrics(
        &mut message,
        &request.redaction_policy,
        crate::metrics::operation::VALIDATE_REDACTED,
    )?;
    let redacted_hl7 = String::from_utf8(hl7v2::write(&message))
        .map_err(|error| AppError::Internal(format!("redacted message was not UTF-8: {error}")))?;

    let validation = validate_message_with_profile(
        &message,
        &request.profile,
        crate::metrics::operation::VALIDATE_REDACTED,
        |profile| Some(profile.message_structure.clone()),
    )?;
    let profile = validation.profile;
    let validation_report = validation.report;
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
    let mut message = parse_request_message_with_metrics(
        &raw_input,
        request.mllp_framed,
        crate::metrics::operation::BUNDLE,
        state.max_message_size,
    )?;
    let log_context = MessageLogContext::from_message(&message);
    let receipt = redact_message_with_metrics(
        &mut message,
        &request.redaction_policy,
        crate::metrics::operation::BUNDLE,
    )?;
    let redacted_hl7 = String::from_utf8(hl7v2::write(&message))
        .map_err(|error| AppError::Internal(format!("redacted message was not UTF-8: {error}")))?;

    let validation = validate_message_with_profile(
        &message,
        &request.profile,
        crate::metrics::operation::BUNDLE,
        |_profile| Some("profile.yaml".to_string()),
    )?;
    let profile = validation.profile;
    let validation_report = validation.report;

    let summary =
        crate::evidence::write_evidence_bundle(crate::evidence::EvidenceBundleWriteRequest {
            root: bundle_output_root,
            bundle_id: &request.bundle_id,
            public_output_dir: Some(&audit::hash_identifier(&request.bundle_id)),
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
    crate::metrics::record_bundle_created();

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
        bundle_id_hash = %audit::hash_identifier(&request.bundle_id),
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
        crate::metrics::record_replay_result(false);
        return Err(AppError::BundleNotFound(
            "bundle id was not found".to_string(),
        ));
    }

    let report = hl7v2::evidence::replay_evidence_bundle(&bundle_dir, "hl7v2-server");
    crate::metrics::record_replay_result(report.reproduced);
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
    crate::metrics::record_corpus_diff();
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
    State(state): State<Arc<AppState>>,
    Json(request): Json<AckRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message_with_metrics(
        request.message.as_bytes(),
        request.mllp_framed,
        crate::metrics::operation::ACK,
        state.max_message_size,
    )?;
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

    let (message, validation_report, decision) = match parse_request_message_with_metrics(
        &raw_input,
        request.mllp_framed,
        crate::metrics::operation::ACK_POLICY,
        state.max_message_size,
    ) {
        Ok(message) => {
            let validation = validate_message_with_profile(
                &message,
                &request.profile,
                crate::metrics::operation::ACK_POLICY,
                |profile| Some(profile.message_structure.clone()),
            )?;
            let report = validation.report;
            let decision = ack_policy_decision_for_validation(policy, &report)?;
            (message, Some(report), decision)
        }
        Err(error @ AppError::Parse(_)) if policy.rejects(AckPolicyRejectCondition::ParseError) => {
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
    State(state): State<Arc<AppState>>,
    Json(request): Json<NormalizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message_bytes = request.message.as_bytes();
    if let Err(error) = enforce_message_size(message_bytes, state.max_message_size) {
        crate::metrics::record_parse_failure(crate::metrics::operation::NORMALIZE);
        return Err(error);
    }
    let input = if request.mllp_framed {
        hl7v2::unwrap_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))?
    } else {
        message_bytes
    };

    let normalized_bytes = hl7v2::normalize(input, request.options.canonical_delimiters)
        .map_err(|e| AppError::Parse(format!("Normalize error: {}", e)))?;
    let normalized_message = hl7v2::parse(&normalized_bytes).map_err(|e| {
        crate::metrics::record_parse_failure(crate::metrics::operation::NORMALIZE);
        AppError::Parse(format!("Normalized message parse error: {}", e))
    })?;
    crate::metrics::record_parse_success(
        crate::metrics::operation::NORMALIZE,
        normalized_bytes.len(),
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John\r";

    #[test]
    fn test_error_response_creation() {
        let err = ErrorResponse::new("TEST_ERROR", "Test error message");
        assert_eq!(err.code, "TEST_ERROR");
        assert_eq!(err.message, "Test error message");
        assert!(err.safe_detail.is_none());
        assert!(err.location.is_none());
        assert!(err.suggested_next_action.is_none());
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
