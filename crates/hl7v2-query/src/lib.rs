//! HL7 v2 path-based field access and query functionality.
//!
//! This crate provides query functionality for HL7 v2 messages,
//! including:
//! - Path-based field access via [`get`]
//! - Presence semantics via [`get_presence`]
//!
//! # Path Format
//!
//! Paths use the format: `SEGMENT.FIELD[REP].COMPONENT`
//!
//! Examples:
//! - `PID.5.1` - First component of 5th field in PID segment (first repetition)
//! - `PID.5[2].1` - First component of 5th field, second repetition
//! - `MSH.9` - 9th field of MSH segment
//! - `MSH.9.1` - First component of 9th field of MSH segment
//!
//! # Example
//!
//! ```
//! use hl7v2_model::Message;
//! use hl7v2_query::get;
//!
//! // Assuming you have a parsed Message from hl7v2-parser
//! // let message = hl7v2_parser::parse(hl7_bytes).unwrap();
//! // let last_name = get(&message, "PID.5.1").unwrap();
//! ```

use hl7v2_model::{Atom, Message, Presence, Segment};

/// Get value at path (e.g., "PID.5[1].1")
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - The path to the field (e.g., "PID.5.1", "PID.5[1].1", "MSH.9")
///
/// # Returns
///
/// The value at the path, or `None` if not found
///
/// # Example
///
/// ```
/// use hl7v2_query::get;
/// use hl7v2_model::{Message, Segment, Field, Rep, Comp, Atom, Delims};
///
/// // Create a minimal message for testing
/// let message = Message {
///     delims: Delims::default(),
///     segments: vec![],
///     charsets: vec![],
/// };
///
/// // Returns None for missing segment
/// assert!(get(&message, "PID.5.1").is_none());
/// ```
pub fn get<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    // Parse the path
    // Format: SEGMENT.FIELD[REP].COMPONENT
    // Examples: "PID.5.1", "PID.5[1].1", "MSH.9"

    let mut parts = path.split('.');
    let segment_id = parts.next()?;

    // Find the segment
    let segment = msg
        .segments
        .iter()
        .find(|s| std::str::from_utf8(&s.id).map_or(false, |id| id == segment_id))?;

    // Parse field index (1-based)
    let field_part = parts.next()?;
    let (field_index, rep_index) = parse_field_and_rep(field_part)?;

    // Special handling for MSH segments
    if segment_id == "MSH" {
        get_msh_field(msg, segment, field_index, rep_index, parts)
    } else {
        get_field(segment, field_index, rep_index, parts)
    }
}

/// Get presence semantics for a field at path.
///
/// Presence semantics distinguish between:
/// - `Presence::Value(String)` - Field exists with a value
/// - `Presence::Empty` - Field exists but is empty
/// - `Presence::Null` - Field is explicitly null (HL7 null value: "")
/// - `Presence::Missing` - Field does not exist
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - The path to the field
///
/// # Returns
///
/// The presence status of the field
///
/// # Example
///
/// ```
/// use hl7v2_query::get_presence;
/// use hl7v2_model::{Message, Delims, Presence};
///
/// let message = Message {
///     delims: Delims::default(),
///     segments: vec![],
///     charsets: vec![],
/// };
///
/// assert!(matches!(get_presence(&message, "PID.5.1"), Presence::Missing));
/// ```
pub fn get_presence(msg: &Message, path: &str) -> Presence {
    // Parse the path
    let mut parts = path.split('.');
    let segment_id = match parts.next() {
        Some(id) => id,
        None => return Presence::Missing,
    };

    // Find the segment
    let segment = match msg
        .segments
        .iter()
        .find(|s| std::str::from_utf8(&s.id).map_or(false, |id| id == segment_id))
    {
        Some(seg) => seg,
        None => return Presence::Missing,
    };

    // Parse field index (1-based)
    let field_part = match parts.next() {
        Some(part) => part,
        None => return Presence::Missing,
    };

    let (field_index, rep_index) = match parse_field_and_rep(field_part) {
        Some(indices) => indices,
        None => return Presence::Missing,
    };

    // Special handling for MSH segments
    if segment_id == "MSH" {
        get_msh_field_presence(msg, segment, field_index, rep_index, parts)
    } else {
        get_field_presence(segment, field_index, rep_index, parts)
    }
}

// ============================================================================
// Internal helper functions
// ============================================================================

/// Parse field and repetition indices from a string like "5" or "5[1]"
fn parse_field_and_rep(field_str: &str) -> Option<(usize, usize)> {
    if let Some(bracket_pos) = field_str.find('[') {
        // Has repetition index
        let field_index = field_str[..bracket_pos].parse::<usize>().ok()?;
        let rep_part = &field_str[bracket_pos + 1..];
        if let Some(end_bracket) = rep_part.find(']') {
            let rep_index = rep_part[..end_bracket].parse::<usize>().ok()?;
            Some((field_index, rep_index))
        } else {
            None
        }
    } else {
        // No repetition index, default to 1
        let field_index = field_str.parse::<usize>().ok()?;
        Some((field_index, 1))
    }
}

/// Get field value from a non-MSH segment
fn get_field<'a>(
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Option<&'a str> {
    // Convert to 0-based indexing
    if field_index == 0 {
        return None;
    }
    let zero_based_field_index = field_index - 1;

    // Get the field
    if zero_based_field_index >= segment.fields.len() {
        return None;
    }
    let field = &segment.fields[zero_based_field_index];

    // Get the repetition
    if rep_index == 0 || rep_index > field.reps.len() {
        return None;
    }
    let rep = &field.reps[rep_index - 1];

    // Parse component index if provided
    let comp_index = if let Some(comp_part) = parts.next() {
        comp_part.parse::<usize>().ok()?
    } else {
        1 // Default to first component
    };

    // Get the component
    if comp_index == 0 || comp_index > rep.comps.len() {
        return None;
    }
    let comp = &rep.comps[comp_index - 1];

    // Get the first subcomponent as text
    if comp.subs.is_empty() {
        return None;
    }

    match &comp.subs[0] {
        Atom::Text(text) => Some(text.as_str()),
        Atom::Null => None,
    }
}

/// Get field value from an MSH segment
fn get_msh_field<'a>(
    _msg: &'a Message,
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Option<&'a str> {
    if field_index == 1 {
        // MSH-1 is the field separator character
        return None; // We can't return a reference to a temporary
    } else if field_index == 2 {
        // MSH-2 is the encoding characters
        if segment.fields.is_empty() {
            return None;
        }
        let field = &segment.fields[0];
        if rep_index == 0 || rep_index > field.reps.len() {
            return None;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            comp_part.parse::<usize>().ok()?
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return None;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return None;
        }
        match &comp.subs[0] {
            Atom::Text(text) => Some(text.as_str()),
            Atom::Null => None,
        }
    } else {
        // MSH-3 and beyond
        let adjusted_field_index = field_index - 2;
        if adjusted_field_index >= segment.fields.len() {
            return None;
        }
        let field = &segment.fields[adjusted_field_index];
        if rep_index == 0 || rep_index > field.reps.len() {
            return None;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            comp_part.parse::<usize>().ok()?
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return None;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return None;
        }
        match &comp.subs[0] {
            Atom::Text(text) => Some(text.as_str()),
            Atom::Null => None,
        }
    }
}

/// Get field presence from a non-MSH segment
fn get_field_presence(
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Presence {
    if field_index == 0 {
        return Presence::Missing;
    }
    let zero_based_field_index = field_index - 1;

    if zero_based_field_index >= segment.fields.len() {
        return Presence::Missing;
    }
    let field = &segment.fields[zero_based_field_index];

    if rep_index == 0 || rep_index > field.reps.len() {
        return Presence::Missing;
    }
    let rep = &field.reps[rep_index - 1];

    let comp_index = if let Some(comp_part) = parts.next() {
        match comp_part.parse::<usize>() {
            Ok(index) => index,
            Err(_) => return Presence::Missing,
        }
    } else {
        1
    };

    if comp_index == 0 || comp_index > rep.comps.len() {
        return Presence::Missing;
    }
    let comp = &rep.comps[comp_index - 1];

    if comp.subs.is_empty() {
        return Presence::Missing;
    }

    match &comp.subs[0] {
        Atom::Text(text) => {
            if text.is_empty() {
                Presence::Empty
            } else {
                Presence::Value(text.clone())
            }
        }
        Atom::Null => Presence::Null,
    }
}

/// Get field presence from an MSH segment
fn get_msh_field_presence(
    msg: &Message,
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Presence {
    if field_index == 1 {
        // MSH-1 is the field separator character
        return Presence::Value(msg.delims.field.to_string());
    } else if field_index == 2 {
        if segment.fields.is_empty() {
            return Presence::Missing;
        }
        let field = &segment.fields[0];
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            match comp_part.parse::<usize>() {
                Ok(index) => index,
                Err(_) => return Presence::Missing,
            }
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return Presence::Missing;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return Presence::Missing;
        }
        match &comp.subs[0] {
            Atom::Text(text) => {
                if text.is_empty() {
                    Presence::Empty
                } else {
                    Presence::Value(text.clone())
                }
            }
            Atom::Null => Presence::Null,
        }
    } else {
        let adjusted_field_index = field_index - 2;
        if adjusted_field_index >= segment.fields.len() {
            return Presence::Missing;
        }
        let field = &segment.fields[adjusted_field_index];
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            match comp_part.parse::<usize>() {
                Ok(index) => index,
                Err(_) => return Presence::Missing,
            }
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return Presence::Missing;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return Presence::Missing;
        }
        match &comp.subs[0] {
            Atom::Text(text) => {
                if text.is_empty() {
                    Presence::Empty
                } else {
                    Presence::Value(text.clone())
                }
            }
            Atom::Null => Presence::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hl7v2_model::{Comp, Delims, Field, Rep};

    /// Helper to create a simple segment for testing
    fn create_test_segment(id: &str, fields: Vec<Field>) -> Segment {
        let id_bytes = id.as_bytes();
        let mut id_arr = [0u8; 3];
        id_arr.copy_from_slice(&id_bytes[..3]);
        Segment { id: id_arr, fields }
    }

    /// Helper to create a text field with repetitions
    fn create_text_field(texts: Vec<&str>) -> Field {
        Field {
            reps: texts
                .iter()
                .map(|t| Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text(t.to_string())],
                    }],
                })
                .collect(),
        }
    }

    /// Helper to create a field with components
    fn create_component_field(components: Vec<Vec<&str>>) -> Field {
        Field {
            reps: vec![Rep {
                comps: components
                    .iter()
                    .map(|subs| Comp {
                        subs: subs
                            .iter()
                            .map(|s| {
                                if *s == "\"\"" {
                                    Atom::Null
                                } else {
                                    Atom::Text(s.to_string())
                                }
                            })
                            .collect(),
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn test_parse_field_and_rep() {
        assert_eq!(parse_field_and_rep("5"), Some((5, 1)));
        assert_eq!(parse_field_and_rep("5[1]"), Some((5, 1)));
        assert_eq!(parse_field_and_rep("5[2]"), Some((5, 2)));
        assert_eq!(parse_field_and_rep("10[5]"), Some((10, 5)));
        assert_eq!(parse_field_and_rep("abc"), None);
        assert_eq!(parse_field_and_rep("5["), None);
        assert_eq!(parse_field_and_rep("5[abc]"), None);
    }

    #[test]
    fn test_get_simple_field() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "PID",
                vec![
                    create_text_field(vec!["1"]),     // PID.1
                    create_text_field(vec![""]),      // PID.2 (empty)
                    create_text_field(vec!["12345"]), // PID.3
                    create_text_field(vec![""]),      // PID.4
                    create_component_field(vec![
                        // PID.5
                        vec!["Doe"],  // Component 1 (family name)
                        vec!["John"], // Component 2 (given name)
                    ]),
                ],
            )],
            charsets: vec![],
        };

        // Get patient's last name (PID.5.1)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));

        // Get patient's first name (PID.5.2)
        assert_eq!(get(&message, "PID.5.2"), Some("John"));

        // Get PID.1
        assert_eq!(get(&message, "PID.1"), Some("1"));

        // Missing field
        assert_eq!(get(&message, "PID.10"), None);

        // Missing segment
        assert_eq!(get(&message, "EVN.1"), None);
    }

    #[test]
    fn test_get_with_repetitions() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "PID",
                vec![
                    create_text_field(vec!["1"]), // PID.1
                    create_text_field(vec![""]),  // PID.2
                    create_text_field(vec![""]),  // PID.3
                    create_text_field(vec![""]),  // PID.4
                    Field {
                        reps: vec![
                            Rep {
                                comps: vec![
                                    Comp {
                                        subs: vec![Atom::Text("Doe".to_string())],
                                    },
                                    Comp {
                                        subs: vec![Atom::Text("John".to_string())],
                                    },
                                ],
                            },
                            Rep {
                                comps: vec![
                                    Comp {
                                        subs: vec![Atom::Text("Smith".to_string())],
                                    },
                                    Comp {
                                        subs: vec![Atom::Text("Jane".to_string())],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            )],
            charsets: vec![],
        };

        // Test first repetition (default)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        assert_eq!(get(&message, "PID.5.2"), Some("John"));

        // Test second repetition
        assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
        assert_eq!(get(&message, "PID.5[2].2"), Some("Jane"));

        // Test invalid repetition
        assert_eq!(get(&message, "PID.5[3].1"), None);
    }

    #[test]
    fn test_get_msh_fields() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "MSH",
                vec![
                    // MSH.2 (encoding chars) - stored in fields[0]
                    create_text_field(vec!["^~\\&"]),
                    // MSH.3 (sending app) - stored in fields[1]
                    create_component_field(vec![vec!["SendingApp"]]),
                    // MSH.4 (sending fac) - stored in fields[2]
                    create_component_field(vec![vec!["SendingFac"]]),
                    // MSH.5 (receiving app) - stored in fields[3]
                    create_component_field(vec![vec!["ReceivingApp"]]),
                    // MSH.6 (receiving fac) - stored in fields[4]
                    create_component_field(vec![vec!["ReceivingFac"]]),
                    // MSH.7 (datetime) - stored in fields[5]
                    create_component_field(vec![vec!["20250128152312"]]),
                    // MSH.8 (security) - stored in fields[6]
                    create_component_field(vec![vec![""]]),
                    // MSH.9 (message type) - stored in fields[7]
                    create_component_field(vec![vec!["ADT"], vec!["A01"], vec!["ADT_A01"]]),
                    // MSH.10 (control ID) - stored in fields[8]
                    create_component_field(vec![vec!["ABC123"]]),
                    // MSH.11 (processing ID) - stored in fields[9]
                    create_component_field(vec![vec!["P"]]),
                    // MSH.12 (version) - stored in fields[10]
                    create_component_field(vec![vec!["2.5.1"]]),
                ],
            )],
            charsets: vec![],
        };

        // Get sending application (MSH.3)
        assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));

        // Get message type (MSH.9)
        assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
        assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
        assert_eq!(get(&message, "MSH.9.3"), Some("ADT_A01"));

        // Get control ID (MSH.10)
        assert_eq!(get(&message, "MSH.10"), Some("ABC123"));
    }

    #[test]
    fn test_presence_value() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "PID",
                vec![
                    create_text_field(vec!["1"]),
                    create_text_field(vec![""]),
                    create_text_field(vec!["12345"]),
                    create_text_field(vec![""]),
                    create_component_field(vec![vec!["Doe", "John"]]),
                ],
            )],
            charsets: vec![],
        };

        // Test existing field with value
        match get_presence(&message, "PID.5.1") {
            Presence::Value(val) => assert_eq!(val, "Doe"),
            _ => panic!("Expected Value"),
        }
    }

    #[test]
    fn test_presence_empty() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "PID",
                vec![
                    create_text_field(vec!["1"]),
                    create_text_field(vec![""]), // Empty field
                ],
            )],
            charsets: vec![],
        };

        // Test existing field with empty value
        match get_presence(&message, "PID.2") {
            Presence::Empty => {}
            _ => panic!("Expected Empty"),
        }
    }

    #[test]
    fn test_presence_missing() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "PID",
                vec![create_text_field(vec!["1"])],
            )],
            charsets: vec![],
        };

        // Test missing field
        match get_presence(&message, "PID.50.1") {
            Presence::Missing => {}
            _ => panic!("Expected Missing"),
        }

        // Test missing segment
        match get_presence(&message, "EVN.1") {
            Presence::Missing => {}
            _ => panic!("Expected Missing"),
        }
    }

    #[test]
    fn test_presence_null() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![create_test_segment(
                "PID",
                vec![Field {
                    reps: vec![Rep {
                        comps: vec![Comp {
                            subs: vec![Atom::Null],
                        }],
                    }],
                }],
            )],
            charsets: vec![],
        };

        // Test null field
        match get_presence(&message, "PID.1") {
            Presence::Null => {}
            _ => panic!("Expected Null"),
        }
    }

    #[test]
    fn test_presence_msh_field_1() {
        let message = Message {
            delims: Delims {
                field: '|',
                comp: '^',
                rep: '~',
                esc: '\\',
                sub: '&',
            },
            segments: vec![create_test_segment("MSH", vec![])],
            charsets: vec![],
        };

        // MSH-1 should return the field separator
        match get_presence(&message, "MSH.1") {
            Presence::Value(val) => assert_eq!(val, "|"),
            _ => panic!("Expected Value"),
        }
    }

    #[test]
    fn test_invalid_paths() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };

        // Empty path
        assert!(get(&message, "").is_none());

        // Segment only
        assert!(get(&message, "PID").is_none());

        // Invalid field index
        assert!(get(&message, "PID.abc").is_none());

        // Invalid component index
        // Note: This would need a proper message to test
    }
}
