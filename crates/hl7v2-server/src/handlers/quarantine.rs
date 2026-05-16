use crate::handlers::error::AppError;
use crate::models::{QuarantineOutputSummary, RedactionReceipt};
use crate::server::AppState;

pub(super) struct RedactedQuarantineContext<'a> {
    pub(super) state: &'a AppState,
    pub(super) raw_input: &'a [u8],
    pub(super) profile_yaml: &'a str,
    pub(super) policy_text: &'a str,
    pub(super) redacted_message: &'a hl7v2::Message,
    pub(super) redacted_hl7: &'a str,
    pub(super) redaction_receipt: &'a RedactionReceipt,
    pub(super) validation_report: &'a hl7v2::ValidationReport,
}

pub(super) fn maybe_write_redacted_quarantine(
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

pub(super) fn generated_quarantine_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("quarantine-{}-{nanos}", std::process::id())
}
