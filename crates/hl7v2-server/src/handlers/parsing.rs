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
    max_message_size: usize,
) -> Result<hl7v2::Message, AppError> {
    if let Err(error) = enforce_message_size(message_bytes, max_message_size) {
        crate::metrics::record_parse_failure(operation);
        return Err(error);
    }

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

pub(super) fn enforce_message_size(
    message_bytes: &[u8],
    max_message_size: usize,
) -> Result<(), AppError> {
    if message_bytes.len() > max_message_size {
        return Err(AppError::MessageTooLarge {
            actual: message_bytes.len(),
            max: max_message_size,
        });
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_size_limit_includes_exact_boundary_and_rejects_overage() -> Result<(), String> {
        enforce_message_size(b"1234", 4).map_err(|error| error.to_string())?;
        match enforce_message_size(b"12345", 4) {
            Err(AppError::MessageTooLarge { actual: 5, max: 4 }) => Ok(()),
            Ok(()) => Err("message overage was accepted".to_string()),
            Err(error) => Err(format!("unexpected message size error: {error}")),
        }
    }
}
