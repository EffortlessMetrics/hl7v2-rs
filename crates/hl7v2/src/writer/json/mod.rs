//! HL7 v2 JSON serialization.
//!
//! This module provides JSON serialization functionality for HL7 v2 messages,
//! converting message structures to JSON format.
//!
//! # Example
//!
//! ```
//! use hl7v2::{Message, Segment, Field, Delims};
//! use hl7v2::writer::json::to_json;
//!
//! let message = Message {
//!     delims: Delims::default(),
//!     segments: vec![
//!         Segment {
//!             id: *b"MSH",
//!             fields: vec![
//!                 Field::from_text("^~\\&"),
//!                 Field::from_text("SendingApp"),
//!             ],
//!         },
//!     ],
//!     charsets: vec![],
//! };
//!
//! let json = to_json(&message);
//! assert!(json.is_object());
//! ```

#![expect(
    clippy::arithmetic_side_effects,
    reason = "pre-existing JSON writer implementation debt moved from staged microcrate into hl7v2; cleanup is split from topology collapse"
)]

use crate::model::*;
use serde_json::json;

/// Errors returned while converting the canonical JSON representation back to
/// an HL7 message.
#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    /// The input was not valid JSON text.
    #[error("invalid JSON syntax: {0}")]
    Syntax(#[from] serde_json::Error),

    /// A required JSON member was not present.
    #[error("missing JSON member: {0}")]
    Missing(String),

    /// A JSON value had a different type than the canonical representation
    /// requires.
    #[error("invalid JSON at {path}: expected {expected}")]
    InvalidType {
        path: String,
        expected: &'static str,
    },

    /// A JSON value could not be represented by the HL7 model.
    #[error("invalid JSON at {path}: {message}")]
    Invalid { path: String, message: String },
}

/// Convert the canonical JSON representation produced by [`to_json`] back to
/// an HL7 message.
///
/// The representation uses numeric field keys (`"1"`, `"2"`, and so on),
/// with MSH fields numbered according to their HL7 positions. Missing keys are
/// reconstructed as empty fields. The `"__NULL__"` atom marker is converted
/// back to [`Atom::Null`].
///
/// # Errors
///
/// Returns [`JsonError`] when required members are missing, values have the
/// wrong type, or delimiters, segment IDs, or field numbers are invalid.
pub fn from_json(value: &serde_json::Value) -> Result<Message, JsonError> {
    let root = object(value, "message")?;
    let meta = object(member(root, "meta", "message")?, "message.meta")?;
    let delimiters = object(
        member(meta, "delims", "message.meta")?,
        "message.meta.delims",
    )?;

    let delims = Delims {
        field: delimiter(delimiters, "field")?,
        comp: delimiter(delimiters, "comp")?,
        rep: delimiter(delimiters, "rep")?,
        esc: delimiter(delimiters, "esc")?,
        sub: delimiter(delimiters, "sub")?,
    };
    let delimiter_values = [
        delims.field,
        delims.comp,
        delims.rep,
        delims.esc,
        delims.sub,
    ];
    if delimiter_values
        .iter()
        .enumerate()
        .any(|(index, delimiter)| {
            delimiter_values
                .get(..index)
                .is_some_and(|prefix| prefix.contains(delimiter))
        })
    {
        return Err(invalid(
            "message.meta.delims",
            "delimiter characters must be distinct",
        ));
    }

    let charsets = match meta.get("charsets") {
        Some(value) => strings(value, "message.meta.charsets")?,
        None => Vec::new(),
    };

    let segments = array(member(root, "segments", "message")?, "message.segments")?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_segment(value, index))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Message {
        delims,
        segments,
        charsets,
    })
}

/// Parse JSON text containing the canonical representation produced by
/// [`to_json`].
///
/// # Errors
///
/// Returns [`JsonError::Syntax`] for invalid JSON text or another
/// [`JsonError`] when the decoded value is not a valid canonical message.
pub fn from_json_string(input: &str) -> Result<Message, JsonError> {
    let value = serde_json::from_str(input)?;
    from_json(&value)
}

fn parse_segment(value: &serde_json::Value, segment_index: usize) -> Result<Segment, JsonError> {
    let path = format!("message.segments[{segment_index}]");
    let segment = object(value, &path)?;
    let id_path = format!("{path}.id");
    let id = string(member(segment, "id", &path)?, &id_path)?;
    let id_bytes = id.as_bytes();
    if id_bytes.len() != 3 || !id_bytes.iter().all(u8::is_ascii) {
        return Err(invalid(
            id_path,
            "segment IDs must contain exactly three ASCII bytes",
        ));
    }
    let id = id_bytes
        .try_into()
        .map_err(|_error| invalid(format!("{path}.id"), "segment ID must be three bytes"))?;

    let fields_path = format!("{path}.fields");
    let fields_object = object(member(segment, "fields", &path)?, &fields_path)?;
    let mut indexed_fields = Vec::with_capacity(fields_object.len());
    for (field_number, value) in fields_object {
        let field_path = format!("{fields_path}.{field_number}");
        let field_number = field_number.parse::<usize>().map_err(|_error| {
            invalid(
                field_path.clone(),
                "field keys must be positive decimal integers",
            )
        })?;
        let field_index = if id == *b"MSH" {
            field_number.checked_sub(2)
        } else {
            field_number.checked_sub(1)
        }
        .ok_or_else(|| {
            invalid(
                field_path.clone(),
                "field numbers must start at 2 for MSH and 1 for other segments",
            )
        })?;
        indexed_fields.push((field_index, parse_field(value, &field_path)?));
    }

    let fields = match indexed_fields.iter().map(|(index, _)| *index).max() {
        Some(max_index) => {
            let field_count = max_index
                .checked_add(1)
                .ok_or_else(|| invalid(fields_path.clone(), "field index is too large"))?;
            let mut fields = vec![Field::new(); field_count];
            for (index, field) in indexed_fields {
                let slot = fields.get_mut(index).ok_or_else(|| {
                    invalid(fields_path.clone(), "field index is outside the message")
                })?;
                *slot = field;
            }
            fields
        }
        None => Vec::new(),
    };

    Ok(Segment { id, fields })
}

fn parse_field(value: &serde_json::Value, path: &str) -> Result<Field, JsonError> {
    let reps = array(value, path)?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_rep(value, &format!("{path}[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Field { reps })
}

fn parse_rep(value: &serde_json::Value, path: &str) -> Result<Rep, JsonError> {
    let comps = array(value, path)?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_comp(value, &format!("{path}[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Rep { comps })
}

fn parse_comp(value: &serde_json::Value, path: &str) -> Result<Comp, JsonError> {
    let subs = array(value, path)?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let atom_path = format!("{path}[{index}]");
            let atom = string(value, &atom_path)?;
            Ok(if atom == "__NULL__" {
                Atom::Null
            } else {
                Atom::Text(atom.to_owned())
            })
        })
        .collect::<Result<Vec<_>, JsonError>>()?;
    Ok(Comp { subs })
}

fn member<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
    path: &str,
) -> Result<&'a serde_json::Value, JsonError> {
    object
        .get(key)
        .ok_or_else(|| JsonError::Missing(format!("{path}.{key}")))
}

fn object<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, JsonError> {
    value.as_object().ok_or_else(|| JsonError::InvalidType {
        path: path.to_owned(),
        expected: "an object",
    })
}

fn array<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a Vec<serde_json::Value>, JsonError> {
    value.as_array().ok_or_else(|| JsonError::InvalidType {
        path: path.to_owned(),
        expected: "an array",
    })
}

fn strings(value: &serde_json::Value, path: &str) -> Result<Vec<String>, JsonError> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(index, value)| string(value, &format!("{path}[{index}]")).map(str::to_owned))
        .collect()
}

fn string<'a>(value: &'a serde_json::Value, path: &str) -> Result<&'a str, JsonError> {
    value.as_str().ok_or_else(|| JsonError::InvalidType {
        path: path.to_owned(),
        expected: "a string",
    })
}

fn delimiter(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<char, JsonError> {
    let path = format!("message.meta.delims.{name}");
    let value = string(member(object, name, "message.meta.delims")?, &path)?;
    let mut chars = value.chars();
    let delimiter = chars
        .next()
        .ok_or_else(|| invalid(path.clone(), "delimiter must be one ASCII character"))?;
    if chars.next().is_some() || !delimiter.is_ascii() || matches!(delimiter, '\r' | '\n') {
        return Err(invalid(path, "delimiter must be one ASCII character"));
    }
    Ok(delimiter)
}

fn invalid(path: impl Into<String>, message: impl Into<String>) -> JsonError {
    JsonError::Invalid {
        path: path.into(),
        message: message.into(),
    }
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
///
/// # Example
///
/// ```
/// use hl7v2::{Message, Segment, Field, Delims};
/// use hl7v2::writer::json::to_json;
///
/// let message = Message {
///     delims: Delims::default(),
///     segments: vec![
///         Segment {
///             id: *b"MSH",
///             fields: vec![Field::from_text("^~\\&")],
///         },
///     ],
///     charsets: vec![],
/// };
///
/// let json = to_json(&message);
/// assert!(json.get("meta").is_some());
/// assert!(json.get("segments").is_some());
/// ```
pub fn to_json(msg: &Message) -> serde_json::Value {
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
                        let field_number = if segment.id == *b"MSH" {
                            index + 2
                        } else {
                            index + 1
                        };
                        Some((field_number.to_string(), field_value))
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
///
/// # Example
///
/// ```
/// use hl7v2::{Message, Segment, Field, Delims};
/// use hl7v2::writer::json::to_json_string;
///
/// let message = Message {
///     delims: Delims::default(),
///     segments: vec![
///         Segment {
///             id: *b"MSH",
///             fields: vec![Field::from_text("^~\\&")],
///         },
///     ],
///     charsets: vec![],
/// };
///
/// let json_str = to_json_string(&message);
/// assert!(json_str.starts_with('{'));
/// ```
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
///
/// # Example
///
/// ```
/// use hl7v2::{Message, Segment, Field, Delims};
/// use hl7v2::writer::json::to_json_string_pretty;
///
/// let message = Message {
///     delims: Delims::default(),
///     segments: vec![
///         Segment {
///             id: *b"MSH",
///             fields: vec![Field::from_text("^~\\&")],
///         },
///     ],
///     charsets: vec![],
/// };
///
/// let json_str = to_json_string_pretty(&message);
/// assert!(json_str.contains('\n')); // Pretty-printed has newlines
/// ```
pub fn to_json_string_pretty(msg: &Message) -> String {
    serde_json::to_string_pretty(&to_json(msg)).unwrap_or_default()
}

// ============================================================================
// Internal helper functions
// ============================================================================

/// Convert a field to JSON
fn field_to_json(field: &Field) -> serde_json::Value {
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
mod tests;
