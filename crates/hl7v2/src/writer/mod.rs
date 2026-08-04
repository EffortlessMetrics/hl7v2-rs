//! HL7 v2 message writer/serializer.
//!
//! This module provides serialization functionality for HL7 v2 messages,
//! including:
//! - Converting message structures to HL7 format
//! - MLLP framing for network transmission
//! - JSON serialization
//!
//! # Example
//!
//! ```
//! use hl7v2::{Message, Segment, Field, Rep, Comp, Atom, Delims};
//! use hl7v2::writer::write;
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

use crate::escape::escape_text;
use crate::model::*;

pub mod json;

pub use json::{to_json, to_json_string, to_json_string_pretty};

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
/// use hl7v2::{Message, Delims};
/// use hl7v2::writer::write;
///
/// let message = Message::new();
/// let bytes = write(&message);
/// ```
pub fn write(msg: &Message) -> Vec<u8> {
    let mut buf = Vec::with_capacity(message_capacity(msg));

    write_message_into(msg, &mut buf);

    buf
}

fn write_message_into(msg: &Message, buf: &mut Vec<u8>) {
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
            for field in segment.fields.iter().skip(1) {
                // Skip the encoding characters field
                buf.push(msg.delims.field as u8);
                write_field(buf, field, &msg.delims);
            }
        } else {
            // Write fields
            for field in &segment.fields {
                buf.push(msg.delims.field as u8);
                write_field(buf, field, &msg.delims);
            }
        }

        // End segment with carriage return
        buf.push(b'\r');
    }
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
/// use hl7v2::{Message, Delims};
/// use hl7v2::writer::write_mllp;
///
/// let message = Message::new();
/// let framed = write_mllp(&message);
/// assert_eq!(framed[0], 0x0B); // MLLP start byte
/// ```
pub fn write_mllp(msg: &Message) -> Vec<u8> {
    let hl7_bytes = write(msg);
    crate::transport::mllp::wrap_mllp(&hl7_bytes)
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
    let mut result = Vec::with_capacity(batch_capacity(batch));

    write_batch_into(batch, &mut result);

    result
}

fn write_batch_into(batch: &Batch, result: &mut Vec<u8>) {
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
        write_segment_fields(header, result, delims);
        result.push(b'\r');
    }

    // Write all messages
    for message in &batch.messages {
        write_message_into(message, result);
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
        write_segment_fields(trailer, result, delims);
        result.push(b'\r');
    }
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
    let mut result = Vec::with_capacity(file_batch_capacity(file_batch));

    write_file_batch_into(file_batch, &mut result);

    result
}

fn write_file_batch_into(file_batch: &FileBatch, result: &mut Vec<u8>) {
    // Write FHS if present
    if let Some(header) = &file_batch.header {
        result.extend_from_slice(&header.id);
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(header, result, &delims);
        result.push(b'\r');
    }

    // Write all batches
    for batch in &file_batch.batches {
        write_batch_into(batch, result);
    }

    // Write FTS if present
    if let Some(trailer) = &file_batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(trailer, result, &delims);
        result.push(b'\r');
    }
}

// ============================================================================
// Internal helper functions
// ============================================================================

fn message_capacity(msg: &Message) -> usize {
    msg.segments.iter().fold(0usize, |capacity, segment| {
        capacity.saturating_add(segment_capacity(
            segment,
            &msg.delims,
            segment.id == *b"MSH",
        ))
    })
}

fn batch_capacity(batch: &Batch) -> usize {
    let default_delims = Delims::default();
    let delims = batch
        .messages
        .first()
        .map_or(&default_delims, |message| &message.delims);
    let mut capacity: usize = 0;

    if let Some(header) = &batch.header {
        capacity = capacity.saturating_add(segment_fields_capacity(header, delims));
    }
    for message in &batch.messages {
        capacity = capacity.saturating_add(message_capacity(message));
    }
    if let Some(trailer) = &batch.trailer {
        capacity = capacity.saturating_add(segment_fields_capacity(trailer, delims));
    }

    capacity
}

fn file_batch_capacity(file_batch: &FileBatch) -> usize {
    let delims = get_delimiters_from_file_batch(file_batch);
    let mut capacity: usize = 0;

    if let Some(header) = &file_batch.header {
        capacity = capacity.saturating_add(segment_fields_capacity(header, &delims));
    }
    for batch in &file_batch.batches {
        capacity = capacity.saturating_add(batch_capacity(batch));
    }
    if let Some(trailer) = &file_batch.trailer {
        capacity = capacity.saturating_add(segment_fields_capacity(trailer, &delims));
    }

    capacity
}

fn segment_capacity(segment: &Segment, delims: &Delims, is_msh: bool) -> usize {
    let mut capacity = segment.id.len();
    if is_msh {
        capacity = capacity.saturating_add(5);
        for field in segment.fields.iter().skip(1) {
            capacity = capacity
                .saturating_add(1)
                .saturating_add(field_capacity(field, delims));
        }
    } else {
        for field in &segment.fields {
            capacity = capacity
                .saturating_add(1)
                .saturating_add(field_capacity(field, delims));
        }
    }
    capacity.saturating_add(1)
}

fn segment_fields_capacity(segment: &Segment, delims: &Delims) -> usize {
    let fields_capacity =
        segment
            .fields
            .iter()
            .enumerate()
            .fold(0usize, |capacity, (index, field)| {
                capacity
                    .saturating_add(if index > 0 { 1 } else { 0 })
                    .saturating_add(field_capacity(field, delims))
            });
    segment
        .id
        .len()
        .saturating_add(1)
        .saturating_add(fields_capacity)
        .saturating_add(1)
}

fn field_capacity(field: &Field, delims: &Delims) -> usize {
    field
        .reps
        .iter()
        .enumerate()
        .fold(0usize, |capacity, (index, repetition)| {
            capacity
                .saturating_add(if index > 0 { 1 } else { 0 })
                .saturating_add(rep_capacity(repetition, delims))
        })
}

fn rep_capacity(rep: &Rep, delims: &Delims) -> usize {
    rep.comps
        .iter()
        .enumerate()
        .fold(0usize, |capacity, (index, component)| {
            capacity
                .saturating_add(if index > 0 { 1 } else { 0 })
                .saturating_add(comp_capacity(component, delims))
        })
}

fn comp_capacity(comp: &Comp, delims: &Delims) -> usize {
    comp.subs
        .iter()
        .enumerate()
        .fold(0usize, |capacity, (index, atom)| {
            capacity
                .saturating_add(if index > 0 { 1 } else { 0 })
                .saturating_add(atom_capacity(atom, delims))
        })
}

fn atom_capacity(atom: &Atom, delims: &Delims) -> usize {
    match atom {
        Atom::Text(text) => text.chars().fold(0usize, |capacity, character| {
            let character_capacity = if [
                delims.field,
                delims.comp,
                delims.rep,
                delims.esc,
                delims.sub,
            ]
            .contains(&character)
            {
                delims.esc.len_utf8().saturating_mul(2).saturating_add(1)
            } else {
                character.len_utf8()
            };
            capacity.saturating_add(character_capacity)
        }),
        Atom::Null => 2,
    }
}

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
    if let Some(first_batch) = file_batch.batches.first()
        && let Some(first_message) = first_batch.messages.first()
    {
        return first_message.delims.clone();
    }
    // Fallback to default delimiters
    Delims::default()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests {
    #![expect(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        reason = "pre-existing writer inline test debt moved into hl7v2; cleanup is split from topology collapse"
    )]

    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_write_simple_message() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("SendingApp"),
                    Field::from_text("SendingFac"),
                ],
            }],
            charsets: vec![],
        };

        let bytes = write(&message);
        let result = String::from_utf8(bytes).unwrap();

        assert!(result.starts_with("MSH|"));
        assert!(result.ends_with('\r'));
    }

    #[test]
    fn test_write_with_repetitions() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field {
                        reps: vec![Rep::from_text("1")],
                    },
                    Field {
                        reps: vec![Rep::from_text("12345")],
                    },
                    Field {
                        reps: vec![
                            Rep {
                                comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
                            },
                            Rep {
                                comps: vec![Comp::from_text("Smith"), Comp::from_text("Jane")],
                            },
                        ],
                    },
                ],
            }],
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
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("test|value"), // Contains field separator
                ],
            }],
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
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };

        let framed = write_mllp(&message);

        assert_eq!(framed[0], crate::transport::mllp::MLLP_START);
        assert_eq!(framed[framed.len() - 2], crate::transport::mllp::MLLP_END_1);
        assert_eq!(framed[framed.len() - 1], crate::transport::mllp::MLLP_END_2);
    }

    #[test]
    fn test_to_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&"), Field::from_text("SendingApp")],
            }],
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
                                comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
                            }],
                        },
                    ],
                },
            ],
            charsets: vec![],
        };

        // Write to bytes
        let bytes = write(&original);

        // Parse back through the parser crate and compare key structure.
        let parsed = parse(&bytes).unwrap();

        // Compare
        assert_eq!(original.segments.len(), parsed.segments.len());
        assert_eq!(original.segments[0].id, parsed.segments[0].id);
        assert_eq!(original.segments[1].id, parsed.segments[1].id);
    }
}
