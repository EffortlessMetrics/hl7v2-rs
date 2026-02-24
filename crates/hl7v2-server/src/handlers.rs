//! HTTP request handlers for HL7v2 endpoints.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::models::*;
use crate::state::AppState;

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

    // TODO: Load profile and validate
    // For now, return a placeholder response
    let response = ValidateResponse {
        valid: true,
        errors: Vec::new(),
        warnings: Vec::new(),
        metadata,
    };

    Ok((StatusCode::OK, Json(response)))
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
    let message_type = hl7v2_core::get(message, "MSH.9")
        .unwrap_or("UNKNOWN")
        .to_string();

    let version = hl7v2_core::get(message, "MSH.12")
        .unwrap_or("2.5")
        .to_string();

    let sending_application = hl7v2_core::get(message, "MSH.3")
        .unwrap_or("")
        .to_string();

    let sending_facility = hl7v2_core::get(message, "MSH.4")
        .unwrap_or("")
        .to_string();

    let message_control_id = hl7v2_core::get(message, "MSH.10")
        .unwrap_or("")
        .to_string();

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
