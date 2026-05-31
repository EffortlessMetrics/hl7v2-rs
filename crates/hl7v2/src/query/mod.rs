//! HL7 v2 path-based field access and query functionality.
//!
//! This module provides query functionality for HL7 v2 messages,
//! including:
//! - Path-based field access via [`get`]
//! - Presence semantics via [`get_presence`]
//!
//! # Path Format
//!
//! Paths use legacy dot notation, `SEGMENT.FIELD[REP].COMPONENT.SUBCOMPONENT`,
//! and diagnostic dash notation, `SEGMENT[REP]-FIELD[REP].COMPONENT.SUBCOMPONENT`.
//!
//! Examples:
//! - `PID.5.1` - First component of 5th field in PID segment (first repetition)
//! - `PID-5.1` - Same field using diagnostic dash notation
//! - `PID.5.1.2` - Second subcomponent of the first component of PID-5
//! - `PID.5[2].1` - First component of 5th field, second repetition
//! - `OBX[3]-5` - 5th field in the third OBX segment
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
    reason = "pre-existing query implementation debt moved from staged microcrate into hl7v2; cleanup is split from topology collapse"
)]

pub mod path;

use crate::model::{Atom, Message, Presence, Segment};

/// Get value at path (e.g., `PID.5[1].1` or `PID-5[1].1`)
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - The path to the field (e.g., `PID.5.1`, `PID-5.1`, `PID.5[1].1`, `OBX[3]-5`)
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
    let parsed = self::path::parse_located_path(path).ok()?;
    let segment = find_segment(msg, &parsed)?;
    let rep_index = parsed.path.repetition.unwrap_or(1);

    if parsed.path.segment == "MSH" {
        get_msh_field(
            msg,
            segment,
            parsed.path.field,
            rep_index,
            parsed.path.component,
            parsed.path.subcomponent,
        )
    } else {
        get_field(
            segment,
            parsed.path.field,
            rep_index,
            parsed.path.component,
            parsed.path.subcomponent,
        )
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
    let parsed = match self::path::parse_located_path(path) {
        Ok(path) => path,
        Err(_) => return Presence::Missing,
    };
    let segment = match find_segment(msg, &parsed) {
        Some(segment) => segment,
        None => return Presence::Missing,
    };
    let rep_index = parsed.path.repetition.unwrap_or(1);

    if parsed.path.segment == "MSH" {
        get_msh_field_presence(
            msg,
            segment,
            parsed.path.field,
            rep_index,
            parsed.path.component,
            parsed.path.subcomponent,
        )
    } else {
        get_field_presence(
            segment,
            parsed.path.field,
            rep_index,
            parsed.path.component,
            parsed.path.subcomponent,
        )
    }
}

// ============================================================================
// Internal helper functions
// ============================================================================

fn find_segment<'a>(msg: &'a Message, path: &self::path::LocatedPath) -> Option<&'a Segment> {
    let segment_repetition = path.segment_repetition.unwrap_or(1);
    if segment_repetition == 0 {
        return None;
    }

    msg.segments
        .iter()
        .filter(|segment| segment.id_str() == path.path.segment)
        .nth(segment_repetition - 1)
}

fn component_and_subcomponent(
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Option<(usize, usize)> {
    let comp_index = component.unwrap_or(1);
    let sub_index = subcomponent.unwrap_or(1);

    if comp_index == 0 || sub_index == 0 {
        None
    } else {
        Some((comp_index, sub_index))
    }
}

/// Get field value from a non-MSH segment
fn get_field(
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Option<&str> {
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

    let (comp_index, sub_index) = component_and_subcomponent(component, subcomponent)?;

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

/// Get field value from an MSH segment
fn get_msh_field<'a>(
    _msg: &'a Message,
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    component: Option<usize>,
    subcomponent: Option<usize>,
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
        let (comp_index, sub_index) = component_and_subcomponent(component, subcomponent)?;
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
        let (comp_index, sub_index) = component_and_subcomponent(component, subcomponent)?;
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
}

/// Get field presence from a non-MSH segment
fn get_field_presence(
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    component: Option<usize>,
    subcomponent: Option<usize>,
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

    let Some((comp_index, sub_index)) = component_and_subcomponent(component, subcomponent) else {
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

/// Get field presence from an MSH segment
fn get_msh_field_presence(
    msg: &Message,
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    component: Option<usize>,
    subcomponent: Option<usize>,
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
        let Some((comp_index, sub_index)) = component_and_subcomponent(component, subcomponent)
        else {
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
        let Some((comp_index, sub_index)) = component_and_subcomponent(component, subcomponent)
        else {
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
}

#[cfg(test)]
mod tests;
