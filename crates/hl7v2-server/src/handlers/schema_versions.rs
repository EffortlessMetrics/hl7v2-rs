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
