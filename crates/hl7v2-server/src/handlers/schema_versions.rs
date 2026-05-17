use crate::handlers::error::AppError;

const DEFAULT_SCHEMA_VERSION: u8 = 1;
const LATEST_SCHEMA_VERSION: u8 = 2;

fn requested_schema_version(version: Option<u8>, artifact: &str) -> Result<u8, AppError> {
    match version.unwrap_or(DEFAULT_SCHEMA_VERSION) {
        DEFAULT_SCHEMA_VERSION => Ok(DEFAULT_SCHEMA_VERSION),
        LATEST_SCHEMA_VERSION => Ok(LATEST_SCHEMA_VERSION),
        other => Err(AppError::Validation(format!(
            "unsupported {artifact} schema version {other}; expected {DEFAULT_SCHEMA_VERSION} or {LATEST_SCHEMA_VERSION}"
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
