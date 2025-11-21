//! HTTP request handlers for HL7v2 endpoints.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use itertools::Itertools;
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
pub async fn parse_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ParseRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Parse the message
    let message_bytes = request.message.as_bytes();

    let message = if request.mllp_framed {
        hl7v2_core::parse_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))?
    } else {
        hl7v2_core::parse(message_bytes)
            .map_err(|e| AppError::Parse(format!("Parse error: {}", e)))?
    };

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
pub async fn validate_handler(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ValidateRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Parse the message
    let message_bytes = request.message.as_bytes();

    let message = if request.mllp_framed {
        hl7v2_core::parse_mllp(message_bytes)
            .map_err(|e| AppError::Parse(format!("MLLP parse error: {}", e)))?
    } else {
        hl7v2_core::parse(message_bytes)
            .map_err(|e| AppError::Parse(format!("Parse error: {}", e)))?
    };

    // Extract metadata
    let metadata = extract_metadata(&message)?;

    // Load profile and validate
    let profile = hl7v2_prof::load_profile(&request.profile)
        .map_err(|e| AppError::Validation(format!("Invalid profile: {:?}", e)))?;

    let issues = hl7v2_prof::validate(&message, &profile);

    // Separate issues into errors and warnings
    let (errors, warnings): (Vec<ValidationError>, Vec<ValidationWarning>) = issues
        .into_iter()
        .partition_map(|issue| {
            match issue.severity {
                hl7v2_prof::Severity::Error => itertools::Either::Left(ValidationError {
                    code: issue.code.to_string(),
                    message: issue.detail,
                    location: issue.path,
                    severity: ErrorSeverity::Error,
                }),
                hl7v2_prof::Severity::Warning => itertools::Either::Right(ValidationWarning {
                    code: issue.code.to_string(),
                    message: issue.detail,
                    location: issue.path,
                }),
            }
        });

    let valid = errors.is_empty();

    let response = ValidateResponse {
        valid,
        errors,
        warnings,
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Extract message metadata from parsed message
fn extract_metadata(message: &hl7v2_core::Message) -> Result<MessageMetadata, AppError> {
    message.metadata().map_err(|e| AppError::Parse(e.to_string()))
}

/// Application error type
#[derive(Debug)]
pub enum AppError {
    Parse(String),
    Validation(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Parse(msg) => (StatusCode::BAD_REQUEST, "PARSE_ERROR", msg),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        let error = ErrorResponse::new(code, message);
        (status, Json(error)).into_response()
    }
}

impl From<hl7v2_core::Error> for AppError {
    fn from(err: hl7v2_core::Error) -> Self {
        AppError::Parse(err.to_string())
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
