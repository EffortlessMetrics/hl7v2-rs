//! HL7 v2 message writer/serializer.
//!
//! This crate provides serialization functionality for HL7 v2 messages,
//! including:
//! - Converting message structures to HL7 format
//! - MLLP framing for network transmission
//! - JSON serialization
//! - Normalization
//!
//! # Example
//!
//! ```
//! use hl7v2_model::{Message, Segment, Field, Rep, Comp, Atom, Delims};
//! use hl7v2_writer::write;
//!
//! let message = Message {
//!     delims: Delims::default(),
//!     segments: vec![
//!         Segment {
//!             id: *b"MSH",
//!             fields: vec![
//!                 Field::from_text("^~\\&"),  // MSH-2 encoding chars
//!                 Field::from_text("SendingApp"),
//!             ],
//!         },
//!     ],
//!     charsets: vec![],
//! };
//!
//! let bytes = write(&message);
//! assert!(bytes.starts_with(b"MSH|"));
//! ```

use hl7v2_escape::escape_text;
use hl7v2_model::*;
use hl7v2_mllp;

/// Write HL7 message to bytes.
///
/// This function serializes a message structure to its HL7 format.
///
/// # Arguments
///
/// * `msg` - The message to serialize
///
/// # Returns
///
/// The serialized HL7 message bytes
///
/// # Example
///
/// ```
/// use hl7v2_model::{Message, Delims};
/// use hl7v2_writer::write;
///
/// let message = Message::new();
/// let bytes = write(&message);
/// ```
pub fn write(msg: &Message) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Write segments
    for segment in &msg.segments {
        // Write segment ID
        buf.extend_from_slice(&segment.id);
        
        // Special handling for MSH segment
        if &segment.id == b"MSH" {
            // Write field separator
            buf.push(msg.delims.field as u8);
            
            // Write encoding characters as a single field
            buf.push(msg.delims.comp as u8);
            buf.push(msg.delims.rep as u8);
            buf.push(msg.delims.esc as u8);
            buf.push(msg.delims.sub as u8);
            
            // Write the rest of the fields
            for field in &segment.fields[1..] { // Skip the encoding characters field
                buf.push(msg.delims.field as u8);
                write_field(&mut buf, field, &msg.delims);
            }
        } else {
            // Write fields
            for field in &segment.fields {
                buf.push(msg.delims.field as u8);
                write_field(&mut buf, field, &msg.delims);
            }
        }
        
        // End segment with carriage return
        buf.push(b'\r');
    }
    
    buf
}

/// Write HL7 message with MLLP framing.
///
/// This function serializes a message and wraps it with MLLP framing.
///
/// # Arguments
///
/// * `msg` - The message to serialize
///
/// # Returns
///
/// The MLLP-framed HL7 message bytes
///
/// # Example
///
/// ```
/// use hl7v2_model::{Message, Delims};
/// use hl7v2_writer::write_mllp;
///
/// let message = Message::new();
/// let framed = write_mllp(&message);
/// assert_eq!(framed[0], 0x0B); // MLLP start byte
/// ```
pub fn write_mllp(msg: &Message) -> Vec<u8> {
    let hl7_bytes = write(msg);
    hl7v2_mllp::wrap_mllp(&hl7_bytes)
}

/// Write batch to bytes.
///
/// # Arguments
///
/// * `batch` - The batch to serialize
///
/// # Returns
///
/// The serialized HL7 batch bytes
pub fn write_batch(batch: &Batch) -> Vec<u8> {
    let mut result = Vec::new();
    
    // Write BHS if present
    if let Some(header) = &batch.header {
        result.extend_from_slice(&header.id);
        // We need to get delimiters from the first message or use defaults
        let delims = if let Some(first_msg) = batch.messages.first() {
            &first_msg.delims
        } else {
            &Delims::default()
        };
        result.push(delims.field as u8);
        write_segment_fields(header, &mut result, delims);
        result.push(b'\r');
    }
    
    // Write all messages
    for message in &batch.messages {
        result.extend(write(message));
    }
    
    // Write BTS if present
    if let Some(trailer) = &batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = if let Some(first_msg) = batch.messages.first() {
            &first_msg.delims
        } else {
            &Delims::default()
        };
        result.push(delims.field as u8);
        write_segment_fields(trailer, &mut result, delims);
        result.push(b'\r');
    }
    
    result
}

/// Write file batch to bytes.
///
/// # Arguments
///
/// * `file_batch` - The file batch to serialize
///
/// # Returns
///
/// The serialized HL7 file batch bytes
pub fn write_file_batch(file_batch: &FileBatch) -> Vec<u8> {
    let mut result = Vec::new();
    
    // Write FHS if present
    if let Some(header) = &file_batch.header {
        result.extend_from_slice(&header.id);
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(header, &mut result, &delims);
        result.push(b'\r');
    }
    
    // Write all batches
    for batch in &file_batch.batches {
        result.extend(write_batch(batch));
    }
    
    // Write FTS if present
    if let Some(trailer) = &file_batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(trailer, &mut result, &delims);
        result.push(b'\r');
    }
    
    result
}

/// Normalize HL7 v2 message.
///
/// This function parses and rewrites an HL7 message, optionally converting
/// it to canonical delimiters (|^~\&).
///
/// # Arguments
///
/// * `bytes` - The raw HL7 message bytes
/// * `canonical_delims` - Whether to convert to canonical delimiters
///
/// # Returns
///
/// The normalized HL7 message bytes
pub fn normalize(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the message using the parser crate
    let mut message = parse_for_normalize(bytes)?;
    
    // If canonical delimiters are requested, update the message delimiters
    if canonical_delims {
        message.delims = Delims::default();
    }
    
    // Write the normalized message
    Ok(write(&message))
}

/// Convert message to canonical JSON.
///
/// # Arguments
///
/// * `msg` - The message to convert
///
/// # Returns
///
/// A JSON representation of the message
pub fn to_json(msg: &Message) -> serde_json::Value {
    use serde_json::json;
    
    let segments: Vec<serde_json::Value> = msg
        .segments
        .iter()
        .map(|segment| {
            let segment_id = String::from_utf8_lossy(&segment.id).to_string();
            let fields: serde_json::Map<String, serde_json::Value> = segment
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    if field.reps.is_empty() {
                        None
                    } else {
                        let field_value = field_to_json(field);
                        Some(((index + 1).to_string(), field_value))
                    }
                })
                .collect();
            
            json!({
                "id": segment_id,
                "fields": fields
            })
        })
        .collect();
    
    json!({
        "meta": {
            "delims": {
                "field": msg.delims.field.to_string(),
                "comp": msg.delims.comp.to_string(),
                "rep": msg.delims.rep.to_string(),
                "esc": msg.delims.esc.to_string(),
                "sub": msg.delims.sub.to_string()
            },
            "charsets": msg.charsets
        },
        "segments": segments
    })
}

/// Convert message to JSON string.
///
/// # Arguments
///
/// * `msg` - The message to convert
///
/// # Returns
///
/// A JSON string representation of the message
pub fn to_json_string(msg: &Message) -> String {
    serde_json::to_string(&to_json(msg)).unwrap_or_default()
}

/// Convert message to pretty JSON string.
///
/// # Arguments
///
/// * `msg` - The message to convert
///
/// # Returns
///
/// A pretty-printed JSON string representation of the message
pub fn to_json_string_pretty(msg: &Message) -> String {
    serde_json::to_string_pretty(&to_json(msg)).unwrap_or_default()
}

// ============================================================================
// Internal helper functions
// ============================================================================

/// Write a field to bytes (with escaping)
fn write_field(output: &mut Vec<u8>, field: &Field, delims: &Delims) {
    for (i, rep) in field.reps.iter().enumerate() {
        if i > 0 {
            output.push(delims.rep as u8);
        }
        write_rep(output, rep, delims);
    }
}

/// Write a repetition to bytes (with escaping)
fn write_rep(output: &mut Vec<u8>, rep: &Rep, delims: &Delims) {
    for (i, comp) in rep.comps.iter().enumerate() {
        if i > 0 {
            output.push(delims.comp as u8);
        }
        write_comp(output, comp, delims);
    }
}

/// Write a component to bytes (with escaping)
fn write_comp(output: &mut Vec<u8>, comp: &Comp, delims: &Delims) {
    for (i, atom) in comp.subs.iter().enumerate() {
        if i > 0 {
            output.push(delims.sub as u8);
        }
        write_atom(output, atom, delims);
    }
}

/// Write an atom to bytes (with escaping)
fn write_atom(output: &mut Vec<u8>, atom: &Atom, delims: &Delims) {
    match atom {
        Atom::Text(text) => {
            // Escape special characters
            let escaped = escape_text(text, delims);
            output.extend_from_slice(escaped.as_bytes());
        }
        Atom::Null => {
            output.extend_from_slice(b"\"\"");
        }
    }
}

/// Helper function to write segment fields (without segment ID)
fn write_segment_fields(segment: &Segment, output: &mut Vec<u8>, delims: &Delims) {
    for (i, field) in segment.fields.iter().enumerate() {
        if i > 0 {
            output.push(delims.field as u8);
        }
        write_field(output, field, delims);
    }
}

/// Helper function to get delimiters from a file batch
fn get_delimiters_from_file_batch(file_batch: &FileBatch) -> Delims {
    // Try to get delimiters from the first message in the first batch
    if let Some(first_batch) = file_batch.batches.first() {
        if let Some(first_message) = first_batch.messages.first() {
            return first_message.delims.clone();
        }
    }
    // Fallback to default delimiters
    Delims::default()
}

/// Parse message for normalization (simplified inline parser)
fn parse_for_normalize(bytes: &[u8]) -> Result<Message, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    
    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();
    
    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }
    
    // First segment must be MSH
    if !lines[0].starts_with("MSH") {
        return Err(Error::InvalidSegmentId);
    }
    
    // Parse delimiters from MSH segment
    let delims = Delims::parse_from_msh(lines[0])?;
    
    // Parse all segments (simplified)
    let mut segments = Vec::new();
    for line in lines {
        let segment = parse_segment_simple(line, &delims)?;
        segments.push(segment);
    }
    
    Ok(Message { delims, segments, charsets: vec![] })
}

/// Simplified segment parsing for normalization
fn parse_segment_simple(line: &str, delims: &Delims) -> Result<Segment, Error> {
    if line.len() < 3 {
        return Err(Error::InvalidSegmentId);
    }
    
    let id_bytes = line[0..3].as_bytes();
    let mut id = [0u8; 3];
    id.copy_from_slice(id_bytes);
    
    // Validate segment ID
    for &byte in &id {
        if !((byte >= b'A' && byte <= b'Z') || (byte >= b'0' && byte <= b'9')) {
            return Err(Error::InvalidSegmentId);
        }
    }
    
    let fields_str = if line.len() > 4 { &line[4..] } else { "" };
    let mut fields = Vec::new();
    
    for field_str in fields_str.split(delims.field) {
        fields.push(parse_field_simple(field_str, delims)?);
    }
    
    // Special handling for MSH
    if &id == b"MSH" && !fields.is_empty() {
        let encoding_chars = String::from_iter([
            delims.comp, delims.rep, delims.esc, delims.sub
        ]);
        fields[0] = Field::from_text(encoding_chars);
    }
    
    Ok(Segment { id, fields })
}

/// Simplified field parsing
fn parse_field_simple(field_str: &str, delims: &Delims) -> Result<Field, Error> {
    let mut reps = Vec::new();
    for rep_str in field_str.split(delims.rep) {
        reps.push(parse_rep_simple(rep_str, delims)?);
    }
    Ok(Field { reps })
}

/// Simplified repetition parsing
fn parse_rep_simple(rep_str: &str, delims: &Delims) -> Result<Rep, Error> {
    let mut comps = Vec::new();
    for comp_str in rep_str.split(delims.comp) {
        comps.push(parse_comp_simple(comp_str, delims)?);
    }
    Ok(Rep { comps })
}

/// Simplified component parsing
fn parse_comp_simple(comp_str: &str, delims: &Delims) -> Result<Comp, Error> {
    let mut subs = Vec::new();
    for sub_str in comp_str.split(delims.sub) {
        if sub_str == "\"\"" {
            subs.push(Atom::Null);
        } else {
            subs.push(Atom::Text(sub_str.to_string()));
        }
    }
    Ok(Comp { subs })
}

/// Convert a field to JSON
fn field_to_json(field: &Field) -> serde_json::Value {
    use serde_json::json;
    
    let reps: Vec<serde_json::Value> = field
        .reps
        .iter()
        .map(|rep| {
            let comps: Vec<serde_json::Value> = rep
                .comps
                .iter()
                .map(|comp| {
                    let subs: Vec<serde_json::Value> = comp
                        .subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => json!(text),
                            Atom::Null => json!("__NULL__"),
                        })
                        .collect();
                    json!(subs)
                })
                .collect();
            json!(comps)
        })
        .collect();
    
    json!(reps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_simple_message() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"MSH",
                    fields: vec![
                        Field::from_text("^~\\&"),
                        Field::from_text("SendingApp"),
                        Field::from_text("SendingFac"),
                    ],
                },
            ],
            charsets: vec![],
        };
        
        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();
        
        assert!(result.starts_with("MSH|"));
        assert!(result.ends_with("\r"));
    }

    #[test]
    fn test_write_with_repetitions() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"PID",
                    fields: vec![
                        Field {
                            reps: vec![
                                Rep::from_text("1"),
                            ],
                        },
                        Field {
                            reps: vec![
                                Rep::from_text("12345"),
                            ],
                        },
                        Field {
                            reps: vec![
                                Rep {
                                    comps: vec![
                                        Comp::from_text("Doe"),
                                        Comp::from_text("John"),
                                    ],
                                },
                                Rep {
                                    comps: vec![
                                        Comp::from_text("Smith"),
                                        Comp::from_text("Jane"),
                                    ],
                                },
                            ],
                        },
                    ],
                },
            ],
            charsets: vec![],
        };
        
        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();
        
        // Check for repetition separator
        assert!(result.contains("Doe^John~Smith^Jane"));
    }

    #[test]
    fn test_write_with_escaping() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"PID",
                    fields: vec![
                        Field::from_text("1"),
                        Field::from_text("test|value"),  // Contains field separator
                    ],
                },
            ],
            charsets: vec![],
        };
        
        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();
        
        // The field separator should be escaped
        assert!(result.contains("test\\F\\value"));
    }

    #[test]
    fn test_write_mllp() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"MSH",
                    fields: vec![Field::from_text("^~\\&")],
                },
            ],
            charsets: vec![],
        };
        
        let framed = write_mllp(&message);
        
        assert_eq!(framed[0], hl7v2_mllp::MLLP_START);
        assert_eq!(framed[framed.len() - 2], hl7v2_mllp::MLLP_END_1);
        assert_eq!(framed[framed.len() - 1], hl7v2_mllp::MLLP_END_2);
    }

    #[test]
    fn test_to_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"MSH",
                    fields: vec![
                        Field::from_text("^~\\&"),
                        Field::from_text("SendingApp"),
                    ],
                },
            ],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        assert!(json.is_object());
        assert!(json.get("meta").is_some());
        assert!(json.get("segments").is_some());
        
        let meta = json.get("meta").unwrap();
        assert!(meta.get("delims").is_some());
    }

    #[test]
    fn test_roundtrip() {
        // Create a message
        let original = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"MSH",
                    fields: vec![
                        Field::from_text("^~\\&"),
                        Field::from_text("SendingApp"),
                        Field::from_text("SendingFac"),
                    ],
                },
                Segment {
                    id: *b"PID",
                    fields: vec![
                        Field::from_text("1"),
                        Field::from_text("12345"),
                        Field {
                            reps: vec![Rep {
                                comps: vec![
                                    Comp::from_text("Doe"),
                                    Comp::from_text("John"),
                                ],
                            }],
                        },
                    ],
                },
            ],
            charsets: vec![],
        };
        
        // Write to bytes
        let bytes = write(&original);
        
        // Parse back (using the normalize function which has inline parsing)
        let parsed = parse_for_normalize(&bytes).unwrap();
        
        // Compare
        assert_eq!(original.segments.len(), parsed.segments.len());
        assert_eq!(original.segments[0].id, parsed.segments[0].id);
        assert_eq!(original.segments[1].id, parsed.segments[1].id);
    }

    #[test]
    fn test_normalize() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
        let normalized = normalize(hl7, false).unwrap();
        
        // Should produce valid output
        assert!(!normalized.is_empty());
        assert!(normalized.starts_with(b"MSH|"));
    }
}