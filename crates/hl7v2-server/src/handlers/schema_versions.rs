use crate::handlers::error::AppError;

pub(super) fn requested_report_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported validation report schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_redaction_receipt_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported redaction receipt schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_quarantine_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported quarantine output schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_bundle_artifact_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported bundle artifact schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_replay_report_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported replay report schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_corpus_summary_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported corpus summary schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_corpus_fingerprint_schema_version(
    version: Option<u8>,
) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported corpus fingerprint schema version {other}; expected 1 or 2"
        ))),
    }
}

pub(super) fn requested_corpus_diff_schema_version(version: Option<u8>) -> Result<u8, AppError> {
    match version.unwrap_or(1) {
        1 => Ok(1),
        2 => Ok(2),
        other => Err(AppError::Validation(format!(
            "unsupported corpus diff schema version {other}; expected 1 or 2"
        ))),
    }
}
