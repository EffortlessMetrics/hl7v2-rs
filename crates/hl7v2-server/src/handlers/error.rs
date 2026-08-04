use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::PROFILE_LOAD_SAFE_MESSAGE;
use crate::audit;
use crate::models::ErrorResponse;

/// Application error type with specific error variants.
///
/// This enum provides detailed error information for different failure modes,
/// making it easier to diagnose issues and provide meaningful error responses.
#[derive(Debug)]
pub enum AppError {
    /// Message parsing error (malformed HL7, invalid structure, etc.)
    Parse(String),

    /// Decoded HL7 message exceeded the configured application-level limit.
    MessageTooLarge { actual: usize, max: usize },

    /// Profile loading error (YAML syntax, missing fields, etc.)
    ProfileLoad(String),

    /// Validation error (message does not conform to profile)
    Validation(String),

    /// Redaction policy or redaction application error
    Redaction(String),

    /// Bundle output is not configured on the server
    BundleOutputNotConfigured,

    /// Bundle output root is configured but not writable or available
    BundleOutputNotReady(String),

    /// Bundle request or write error
    Bundle(String),

    /// Bundle output already exists
    Conflict(String),

    /// Requested bundle id was not found under the configured output root
    BundleNotFound(String),

    /// Quarantine output is enabled but no path is configured
    QuarantineOutputNotConfigured,

    /// Quarantine output root is configured but not writable or available
    QuarantineOutputNotReady(String),

    /// Quarantine output write error
    Quarantine(String),

    /// Quarantine output already exists
    QuarantineConflict(String),

    /// Internal server error (unexpected failures)
    Internal(String),
}

impl From<crate::evidence::EvidenceBundleError> for AppError {
    fn from(error: crate::evidence::EvidenceBundleError) -> Self {
        match error {
            crate::evidence::EvidenceBundleError::InvalidRequest(message) => Self::Bundle(message),
            crate::evidence::EvidenceBundleError::Conflict(message) => Self::Conflict(message),
            crate::evidence::EvidenceBundleError::Io(message) => {
                Self::BundleOutputNotReady(message)
            }
        }
    }
}

impl AppError {
    pub(super) fn quarantine_from_evidence_error(
        error: crate::evidence::EvidenceBundleError,
    ) -> Self {
        match error {
            crate::evidence::EvidenceBundleError::InvalidRequest(message) => {
                Self::Quarantine(message)
            }
            crate::evidence::EvidenceBundleError::Conflict(message) => {
                Self::QuarantineConflict(message)
            }
            crate::evidence::EvidenceBundleError::Io(message) => {
                Self::QuarantineOutputNotReady(message)
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, safe_detail, location, next_action) = match self {
            AppError::Parse(msg) => (
                StatusCode::BAD_REQUEST,
                "PARSE_ERROR",
                msg,
                "The request message could not be parsed as HL7 v2. Raw message content is not echoed.",
                Some("message"),
                "Check the MSH segment, segment terminators, encoding, and mllp_framed setting.",
            ),
            AppError::MessageTooLarge { actual, max } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "MESSAGE_TOO_LARGE",
                format!("message size {actual} bytes exceeds maximum of {max} bytes"),
                "The HL7 message exceeded the configured application-level size limit and was not parsed.",
                Some("message"),
                "Reduce the message size or configure a reviewed larger message limit.",
            ),
            // Profile load error is a client error since the profile is provided in the request.
            // Keep the public message stable and avoid echoing parser detail derived from profile YAML.
            AppError::ProfileLoad(_) => (
                StatusCode::BAD_REQUEST,
                "PROFILE_LOAD_ERROR",
                PROFILE_LOAD_SAFE_MESSAGE.to_string(),
                "The supplied inline profile could not be loaded. Raw profile content is not echoed.",
                Some("profile"),
                "Run profile lint on the profile, then retry validation with the corrected profile.",
            ),
            AppError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                msg,
                "The request failed validation before a successful evidence response was produced.",
                None,
                "Check request parameters, schema-version fields, and validation issue paths where available.",
            ),
            AppError::Redaction(msg) => (
                StatusCode::BAD_REQUEST,
                "REDACTION_ERROR",
                msg,
                "The redaction policy or redaction run failed before a safe response was produced.",
                Some("redaction_policy"),
                "Check safe-analysis policy paths, actions, reasons, and required-field matches before retrying.",
            ),
            AppError::BundleOutputNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "BUNDLE_OUTPUT_NOT_CONFIGURED",
                "server bundle output root is not configured".to_string(),
                "The server cannot create evidence bundles until an operator configures a bundle root.",
                Some("bundle_output_root"),
                "Configure the server bundle output root and verify readiness before retrying.",
            ),
            AppError::BundleOutputNotReady(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "BUNDLE_OUTPUT_NOT_READY",
                msg,
                "The configured bundle output root is not currently writable or available.",
                Some("bundle_output_root"),
                "Check server filesystem permissions and readiness before retrying.",
            ),
            AppError::Bundle(msg) => (
                StatusCode::BAD_REQUEST,
                "BUNDLE_ERROR",
                msg,
                "The bundle request could not be accepted. Server responses use safe bundle identifiers.",
                Some("bundle_id"),
                "Use a simple bundle id without path traversal and retry after validating inputs.",
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                "BUNDLE_EXISTS",
                msg,
                "The requested bundle output already exists under the configured root.",
                Some("bundle_id"),
                "Choose a new bundle id or replay the existing bundle instead of overwriting it.",
            ),
            AppError::BundleNotFound(msg) => (
                StatusCode::NOT_FOUND,
                "BUNDLE_NOT_FOUND",
                msg,
                "The requested bundle id was not found under the configured root.",
                Some("bundle_id"),
                "Check the bundle id from the bundle creation receipt and retry.",
            ),
            AppError::QuarantineOutputNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "QUARANTINE_OUTPUT_NOT_CONFIGURED",
                "server quarantine output is enabled but no path is configured".to_string(),
                "The server cannot write quarantine artifacts until an operator configures a quarantine root.",
                Some("quarantine.path"),
                "Configure the quarantine output path or disable quarantine output before retrying.",
            ),
            AppError::QuarantineOutputNotReady(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "QUARANTINE_OUTPUT_NOT_READY",
                msg,
                "The configured quarantine output root is not currently writable or available.",
                Some("quarantine.path"),
                "Check server filesystem permissions and readiness before retrying.",
            ),
            AppError::Quarantine(msg) => (
                StatusCode::BAD_REQUEST,
                "QUARANTINE_ERROR",
                msg,
                "The quarantine request could not be written as configured.",
                Some("quarantine"),
                "Check quarantine artifact settings and retry with reviewed redaction inputs.",
            ),
            AppError::QuarantineConflict(msg) => (
                StatusCode::CONFLICT,
                "QUARANTINE_EXISTS",
                msg,
                "The generated quarantine output collided with existing output.",
                Some("quarantine"),
                "Retry the request or inspect the existing quarantine output before sharing evidence.",
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg,
                "The server hit an internal failure. Raw request payloads are not included in this response.",
                None,
                "Check server logs and readiness, then retry with the same request only if disclosure policy allows.",
            ),
        };

        tracing::warn!(
            target: "hl7v2_server::evidence",
            event = audit::EVENT_ERROR,
            status = status.as_u16(),
            error_code = code,
            "request failed"
        );

        let mut error = ErrorResponse::new(code, message)
            .with_safe_detail(safe_detail)
            .with_suggested_next_action(next_action);
        if let Some(location) = location {
            error = error.with_location(location);
        }
        (status, Json(error)).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Parse(msg) => write!(f, "Parse error: {}", msg),
            AppError::MessageTooLarge { actual, max } => write!(
                f,
                "Message size {actual} bytes exceeds maximum of {max} bytes"
            ),
            AppError::ProfileLoad(_) => {
                write!(f, "Profile load error: {PROFILE_LOAD_SAFE_MESSAGE}")
            }
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::Redaction(msg) => write!(f, "Redaction error: {}", msg),
            AppError::BundleOutputNotConfigured => {
                write!(f, "Bundle output root is not configured")
            }
            AppError::BundleOutputNotReady(msg) => {
                write!(f, "Bundle output root is not ready: {}", msg)
            }
            AppError::Bundle(msg) => write!(f, "Bundle error: {}", msg),
            AppError::Conflict(msg) => write!(f, "Bundle conflict: {}", msg),
            AppError::BundleNotFound(msg) => write!(f, "Bundle not found: {}", msg),
            AppError::QuarantineOutputNotConfigured => {
                write!(f, "Quarantine output path is not configured")
            }
            AppError::QuarantineOutputNotReady(msg) => {
                write!(f, "Quarantine output root is not ready: {}", msg)
            }
            AppError::Quarantine(msg) => write!(f, "Quarantine error: {}", msg),
            AppError::QuarantineConflict(msg) => write!(f, "Quarantine conflict: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<hl7v2::Error> for AppError {
    fn from(err: hl7v2::Error) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<hl7v2::conformance::profile::ProfileLoadError> for AppError {
    fn from(_err: hl7v2::conformance::profile::ProfileLoadError) -> Self {
        AppError::ProfileLoad(PROFILE_LOAD_SAFE_MESSAGE.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::EvidenceBundleError;

    #[test]
    fn from_evidence_bundle_error_maps_each_variant_to_a_bundle_app_error() {
        let invalid: AppError =
            EvidenceBundleError::InvalidRequest("bad request".to_string()).into();
        assert!(
            matches!(&invalid, AppError::Bundle(m) if m == "bad request"),
            "got {invalid:?}"
        );

        let conflict: AppError = EvidenceBundleError::Conflict("dup".to_string()).into();
        assert!(
            matches!(&conflict, AppError::Conflict(m) if m == "dup"),
            "got {conflict:?}"
        );

        let io_err: AppError = EvidenceBundleError::Io("disk full".to_string()).into();
        assert!(
            matches!(&io_err, AppError::BundleOutputNotReady(m) if m == "disk full"),
            "got {io_err:?}"
        );
    }

    #[test]
    fn quarantine_from_evidence_error_maps_each_variant_to_quarantine_app_error() {
        let invalid = AppError::quarantine_from_evidence_error(
            EvidenceBundleError::InvalidRequest("bad request".to_string()),
        );
        assert!(
            matches!(&invalid, AppError::Quarantine(m) if m == "bad request"),
            "got {invalid:?}"
        );

        let conflict = AppError::quarantine_from_evidence_error(EvidenceBundleError::Conflict(
            "exists".to_string(),
        ));
        assert!(
            matches!(&conflict, AppError::QuarantineConflict(m) if m == "exists"),
            "got {conflict:?}"
        );

        let io_err = AppError::quarantine_from_evidence_error(EvidenceBundleError::Io(
            "io fail".to_string(),
        ));
        assert!(
            matches!(&io_err, AppError::QuarantineOutputNotReady(m) if m == "io fail"),
            "got {io_err:?}"
        );
    }

    #[test]
    fn display_prefixes_each_variant_with_a_meaningful_label() {
        // Variants carrying a message should embed that message verbatim.
        assert_eq!(
            AppError::Parse("oops".to_string()).to_string(),
            "Parse error: oops"
        );
        assert_eq!(
            AppError::MessageTooLarge {
                actual: 11,
                max: 10
            }
            .to_string(),
            "Message size 11 bytes exceeds maximum of 10 bytes"
        );
        assert_eq!(
            AppError::Validation("bad".to_string()).to_string(),
            "Validation error: bad"
        );
        assert_eq!(
            AppError::Redaction("policy".to_string()).to_string(),
            "Redaction error: policy"
        );
        assert_eq!(
            AppError::Bundle("write".to_string()).to_string(),
            "Bundle error: write"
        );
        assert_eq!(
            AppError::Conflict("exists".to_string()).to_string(),
            "Bundle conflict: exists"
        );
        assert_eq!(
            AppError::BundleNotFound("missing".to_string()).to_string(),
            "Bundle not found: missing"
        );
        assert_eq!(
            AppError::BundleOutputNotReady("not ready".to_string()).to_string(),
            "Bundle output root is not ready: not ready"
        );
        assert_eq!(
            AppError::Quarantine("q".to_string()).to_string(),
            "Quarantine error: q"
        );
        assert_eq!(
            AppError::QuarantineConflict("qc".to_string()).to_string(),
            "Quarantine conflict: qc"
        );
        assert_eq!(
            AppError::QuarantineOutputNotReady("qr".to_string()).to_string(),
            "Quarantine output root is not ready: qr"
        );
        assert_eq!(
            AppError::Internal("boom".to_string()).to_string(),
            "Internal error: boom"
        );
    }

    #[test]
    fn display_for_unit_variants_emits_a_human_readable_message() {
        assert_eq!(
            AppError::BundleOutputNotConfigured.to_string(),
            "Bundle output root is not configured"
        );
        assert_eq!(
            AppError::QuarantineOutputNotConfigured.to_string(),
            "Quarantine output path is not configured"
        );
    }

    #[test]
    fn display_for_profile_load_uses_safe_message_rather_than_raw_detail() {
        // The Display impl must hide the original error detail and use the
        // shared safe message string so logs never leak profile content.
        let raw_detail = "this should not appear";
        let display = AppError::ProfileLoad(raw_detail.to_string()).to_string();
        assert!(
            display.contains(PROFILE_LOAD_SAFE_MESSAGE),
            "display should use safe message, got {display}"
        );
        assert!(
            !display.contains(raw_detail),
            "display must not leak inner detail, got {display}"
        );
    }

    #[test]
    fn from_hl7v2_error_maps_to_parse_variant() {
        let app_error: AppError = hl7v2::Error::InvalidSegmentId.into();
        assert!(matches!(app_error, AppError::Parse(_)));
    }

    #[test]
    fn from_profile_load_error_returns_safe_app_error_variant() {
        let app_error: AppError =
            hl7v2::conformance::profile::ProfileLoadError::YamlParse("bad yaml".to_string()).into();
        assert!(
            matches!(&app_error, AppError::ProfileLoad(m) if m == PROFILE_LOAD_SAFE_MESSAGE),
            "got {app_error:?}"
        );
    }
}
