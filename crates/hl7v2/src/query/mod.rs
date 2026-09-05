//! HL7 v2 path-based field access and query functionality.
//!
//! This module provides query functionality for HL7 v2 messages,
//! including:
//! - Path-based field access via [`get`]
//! - Pre-parsed path access via [`get_located`]
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

use std::collections::HashMap;

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
    get_located(msg, &parsed)
}

/// Get a value with a pre-parsed path.
///
/// This is the parse-once variant of [`get`]. Use it when the same path is
/// applied repeatedly across messages or across multiple lookups of the same
/// message.
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - A path returned by [`path::parse_located_path`]
///
/// # Returns
///
/// The value at the path, or `None` if not found.
pub fn get_located<'a>(msg: &'a Message, path: &self::path::LocatedPath) -> Option<&'a str> {
    let segment = find_segment(msg, path)?;
    get_located_in_segment(msg, segment, path)
}

/// Immutable segment index for repeated queries against one message.
///
/// `QueryIndex` is useful when callers need to inspect many paths from the same
/// parsed message. It builds a small segment-position map once and then reuses
/// it for segment repetition lookups. The index borrows the message immutably,
/// so safe Rust code cannot mutate the segment list while indexed lookups are
/// active.
///
/// # Example
///
/// ```
/// use hl7v2::{QueryIndex, parse};
///
/// let message = parse(
///     b"MSH|^~\\&|LAB|L|EHR|E|202606010101||ORU^R01|CTRL1|P|2.5\r\
///       OBX|1|ST|A||first\r\
///       OBX|2|ST|B||second",
/// )
/// .unwrap();
/// let index = QueryIndex::new(&message);
///
/// assert_eq!(index.get("OBX[2]-5"), Some("second"));
/// ```
#[derive(Debug, Clone)]
pub struct QueryIndex<'a> {
    msg: &'a Message,
    segment_offsets: HashMap<[u8; 3], Vec<usize>>,
}

impl<'a> QueryIndex<'a> {
    /// Build an index for repeated lookups against `msg`.
    #[must_use]
    pub fn new(msg: &'a Message) -> Self {
        let mut segment_offsets: HashMap<[u8; 3], Vec<usize>> = HashMap::new();
        for (index, segment) in msg.segments.iter().enumerate() {
            segment_offsets.entry(segment.id).or_default().push(index);
        }

        Self {
            msg,
            segment_offsets,
        }
    }

    /// Return the message backing this index.
    #[must_use]
    pub fn message(&self) -> &'a Message {
        self.msg
    }

    /// Get value at path using this index.
    ///
    /// This accepts the same path syntax as [`get`].
    pub fn get(&self, path: &str) -> Option<&'a str> {
        let parsed = self::path::parse_located_path(path).ok()?;
        self.get_located(&parsed)
    }

    /// Get value at a pre-parsed path using this index.
    ///
    /// This is the indexed counterpart to [`get_located`].
    pub fn get_located(&self, path: &self::path::LocatedPath) -> Option<&'a str> {
        let segment = find_indexed_segment(self.msg, &self.segment_offsets, path)?;
        get_located_in_segment(self.msg, segment, path)
    }

    /// Get presence semantics using this index.
    ///
    /// This accepts the same path syntax as [`get_presence`].
    #[must_use]
    pub fn get_presence(&self, path: &str) -> Presence {
        let parsed = match self::path::parse_located_path(path) {
            Ok(path) => path,
            Err(_) => return Presence::Missing,
        };
        self.get_presence_located(&parsed)
    }

    /// Get presence semantics for a pre-parsed path using this index.
    ///
    /// This is the indexed counterpart to [`get_presence_located`].
    #[must_use]
    pub fn get_presence_located(&self, path: &self::path::LocatedPath) -> Presence {
        let segment = match find_indexed_segment(self.msg, &self.segment_offsets, path) {
            Some(segment) => segment,
            None => return Presence::Missing,
        };
        get_presence_located_in_segment(self.msg, segment, path)
    }
}

fn get_located_in_segment<'a>(
    msg: &'a Message,
    segment: &'a Segment,
    path: &self::path::LocatedPath,
) -> Option<&'a str> {
    let rep_index = path.path.repetition.unwrap_or(1);

    if path.path.segment == "MSH" {
        get_msh_field(
            msg,
            segment,
            path.path.field,
            rep_index,
            path.path.component,
            path.path.subcomponent,
        )
    } else {
        get_field(
            segment,
            path.path.field,
            rep_index,
            path.path.component,
            path.path.subcomponent,
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
    get_presence_located(msg, &parsed)
}

/// Get presence semantics with a pre-parsed path.
///
/// This is the parse-once variant of [`get_presence`]. It preserves the same
/// missing/empty/null/value behavior as the string-path API.
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - A path returned by [`path::parse_located_path`]
///
/// # Returns
///
/// The presence status of the field.
pub fn get_presence_located(msg: &Message, path: &self::path::LocatedPath) -> Presence {
    let segment = match find_segment(msg, path) {
        Some(segment) => segment,
        None => return Presence::Missing,
    };
    get_presence_located_in_segment(msg, segment, path)
}

fn get_presence_located_in_segment(
    msg: &Message,
    segment: &Segment,
    path: &self::path::LocatedPath,
) -> Presence {
    let rep_index = path.path.repetition.unwrap_or(1);

    if path.path.segment == "MSH" {
        get_msh_field_presence(
            msg,
            segment,
            path.path.field,
            rep_index,
            path.path.component,
            path.path.subcomponent,
        )
    } else {
        get_field_presence(
            segment,
            path.path.field,
            rep_index,
            path.path.component,
            path.path.subcomponent,
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

fn find_indexed_segment<'a>(
    msg: &'a Message,
    segment_offsets: &HashMap<[u8; 3], Vec<usize>>,
    path: &self::path::LocatedPath,
) -> Option<&'a Segment> {
    let segment_repetition = path.segment_repetition.unwrap_or(1);
    if segment_repetition == 0 {
        return None;
    }

    let key = segment_key(&path.path.segment)?;
    let segment_index = segment_offsets
        .get(&key)?
        .get(segment_repetition - 1)
        .copied()?;
    msg.segments.get(segment_index)
}

fn segment_key(segment: &str) -> Option<[u8; 3]> {
    segment.as_bytes().try_into().ok()
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

fn ascii_delimiter_value(delimiter: char) -> Option<&'static str> {
    static ASCII_STRINGS: std::sync::OnceLock<[String; 128]> = std::sync::OnceLock::new();

    if !delimiter.is_ascii() {
        return None;
    }

    let values = ASCII_STRINGS.get_or_init(|| {
        std::array::from_fn(|index| match u8::try_from(index) {
            Ok(byte) => char::from(byte).to_string(),
            Err(_err) => String::new(),
        })
    });
    Some(values[delimiter as usize].as_str())
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
    msg: &'a Message,
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    component: Option<usize>,
    subcomponent: Option<usize>,
) -> Option<&'a str> {
    if field_index == 0 {
        return None;
    }

    if field_index == 1 {
        // MSH-1 is the field separator character
        ascii_delimiter_value(msg.delims.field)
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
    if field_index == 0 {
        return Presence::Missing;
    }

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
