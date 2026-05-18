//! HL7 v2 path-based field access and query functionality.
//!
//! This module provides query functionality for HL7 v2 messages,
//! including:
//! - Path-based field access via [`get`]
//! - Presence semantics via [`get_presence`]
//!
//! # Path Format
//!
//! Paths use the format: `SEGMENT.FIELD\[REP\].COMPONENT.SUBCOMPONENT`
//!
//! Examples:
//! - `PID.5.1` - First component of 5th field in PID segment (first repetition)
//! - `PID.5.1.2` - Second subcomponent of the first component of PID-5
//! - `PID.5[2].1` - First component of 5th field, second repetition
//! - `MSH.9` - 9th field of MSH segment
//! - `MSH.9.1` - First component of 9th field of MSH segment
//!
//! # Example
//!
//! ```
//! use hl7v2::Message;
//! use hl7v2::query::get;
//!
//! // Assuming you have a parsed Message from hl7v2::parser
//! // let message = hl7v2::parser::parse(hl7_bytes).unwrap();
//! // let last_name = get(&message, "PID.5.1").unwrap();
//! ```

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::manual_let_else,
    clippy::string_slice,
    reason = "pre-existing query implementation debt moved from staged microcrate into hl7v2; cleanup is split from topology collapse"
)]

pub mod path;

use crate::model::{Atom, Message, Presence, Segment};

/// Get value at path (e.g., `PID.5[1].1`)
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - The path to the field (e.g., `PID.5.1`, `PID.5[1].1`, `MSH.9`)
///
/// # Returns
///
/// The value at the path, or `None` if not found
///
/// # Example
///
/// ```
/// use hl7v2::query::get;
/// use hl7v2::{Message, Segment, Field, Rep, Comp, Atom, Delims};
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
    // Format: SEGMENT.FIELD\[REP\].COMPONENT
    // Examples: `PID.5.1`, `PID.5[1].1`, `MSH.9`

    let mut parts = path.split('.');
    let segment_id = parts.next()?;

    // Find the segment
    let segment = msg
        .segments
        .iter()
        .find(|s| std::str::from_utf8(&s.id) == Ok(segment_id))?;

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
/// use hl7v2::query::get_presence;
/// use hl7v2::{Message, Delims, Presence};
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
        .find(|s| std::str::from_utf8(&s.id) == Ok(segment_id))
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

/// Parse field and repetition indices from a string like `5` or `5[1]`.
fn parse_field_and_rep(field_str: &str) -> Option<(usize, usize)> {
    if let Some(bracket_pos) = field_str.find('[') {
        // Has repetition index
        let field_index = field_str[..bracket_pos].parse::<usize>().ok()?;
        let rep_part = &field_str[bracket_pos + 1..];
        let end_bracket = rep_part.find(']')?;
        if end_bracket + 1 != rep_part.len() {
            return None;
        }
        let rep_index = rep_part[..end_bracket].parse::<usize>().ok()?;
        Some((field_index, rep_index))
    } else {
        // No repetition index, default to 1
        let field_index = field_str.parse::<usize>().ok()?;
        Some((field_index, 1))
    }
}

fn parse_component_and_subcomponent(mut parts: std::str::Split<char>) -> Option<(usize, usize)> {
    let comp_index = if let Some(comp_part) = parts.next() {
        comp_part.parse::<usize>().ok()?
    } else {
        1
    };

    let sub_index = if let Some(sub_part) = parts.next() {
        sub_part.parse::<usize>().ok()?
    } else {
        1
    };

    if parts.next().is_some() {
        return None;
    }

    Some((comp_index, sub_index))
}

fn get_rep_value<'a>(
    rep: &'a crate::model::Rep,
    parts: std::str::Split<'_, char>,
) -> Option<&'a str> {
    let (comp_index, sub_index) = parse_component_and_subcomponent(parts)?;
    if comp_index == 0 || comp_index > rep.comps.len() {
        return None;
    }
    let comp = &rep.comps[comp_index - 1];
    if sub_index == 0 || sub_index > comp.subs.len() {
        return None;
    }

    match &comp.subs[sub_index - 1] {
        Atom::Text(text) => Some(text.as_str()),
        Atom::Null => None,
    }
}

fn get_rep_presence(rep: &crate::model::Rep, parts: std::str::Split<char>) -> Presence {
    let Some((comp_index, sub_index)) = parse_component_and_subcomponent(parts) else {
        return Presence::Missing;
    };

    if comp_index == 0 || comp_index > rep.comps.len() {
        return Presence::Missing;
    }
    let comp = &rep.comps[comp_index - 1];
    if sub_index == 0 || sub_index > comp.subs.len() {
        return Presence::Missing;
    }

    match &comp.subs[sub_index - 1] {
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

/// Get field value from a non-MSH segment
fn get_field<'a>(
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    parts: std::str::Split<char>,
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

    get_rep_value(rep, parts)
}

/// Get field value from an MSH segment
fn get_msh_field<'a>(
    _msg: &'a Message,
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    parts: std::str::Split<char>,
) -> Option<&'a str> {
    if field_index == 1 {
        // MSH-1 is the field separator character
        None // We can't return a reference to a temporary
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
        get_rep_value(rep, parts)
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
        get_rep_value(rep, parts)
    }
}

/// Get field presence from a non-MSH segment
fn get_field_presence(
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    parts: std::str::Split<char>,
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

    get_rep_presence(rep, parts)
}

/// Get field presence from an MSH segment
fn get_msh_field_presence(
    msg: &Message,
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    parts: std::str::Split<char>,
) -> Presence {
    if field_index == 1 {
        // MSH-1 is the field separator character
        Presence::Value(msg.delims.field.to_string())
    } else if field_index == 2 {
        if segment.fields.is_empty() {
            return Presence::Missing;
        }
        let field = &segment.fields[0];
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];
        get_rep_presence(rep, parts)
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
        get_rep_presence(rep, parts)
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;
    use crate::model::{Comp, Delims, Field, Rep};

    fn test_segment(id: &str, fields: Vec<Field>) -> Segment {
        let id_bytes = id.as_bytes();
        let mut id_arr = [0u8; 3];
        id_arr.copy_from_slice(&id_bytes[..3]);
        Segment { id: id_arr, fields }
    }

    fn component_field(subs: Vec<Atom>) -> Field {
        Field {
            reps: vec![Rep {
                comps: vec![Comp { subs }],
            }],
        }
    }

    #[test]
    fn get_supports_subcomponent_paths() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![test_segment(
                "PID",
                vec![component_field(vec![
                    Atom::Text("Family".to_string()),
                    Atom::Text("Own".to_string()),
                    Atom::Text("Prefix".to_string()),
                ])],
            )],
            charsets: vec![],
        };

        assert_eq!(get(&message, "PID.1.1"), Some("Family"));
        assert_eq!(get(&message, "PID.1.1.1"), Some("Family"));
        assert_eq!(get(&message, "PID.1.1.2"), Some("Own"));
        assert_eq!(get(&message, "PID.1.1.3"), Some("Prefix"));
        assert_eq!(get(&message, "PID.1.1.4"), None);
    }

    #[test]
    fn presence_supports_subcomponent_paths() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![test_segment(
                "PID",
                vec![component_field(vec![
                    Atom::Text("Family".to_string()),
                    Atom::Text(String::new()),
                    Atom::Null,
                ])],
            )],
            charsets: vec![],
        };

        match get_presence(&message, "PID.1.1.1") {
            Presence::Value(val) => assert_eq!(val, "Family"),
            _ => panic!("Expected Value"),
        }

        match get_presence(&message, "PID.1.1.2") {
            Presence::Empty => {}
            _ => panic!("Expected Empty"),
        }

        match get_presence(&message, "PID.1.1.3") {
            Presence::Null => {}
            _ => panic!("Expected Null"),
        }

        match get_presence(&message, "PID.1.1.4") {
            Presence::Missing => {}
            _ => panic!("Expected Missing"),
        }
    }

    #[test]
    fn rejects_trailing_path_components_and_malformed_repetitions() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![test_segment(
                "PID",
                vec![component_field(vec![
                    Atom::Text("Family".to_string()),
                    Atom::Text("Own".to_string()),
                ])],
            )],
            charsets: vec![],
        };

        assert_eq!(parse_field_and_rep("5[2]junk"), None);
        assert_eq!(parse_field_and_rep("5[2][3]"), None);
        assert_eq!(get(&message, "PID.1.1.1.1"), None);
        assert!(matches!(
            get_presence(&message, "PID.1.1.1.1"),
            Presence::Missing
        ));
    }
}

#[cfg(test)]
mod tests;
