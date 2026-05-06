//! HTTP request handlers for HL7v2 endpoints.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::models::*;
use crate::server::AppState;

/// Handler for GET /health
pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();

    let response = HealthResponse {
        status: HealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    };

    (StatusCode::OK, Json(response))
}

/// Handler for POST /hl7/parse
///
/// # Errors
///
/// Returns an application error when the request message cannot be parsed or
/// its required metadata cannot be extracted.
pub async fn parse_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ParseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message(request.message.as_bytes(), request.mllp_framed)?;

    // Extract metadata
    let metadata = extract_metadata(&message)?;

    // Optionally convert to JSON
    let message_json = if request.options.include_json {
        Some(hl7v2_core::to_json(&message))
    } else {
        None
    };

    let response = ParseResponse {
        message: message_json,
        metadata,
        warnings: Vec::new(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/validate
///
/// # Errors
///
/// Returns an application error when the request message cannot be parsed, the
/// profile cannot be loaded, or required message metadata is missing.
pub async fn validate_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ValidateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message(request.message.as_bytes(), request.mllp_framed)?;

    // Extract metadata
    let metadata = extract_metadata(&message)?;

    // Load the profile before validation. Profile load failures are client
    // errors, not successful validation results.
    let profile = hl7v2_prof::load_profile_checked(&request.profile)
        .map_err(|e| AppError::ProfileLoad(e.to_string()))?;

    // Perform validation using the profile
    let issues = hl7v2_prof::validate(&message, &profile);

    // Convert validation issues to response format
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for issue in issues {
        let severity = match issue.severity {
            hl7v2_prof::Severity::Error => ErrorSeverity::Error,
            hl7v2_prof::Severity::Warning => ErrorSeverity::Warning,
        };

        let validation_item = ValidationError {
            code: issue.code,
            message: issue.detail,
            location: issue.path,
            severity,
        };

        match issue.severity {
            hl7v2_prof::Severity::Error => errors.push(validation_item),
            hl7v2_prof::Severity::Warning => {
                warnings.push(ValidationWarning {
                    code: validation_item.code,
                    message: validation_item.message,
                    location: validation_item.location,
                });
            }
        }
    }

    let valid = errors.is_empty();

    let response = ValidateResponse {
        valid,
        errors,
        warnings,
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/ack
///
/// # Errors
///
/// Returns an application error when the request message cannot be parsed, ACK
/// generation fails, or the generated ACK cannot be encoded as UTF-8.
pub async fn ack_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<AckRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = parse_request_message(request.message.as_bytes(), request.mllp_framed)?;
    let ack_code = map_ack_code(request.code);

    let ack_message = if let Some(error_message) = request.error_message.as_deref() {
        hl7v2_gen::ack_with_error(&message, ack_code, Some(error_message))
    } else {
        hl7v2_gen::ack(&message, ack_code)
    }
    .map_err(|e| AppError::Internal(format!("Failed to generate ACK: {e}")))?;

    let metadata = extract_metadata(&ack_message)?;
    let ack_bytes = hl7v2_writer::write(&ack_message);
    let ack_bytes = if request.mllp_frame {
        hl7v2_mllp::wrap_mllp(&ack_bytes)
    } else {
        ack_bytes
    };

    let response = AckResponse {
        ack_message: String::from_utf8(ack_bytes)
            .map_err(|e| AppError::Internal(format!("ACK was not UTF-8: {e}")))?,
        ack_code: request.code.as_str().to_string(),
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /hl7/normalize
///
/// # Errors
///
/// Returns an application error when MLLP framing is invalid, normalization
/// fails, normalized output cannot be parsed, or output is not UTF-8.
pub async fn normalize_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<NormalizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message_bytes = request.message.as_bytes();
    let input = if request.mllp_framed {
        hl7v2_mllp::unwrap_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {e}")))?
    } else {
        message_bytes
    };

    let normalized_bytes = hl7v2_normalize::normalize(input, request.options.canonical_delimiters)
        .map_err(|e| AppError::Parse(format!("Normalize error: {e}")))?;
    let normalized_message = hl7v2_core::parse(&normalized_bytes)
        .map_err(|e| AppError::Parse(format!("Normalized message parse error: {e}")))?;
    let metadata = extract_metadata(&normalized_message)?;

    let response_bytes = if request.options.mllp_frame {
        hl7v2_mllp::wrap_mllp(&normalized_bytes)
    } else {
        normalized_bytes
    };

    let response = NormalizeResponse {
        normalized_message: String::from_utf8(response_bytes)
            .map_err(|e| AppError::Internal(format!("Normalized message was not UTF-8: {e}")))?,
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

fn parse_request_message(
    message_bytes: &[u8],
    mllp_framed: bool,
) -> Result<hl7v2_core::Message, AppError> {
    if mllp_framed {
        hl7v2_core::parse_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {e}")))
    } else {
        hl7v2_core::parse(message_bytes).map_err(|e| AppError::Parse(format!("Parse error: {e}")))
    }
}

fn map_ack_code(code: AckRequestCode) -> hl7v2_gen::AckCode {
    match code {
        AckRequestCode::Aa => hl7v2_gen::AckCode::AA,
        AckRequestCode::Ae => hl7v2_gen::AckCode::AE,
        AckRequestCode::Ar => hl7v2_gen::AckCode::AR,
        AckRequestCode::Ca => hl7v2_gen::AckCode::CA,
        AckRequestCode::Ce => hl7v2_gen::AckCode::CE,
        AckRequestCode::Cr => hl7v2_gen::AckCode::CR,
    }
}

/// Extract message metadata from parsed message
fn extract_metadata(message: &hl7v2_core::Message) -> Result<MessageMetadata, AppError> {
    // Find MSH segment
    let msh = message
        .segments
        .first()
        .ok_or_else(|| AppError::Parse("Missing MSH segment".to_string()))?;

    if &msh.id != b"MSH" {
        return Err(AppError::Parse("First segment must be MSH".to_string()));
    }

    // Extract MSH fields
    let message_type = joined_components(message, "MSH.9").unwrap_or_else(|| "UNKNOWN".to_string());

    let version = hl7v2_core::get(message, "MSH.12")
        .unwrap_or("2.5")
        .to_string();

    let sending_application = hl7v2_core::get(message, "MSH.3").unwrap_or("").to_string();

    let sending_facility = hl7v2_core::get(message, "MSH.4").unwrap_or("").to_string();

    let message_control_id = hl7v2_core::get(message, "MSH.10").unwrap_or("").to_string();

    Ok(MessageMetadata {
        message_type,
        version,
        sending_application,
        sending_facility,
        message_control_id,
        segment_count: message.segments.len(),
        charsets: message.charsets.clone(),
    })
}

fn joined_components(message: &hl7v2_core::Message, path: &str) -> Option<String> {
    let mut components = Vec::new();

    for index in 1.. {
        let component_path = format!("{path}.{index}");
        match hl7v2_core::get(message, &component_path) {
            Some(value) if !value.is_empty() => components.push(value.to_string()),
            Some(_) => {}
            None => break,
        }
    }

    if components.is_empty() {
        hl7v2_core::get(message, path).map(str::to_string)
    } else {
        Some(components.join("^"))
    }
}

/// Application error type with specific error variants.
///
/// This enum provides detailed error information for different failure modes,
/// making it easier to diagnose issues and provide meaningful error responses.
#[derive(Debug)]
pub enum AppError {
    /// Message parsing error (malformed HL7, invalid structure, etc.)
    Parse(String),

    /// Profile loading error (YAML syntax, missing fields, etc.)
    ProfileLoad(String),

    /// Validation error (message does not conform to profile)
    Validation(String),

    /// Internal server error (unexpected failures)
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Parse(msg) => (StatusCode::BAD_REQUEST, "PARSE_ERROR", msg),
            // Profile load error is a client error since the profile is provided in the request
            AppError::ProfileLoad(msg) => (StatusCode::BAD_REQUEST, "PROFILE_LOAD_ERROR", msg),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        let error = ErrorResponse::new(code, message);
        (status, Json(error)).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Parse(msg) => write!(f, "Parse error: {msg}"),
            AppError::ProfileLoad(msg) => write!(f, "Profile load error: {msg}"),
            AppError::Validation(msg) => write!(f, "Validation error: {msg}"),
            AppError::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl From<hl7v2_core::Error> for AppError {
    fn from(err: hl7v2_core::Error) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<hl7v2_prof::ProfileLoadError> for AppError {
    fn from(err: hl7v2_prof::ProfileLoadError) -> Self {
        AppError::ProfileLoad(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_creation() {
        let err = ErrorResponse::new("TEST_ERROR", "Test error message");
        assert_eq!(err.code, "TEST_ERROR");
        assert_eq!(err.message, "Test error message");
        assert!(err.details.is_none());
    }
}
