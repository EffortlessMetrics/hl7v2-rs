use crate::handlers::error::AppError;
use crate::models::MessageMetadata;

/// Extract message metadata from parsed message
pub(super) fn extract_metadata(message: &hl7v2::Message) -> Result<MessageMetadata, AppError> {
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

    let version = hl7v2::get(message, "MSH.12").unwrap_or("2.5").to_string();

    let sending_application = hl7v2::get(message, "MSH.3").unwrap_or("").to_string();

    let sending_facility = hl7v2::get(message, "MSH.4").unwrap_or("").to_string();

    let message_control_id = hl7v2::get(message, "MSH.10").unwrap_or("").to_string();

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

pub(super) fn joined_components(message: &hl7v2::Message, path: &str) -> Option<String> {
    let mut components = Vec::new();

    for index in 1.. {
        let component_path = format!("{}.{}", path, index);
        match hl7v2::get(message, &component_path) {
            Some(value) if !value.is_empty() => components.push(value.to_string()),
            Some(_) => {}
            None => break,
        }
    }

    if components.is_empty() {
        hl7v2::get(message, path).map(str::to_string)
    } else {
        Some(components.join("^"))
    }
}
