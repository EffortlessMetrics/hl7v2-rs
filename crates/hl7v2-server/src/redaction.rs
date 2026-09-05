//! Safe-analysis redaction adapter for HTTP and gRPC evidence surfaces.
//!
//! Policy parsing, safety validation, target traversal, mutation, and receipt
//! semantics belong to `hl7v2`. This module only converts the canonical receipt
//! into the server's public evidence model.

use crate::models::{
    RedactionAction, RedactionActionReceipt, RedactionActionStatus, RedactionReceipt,
};
use hl7v2::Message;

/// Apply a safe-analysis policy to a message and return a server receipt.
pub fn redact_message(
    message: &mut Message,
    policy_text: &str,
) -> Result<RedactionReceipt, String> {
    let receipt = hl7v2::redact::redact_message_safe_analysis(message, policy_text)
        .map_err(|error| error.to_string())?;

    Ok(RedactionReceipt {
        phi_removed: receipt.phi_removed,
        hash_algorithm: receipt.hash_algorithm,
        actions: receipt
            .actions
            .into_iter()
            .map(|receipt| {
                let action = map_action(receipt.action);
                let status = map_status(receipt.status);
                RedactionActionReceipt {
                    path: receipt.path,
                    action,
                    reason: receipt.reason,
                    matched_count: receipt.matched_count,
                    optional: receipt.optional,
                    status,
                }
            })
            .collect(),
    })
}

fn map_action(action: hl7v2::redact::RedactionAction) -> RedactionAction {
    match action {
        hl7v2::redact::RedactionAction::Hash => RedactionAction::Hash,
        hl7v2::redact::RedactionAction::Drop => RedactionAction::Drop,
        hl7v2::redact::RedactionAction::Retain => RedactionAction::Retain,
    }
}

fn map_status(status: hl7v2::redact::RedactionActionStatus) -> RedactionActionStatus {
    match status {
        hl7v2::redact::RedactionActionStatus::Applied => RedactionActionStatus::Applied,
        hl7v2::redact::RedactionActionStatus::Retained => RedactionActionStatus::Retained,
        hl7v2::redact::RedactionActionStatus::NotFound => RedactionActionStatus::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use super::{RedactionActionStatus, redact_message};
    use std::io;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn require(condition: bool, message: &'static str) -> TestResult {
        if condition {
            Ok(())
        } else {
            Err(io::Error::other(message).into())
        }
    }

    fn policy(path: &str) -> String {
        format!(
            r#"
[[rules]]
path = "{path}"
action = "drop"
reason = "remove observation component"
"#
        )
    }

    #[test]
    fn omitted_field_repetition_uses_canonical_all_repetition_semantics() -> TestResult {
        let mut message = hl7v2::parse(
            b"MSH|^~\\&|SEND|FAC|RECV|FAC|202601010000||ORU^R01|CTRL|P|2.5\rOBX|1|TX|CODE||first^left~second^right",
        )?;
        let receipt = redact_message(&mut message, &policy("OBX.5.1")).map_err(io::Error::other)?;
        let output = String::from_utf8(hl7v2::write(&message))?;

        require(
            output.contains("OBX|1|TX|CODE||^left~^right"),
            "server adapter did not redact the component in every field repetition",
        )?;
        require(
            !output.contains("first") && !output.contains("second"),
            "server adapter leaked a targeted repetition",
        )?;
        let action = receipt
            .actions
            .first()
            .ok_or_else(|| io::Error::other("missing redaction receipt action"))?;
        require(
            action.matched_count == 1,
            "receipt count must remain segment-based",
        )?;
        require(
            action.status == RedactionActionStatus::Applied,
            "expected applied receipt status",
        )
    }

    #[test]
    fn explicit_field_repetition_remains_narrow() -> TestResult {
        let mut message = hl7v2::parse(
            b"MSH|^~\\&|SEND|FAC|RECV|FAC|202601010000||ORU^R01|CTRL|P|2.5\rOBX|1|TX|CODE||first^left~second^right",
        )?;
        let receipt =
            redact_message(&mut message, &policy("OBX.5[2].1")).map_err(io::Error::other)?;
        let output = String::from_utf8(hl7v2::write(&message))?;

        require(
            output.contains("OBX|1|TX|CODE||first^left~^right"),
            "server adapter widened an explicit field-repetition selector",
        )?;
        let action = receipt
            .actions
            .first()
            .ok_or_else(|| io::Error::other("missing redaction receipt action"))?;
        require(
            action.matched_count == 1,
            "explicit selector receipt count must remain segment-based",
        )
    }
}
