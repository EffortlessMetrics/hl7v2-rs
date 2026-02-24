//! HL7 v2 JSON serialization.
//!
//! This crate provides JSON serialization functionality for HL7 v2 messages,
//! converting message structures to JSON format.
//!
//! # Example
//!
//! ```
//! use hl7v2_model::{Message, Segment, Field, Delims};
//! use hl7v2_json::to_json;
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

use hl7v2_model::*;
use serde_json::json;

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
/// use hl7v2_model::{Message, Segment, Field, Delims};
/// use hl7v2_json::to_json;
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
///
/// # Example
///
/// ```
/// use hl7v2_model::{Message, Segment, Field, Delims};
/// use hl7v2_json::to_json_string;
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
/// use hl7v2_model::{Message, Segment, Field, Delims};
/// use hl7v2_json::to_json_string_pretty;
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
mod tests {
    use super::*;

    #[test]
    fn test_to_json_simple_message() {
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
    fn test_to_json_with_multiple_segments() {
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
                Segment {
                    id: *b"PID",
                    fields: vec![Field::from_text("1"), Field::from_text("12345")],
                },
            ],
            charsets: vec![],
        };

        let json = to_json(&message);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_to_json_with_repetitions() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
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

        let json = to_json(&message);
        assert!(json.is_object());
    }

    #[test]
    fn test_to_json_string() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string(&message);
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
    }

    #[test]
    fn test_to_json_string_pretty() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string_pretty(&message);
        assert!(json_str.contains('\n'));
        assert!(json_str.starts_with('{'));
    }

    #[test]
    fn test_to_json_with_null_atom() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Null],
                            }],
                        }],
                    },
                ],
            }],
            charsets: vec![],
        };

        let json_str = to_json_string(&message);
        assert!(json_str.contains("__NULL__"));
    }

    #[test]
    fn test_to_json_empty_message() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };

        let json = to_json(&message);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn test_to_json_with_charsets() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec!["ASCII".to_string()],
        };

        let json = to_json(&message);
        let meta = json.get("meta").unwrap();
        let charsets = meta.get("charsets").unwrap().as_array().unwrap();
        assert_eq!(charsets.len(), 1);
        assert_eq!(charsets[0], "ASCII");
    }
}
