use crate::handlers::error::AppError;
use crate::models::RedactionReceipt;
use crate::redaction::redact_message;

pub(super) fn parse_request_message(
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

pub(super) fn parse_request_message_with_metrics(
    message_bytes: &[u8],
    mllp_framed: bool,
    operation: &'static str,
) -> Result<hl7v2::Message, AppError> {
    match parse_request_message(message_bytes, mllp_framed) {
        Ok(message) => {
            crate::metrics::record_parse_success(operation, message_bytes.len());
            Ok(message)
        }
        Err(error) => {
            crate::metrics::record_parse_failure(operation);
            Err(error)
        }
    }
}

pub(super) fn redact_message_with_metrics(
    message: &mut hl7v2::Message,
    policy_toml: &str,
    operation: &'static str,
) -> Result<RedactionReceipt, AppError> {
    match redact_message(message, policy_toml) {
        Ok(receipt) => Ok(receipt),
        Err(error) => {
            crate::metrics::record_redaction_failure(operation);
            Err(AppError::Redaction(error))
        }
    }
}
