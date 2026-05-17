use crate::parser::parse;
use crate::writer::write;

use super::digest::{compute_sha256, compute_sha256_bytes};
use super::policy::redact_message_safe_analysis;
use super::text::message_type;
use super::types::{RedactionError, SafeAnalysisRedactionOutput};

/// Apply a safe-analysis policy to raw HL7 and return redacted evidence output.
///
/// This function fails closed when the policy is malformed, contains duplicate
/// paths, tries to retain built-in sensitive fields, omits present built-in
/// sensitive fields, or has a non-optional redaction rule that matches nothing.
///
/// # Errors
///
/// Returns [`RedactionError`] when the input message cannot parse, the policy
/// cannot be loaded, the policy does not protect present sensitive fields, or
/// the redacted message cannot be encoded as UTF-8.
pub fn redact_hl7_safe_analysis(
    content: impl AsRef<[u8]>,
    policy_text: &str,
) -> Result<SafeAnalysisRedactionOutput, RedactionError> {
    let content = content.as_ref();
    let mut message = parse(content).map_err(|error| RedactionError::Parse(error.to_string()))?;
    let message_type = message_type(&message);
    let receipt = redact_message_safe_analysis(&mut message, policy_text)?;
    let redacted_hl7 = String::from_utf8(write(&message))
        .map_err(|error| RedactionError::Utf8(error.to_string()))?;

    Ok(SafeAnalysisRedactionOutput {
        input_sha256: compute_sha256_bytes(content),
        policy_sha256: compute_sha256(policy_text),
        message_type,
        redacted_hl7,
        receipt,
    })
}
