use crate::handlers::error::AppError;

fn requested_schema_version(version: Option<u8>, schema_name: &str) -> Result<u8, AppError> {
    let requested = version.unwrap_or(1);

    match requested {
        1 | 2 => Ok(requested),
        other => Err(AppError::Validation(format!(
            "unsupported {schema_name} schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_report_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    requested_schema_version(version, "validation report")
}

pub(super) fn requested_redaction_receipt_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    requested_schema_version(version, "redaction receipt")
}

pub(super) fn requested_quarantine_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    requested_schema_version(version, "quarantine output")
}

pub(super) fn requested_bundle_artifact_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    requested_schema_version(version, "bundle artifact")
}

pub(super) fn requested_replay_report_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    requested_schema_version(version, "replay report")
}

pub(super) fn requested_corpus_summary_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    requested_schema_version(version, "corpus summary")
}

pub(super) fn requested_corpus_fingerprint_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    requested_schema_version(version, "corpus fingerprint")
}

pub(super) fn requested_corpus_diff_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    requested_schema_version(version, "corpus diff")
}

pub(super) fn requested_profile_lint_report_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    requested_schema_version(version, "profile lint report")
}

pub(super) fn requested_profile_explain_report_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    requested_schema_version(version, "profile explain report")
}

pub(super) fn requested_profile_test_report_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    requested_schema_version(version, "profile test report")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each validator follows the same `None=>1, 1=>1, 2=>2, other=>Err` shape.
    /// `Validator` lets the same matrix exercise every function.
    type Validator = fn(Option<u8>) -> Result<u8, AppError>;

    const VALIDATORS: &[(&str, Validator)] = &[
        (
            "validation report",
            requested_report_schema_version as Validator,
        ),
        (
            "redaction receipt",
            requested_redaction_receipt_schema_version as Validator,
        ),
        (
            "quarantine",
            requested_quarantine_schema_version as Validator,
        ),
        (
            "bundle artifact",
            requested_bundle_artifact_schema_version as Validator,
        ),
        (
            "replay report",
            requested_replay_report_schema_version as Validator,
        ),
        (
            "corpus summary",
            requested_corpus_summary_schema_version as Validator,
        ),
        (
            "corpus fingerprint",
            requested_corpus_fingerprint_schema_version as Validator,
        ),
        (
            "corpus diff",
            requested_corpus_diff_schema_version as Validator,
        ),
        (
            "profile lint report",
            requested_profile_lint_report_schema_version as Validator,
        ),
        (
            "profile explain report",
            requested_profile_explain_report_schema_version as Validator,
        ),
        (
            "profile test report",
            requested_profile_test_report_schema_version as Validator,
        ),
    ];

    #[test]
    fn none_defaults_to_one_for_every_validator() {
        for (label, validator) in VALIDATORS {
            let actual = validator(None);
            assert!(
                matches!(actual, Ok(1)),
                "{label} validator should default None to 1, got {actual:?}"
            );
        }
    }

    #[test]
    fn explicit_one_returns_one_for_every_validator() {
        for (label, validator) in VALIDATORS {
            let actual = validator(Some(1));
            assert!(
                matches!(actual, Ok(1)),
                "{label} validator should accept 1, got {actual:?}"
            );
        }
    }

    #[test]
    fn explicit_two_returns_two_for_every_validator() {
        for (label, validator) in VALIDATORS {
            let actual = validator(Some(2));
            assert!(
                matches!(actual, Ok(2)),
                "{label} validator should accept 2, got {actual:?}"
            );
        }
    }

    #[test]
    fn zero_is_rejected_for_every_validator() {
        for (label, validator) in VALIDATORS {
            let actual = validator(Some(0));
            assert!(
                matches!(actual, Err(AppError::Validation(_))),
                "{label} validator should reject 0, got {actual:?}"
            );
        }
    }

    #[test]
    fn three_is_rejected_for_every_validator() {
        for (label, validator) in VALIDATORS {
            let actual = validator(Some(3));
            assert!(
                matches!(actual, Err(AppError::Validation(_))),
                "{label} validator should reject 3, got {actual:?}"
            );
        }
    }

    #[test]
    fn max_u8_is_rejected_for_every_validator() {
        for (label, validator) in VALIDATORS {
            let actual = validator(Some(u8::MAX));
            assert!(
                matches!(actual, Err(AppError::Validation(_))),
                "{label} validator should reject 255, got {actual:?}"
            );
        }
    }

    #[test]
    fn error_messages_mention_unsupported_value_and_expected_versions() {
        // Sample one validator to verify the human-readable wording so
        // callers get a useful diagnostic from the error path.
        let err = requested_report_schema_version(Some(7))
            .expect_err("unsupported version must return Err");
        assert!(
            matches!(&err, AppError::Validation(m) if m.contains("7") && m.contains("1 or 2")),
            "expected Validation mentioning '7' and '1 or 2', got {err:?}"
        );
    }

    #[test]
    fn each_validator_emits_its_own_label_in_the_error_message() {
        let cases = [
            (
                requested_report_schema_version as Validator,
                "validation report",
            ),
            (
                requested_redaction_receipt_schema_version as Validator,
                "redaction receipt",
            ),
            (
                requested_quarantine_schema_version as Validator,
                "quarantine output",
            ),
            (
                requested_bundle_artifact_schema_version as Validator,
                "bundle artifact",
            ),
            (
                requested_replay_report_schema_version as Validator,
                "replay report",
            ),
            (
                requested_corpus_summary_schema_version as Validator,
                "corpus summary",
            ),
            (
                requested_corpus_fingerprint_schema_version as Validator,
                "corpus fingerprint",
            ),
            (
                requested_corpus_diff_schema_version as Validator,
                "corpus diff",
            ),
            (
                requested_profile_lint_report_schema_version as Validator,
                "profile lint report",
            ),
            (
                requested_profile_explain_report_schema_version as Validator,
                "profile explain report",
            ),
            (
                requested_profile_test_report_schema_version as Validator,
                "profile test report",
            ),
        ];

        for (validator, expected_label) in cases {
            let err = validator(Some(9)).expect_err("unsupported version must return Err");
            assert!(
                matches!(&err, AppError::Validation(m) if m.contains(expected_label)),
                "expected Validation mentioning '{expected_label}', got {err:?}"
            );
        }
    }
}
