//! HL7 v2 field path parsing and resolution.
//!
//! This module provides path-based access to HL7 v2 message fields,
//! supporting legacy dot notation (e.g., `PID.5.1`) and diagnostic dash
//! notation used by operator output (e.g., `PID-5.1`, `OBX[2]-5`).
//!
//! # Path Format
//!
//! - `SEGMENT.FIELD` or `SEGMENT-FIELD` - Access a field (e.g., `PID.5`, `PID-5`)
//! - `SEGMENT.FIELD.COMPONENT` or `SEGMENT-FIELD.COMPONENT` - Access a component
//!   (e.g., `PID.5.1`, `PID-5.1`)
//! - `SEGMENT.FIELD[REP].COMPONENT` or `SEGMENT-FIELD[REP].COMPONENT` - Access
//!   with field repetition (e.g., `PID.5[2].1`, `PID-5[2].1`)
//! - `SEGMENT[REP].FIELD` or `SEGMENT[REP]-FIELD` - Access a repeated segment
//!   (e.g., `OBX[3].5`, `OBX[3]-5`)
//! - `SEGMENT.FIELD.COMPONENT.SUBCOMPONENT` - Access subcomponent
//!
//! # Example
//!
//! ```
//! use hl7v2::{Path, parse_path};
//!
//! let path = parse_path("PID.5[2].1").unwrap();
//! assert_eq!(path.segment, "PID");
//! assert_eq!(path.field, 5);
//! assert_eq!(path.repetition, Some(2));
//! assert_eq!(path.component, Some(1));
//! ```

use thiserror::Error;

/// Error type for path parsing
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PathError {
    /// Input does not match the expected path format.
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),

    /// Segment identifier is not valid for HL7 v2 paths.
    #[error("Invalid segment ID: {0}")]
    InvalidSegmentId(String),

    /// Field number is missing or outside the valid HL7 range.
    #[error("Invalid field number: {0}")]
    InvalidFieldNumber(String),

    /// Component number is missing or outside the valid HL7 range.
    #[error("Invalid component number: {0}")]
    InvalidComponentNumber(String),

    /// Repetition index is missing or outside the valid HL7 range.
    #[error("Invalid repetition index: {0}")]
    InvalidRepetitionIndex(String),
}

/// Represents a parsed HL7 field path plus a segment repetition selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedPath {
    /// Segment repetition index (1-based), None means first/default.
    pub segment_repetition: Option<usize>,
    /// Field, field repetition, component, and subcomponent selector.
    pub path: Path,
}

impl LocatedPath {
    /// Format as a path string, preserving any segment repetition selector.
    pub fn to_path_string(&self) -> String {
        let mut result = self.path.segment.clone();
        if let Some(rep) = self.segment_repetition {
            result.push('[');
            result.push_str(&rep.to_string());
            result.push(']');
        }
        result.push('.');
        result.push_str(&self.path.field.to_string());

        if let Some(rep) = self.path.repetition {
            result.push('[');
            result.push_str(&rep.to_string());
            result.push(']');
        }

        if let Some(comp) = self.path.component {
            result.push('.');
            result.push_str(&comp.to_string());
        }

        if let Some(sub) = self.path.subcomponent {
            result.push('.');
            result.push_str(&sub.to_string());
        }

        result
    }
}

/// Represents a parsed HL7 field path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// Segment ID (e.g., "PID", "MSH")
    pub segment: String,
    /// Field number (1-based)
    pub field: usize,
    /// Repetition index (1-based), None means first/default
    pub repetition: Option<usize>,
    /// Component number (1-based), None means whole field
    pub component: Option<usize>,
    /// Subcomponent number (1-based), None means whole component
    pub subcomponent: Option<usize>,
}

impl Path {
    /// Create a new path with the minimum required components
    pub fn new(segment: &str, field: usize) -> Self {
        Self {
            segment: segment.to_uppercase(),
            field,
            repetition: None,
            component: None,
            subcomponent: None,
        }
    }

    /// Set the repetition index
    pub fn with_repetition(mut self, rep: usize) -> Self {
        self.repetition = Some(rep);
        self
    }

    /// Set the component number
    pub fn with_component(mut self, comp: usize) -> Self {
        self.component = Some(comp);
        self
    }

    /// Set the subcomponent number
    pub fn with_subcomponent(mut self, sub: usize) -> Self {
        self.subcomponent = Some(sub);
        self
    }

    /// Format as a path string
    pub fn to_path_string(&self) -> String {
        let mut result = self.segment.clone();
        result.push('.');
        result.push_str(&self.field.to_string());

        if let Some(rep) = self.repetition {
            result.push('[');
            result.push_str(&rep.to_string());
            result.push(']');
        }

        if let Some(comp) = self.component {
            result.push('.');
            result.push_str(&comp.to_string());
        }

        if let Some(sub) = self.subcomponent {
            result.push('.');
            result.push_str(&sub.to_string());
        }

        result
    }

    /// Check if this path points to an MSH segment
    pub fn is_msh(&self) -> bool {
        self.segment == "MSH"
    }

    /// Get the stored field index for MSH segments.
    ///
    /// `MSH-1` is the field separator delimiter and is not stored as a field.
    /// `MSH-2` is the encoding characters at `Segment::fields[0]`.
    /// `MSH-3` and later fields follow from `Segment::fields[1]`.
    pub fn msh_stored_field_index(&self) -> Option<usize> {
        match self.field {
            0 | 1 => None,
            2 => Some(0),
            field => Some(field.saturating_sub(2)),
        }
    }

    /// Get the adjusted field index for MSH segments.
    ///
    /// Prefer [`Path::msh_stored_field_index`] when callers need to distinguish
    /// delimiter metadata (`MSH-1`) from stored fields. This legacy helper returns
    /// `0` for `MSH-1` because there is no stored field index for the delimiter.
    pub fn msh_adjusted_field(&self) -> usize {
        self.msh_stored_field_index().unwrap_or(0)
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_path_string())
    }
}

/// Parse an HL7 field path string
///
/// # Supported Formats
///
/// - `SEGMENT.FIELD` - e.g., `PID.5`
/// - `SEGMENT-FIELD` - e.g., `PID-5`
/// - `SEGMENT.FIELD.COMPONENT` - e.g., `PID.5.1`
/// - `SEGMENT-FIELD.COMPONENT` - e.g., `PID-5.1`
/// - `SEGMENT.FIELD[REP]` - e.g., `PID.5[2]`
/// - `SEGMENT-FIELD[REP]` - e.g., `PID-5[2]`
/// - `SEGMENT[REP].FIELD` - e.g., `OBX[3].5`
/// - `SEGMENT[REP]-FIELD` - e.g., `OBX[3]-5`
/// - `SEGMENT.FIELD.COMPONENT.SUBCOMPONENT` - e.g., `PID.5.1.1`
///
/// # Errors
///
/// Returns [`PathError`] when the input is empty, missing the field component,
/// has an invalid segment identifier, or contains zero/non-numeric field,
/// repetition, component, or subcomponent indexes.
///
/// # Example
///
/// ```
/// use hl7v2::parse_path;
///
/// let path = parse_path("MSH.9.1").unwrap();
/// assert_eq!(path.segment, "MSH");
/// assert_eq!(path.field, 9);
/// assert_eq!(path.component, Some(1));
/// ```
pub fn parse_path(s: &str) -> Result<Path, PathError> {
    let located = parse_located_path(s)?;
    if located.segment_repetition.is_some() {
        return Err(PathError::InvalidFormat(
            "Segment repetition requires parse_located_path".to_string(),
        ));
    }

    Ok(located.path)
}

/// Parse an HL7 path that may include a segment repetition selector.
///
/// This accepts the same field forms as [`parse_path`] plus segment repetition
/// selectors like `OBX[3]-5` or `NTE[1].3`.
///
/// # Errors
///
/// Returns [`PathError`] when the input is empty, missing the field component,
/// has an invalid segment identifier, or contains zero/non-numeric segment,
/// field, repetition, component, or subcomponent indexes.
pub fn parse_located_path(s: &str) -> Result<LocatedPath, PathError> {
    let s = s.trim();

    if s.is_empty() {
        return Err(PathError::InvalidFormat("Path cannot be empty".to_string()));
    }

    let (segment_part, field_and_component_part) = split_segment_and_field(s)?;
    let mut parts = field_and_component_part.split('.');
    let field_part = parts.next().unwrap_or_default();
    let component_part = parts.next();
    let subcomponent_part = parts.next();
    if parts.next().is_some() {
        return Err(PathError::InvalidFormat(format!(
            "Path has too many components, got: {s}"
        )));
    }

    let (segment, segment_repetition) = parse_segment_part(segment_part)?;

    // Parse field number (may include repetition)
    let (field, repetition) = parse_field_part(field_part)?;

    let mut path = Path::new(&segment, field);
    if let Some(rep) = repetition {
        path = path.with_repetition(rep);
    }

    // Parse optional component
    if let Some(component_part) = component_part {
        let comp = component_part
            .parse::<usize>()
            .map_err(|_parse_err| PathError::InvalidComponentNumber(component_part.to_string()))?;

        if comp == 0 {
            return Err(PathError::InvalidComponentNumber(
                "Component must be >= 1".to_string(),
            ));
        }

        path = path.with_component(comp);
    }

    // Parse optional subcomponent
    if let Some(subcomponent_part) = subcomponent_part {
        let sub = subcomponent_part.parse::<usize>().map_err(|_parse_err| {
            PathError::InvalidComponentNumber(subcomponent_part.to_string())
        })?;

        if sub == 0 {
            return Err(PathError::InvalidComponentNumber(
                "Subcomponent must be >= 1".to_string(),
            ));
        }

        path = path.with_subcomponent(sub);
    }

    Ok(LocatedPath {
        segment_repetition,
        path,
    })
}

fn split_segment_and_field(s: &str) -> Result<(&str, &str), PathError> {
    if let Some((segment_part, field_part)) = s.split_once('-') {
        if segment_part.is_empty() || field_part.is_empty() {
            return Err(PathError::InvalidFormat(format!(
                "Path must have SEGMENT-FIELD, got: {s}"
            )));
        }
        return Ok((segment_part, field_part));
    }

    s.split_once('.').ok_or_else(|| {
        PathError::InvalidFormat(format!("Path must have at least SEGMENT.FIELD, got: {s}"))
    })
}

fn parse_segment_part(s: &str) -> Result<(String, Option<usize>), PathError> {
    let (segment, repetition) = if s.contains('[') {
        let stripped = s.strip_suffix(']').ok_or_else(|| {
            PathError::InvalidFormat(format!("Invalid segment format, missing ']': {s}"))
        })?;
        let Some((segment_str, rep_str)) = stripped.split_once('[') else {
            return Err(PathError::InvalidFormat(format!(
                "Invalid segment format, missing '[': {s}"
            )));
        };
        let rep = rep_str
            .parse::<usize>()
            .map_err(|_parse_err| PathError::InvalidRepetitionIndex(rep_str.to_string()))?;
        if rep == 0 {
            return Err(PathError::InvalidRepetitionIndex(
                "Segment repetition must be >= 1".to_string(),
            ));
        }
        (segment_str.to_uppercase(), Some(rep))
    } else {
        (s.to_uppercase(), None)
    };

    // Parse segment ID (must be 3 characters, start with letter, rest alphanumeric)
    if segment.len() != 3
        || !segment.starts_with(|c: char| c.is_ascii_alphabetic())
        || !segment.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(PathError::InvalidSegmentId(segment));
    }

    Ok((segment, repetition))
}

/// Parse a field part which may include repetition index
/// Returns (field_number, optional_repetition)
fn parse_field_part(s: &str) -> Result<(usize, Option<usize>), PathError> {
    if s.contains('[') {
        // Has repetition: "5[2]" or "5[1]"
        let stripped = s.strip_suffix(']').ok_or_else(|| {
            PathError::InvalidFormat(format!("Invalid field format, missing ']': {s}"))
        })?;
        let Some((field_str, rep_str)) = stripped.split_once('[') else {
            return Err(PathError::InvalidFormat(format!(
                "Invalid field format, missing '[': {s}"
            )));
        };

        let field = field_str
            .parse::<usize>()
            .map_err(|_parse_err| PathError::InvalidFieldNumber(field_str.to_string()))?;

        if field == 0 {
            return Err(PathError::InvalidFieldNumber(
                "Field must be >= 1".to_string(),
            ));
        }

        let rep = rep_str
            .parse::<usize>()
            .map_err(|_parse_err| PathError::InvalidRepetitionIndex(rep_str.to_string()))?;

        if rep == 0 {
            return Err(PathError::InvalidRepetitionIndex(
                "Repetition must be >= 1".to_string(),
            ));
        }

        Ok((field, Some(rep)))
    } else {
        // No repetition
        let field = s
            .parse::<usize>()
            .map_err(|_parse_err| PathError::InvalidFieldNumber(s.to_string()))?;

        if field == 0 {
            return Err(PathError::InvalidFieldNumber(
                "Field must be >= 1".to_string(),
            ));
        }

        Ok((field, None))
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::panic,
        reason = "path parser tests use explicit failure messages for unexpected parse errors"
    )]

    use super::*;

    #[test]
    fn parse_path_accepts_dash_field_separator() {
        let path = match parse_path("MSH-9.1") {
            Ok(path) => path,
            Err(err) => panic!("dash path should parse: {err}"),
        };

        assert_eq!(path.segment, "MSH");
        assert_eq!(path.field, 9);
        assert_eq!(path.repetition, None);
        assert_eq!(path.component, Some(1));
        assert_eq!(path.to_path_string(), "MSH.9.1");
    }

    #[test]
    fn msh_stored_field_index_matches_segment_storage() {
        assert_eq!(Path::new("MSH", 1).msh_stored_field_index(), None);
        assert_eq!(Path::new("MSH", 2).msh_stored_field_index(), Some(0));
        assert_eq!(Path::new("MSH", 3).msh_stored_field_index(), Some(1));
        assert_eq!(Path::new("MSH", 9).msh_stored_field_index(), Some(7));
    }

    #[test]
    fn msh_adjusted_field_keeps_legacy_delimiter_sentinel() {
        assert_eq!(Path::new("MSH", 1).msh_adjusted_field(), 0);
        assert_eq!(Path::new("MSH", 2).msh_adjusted_field(), 0);
        assert_eq!(Path::new("MSH", 3).msh_adjusted_field(), 1);
        assert_eq!(Path::new("MSH", 9).msh_adjusted_field(), 7);
    }

    #[test]
    fn parse_path_accepts_dash_field_repetition() {
        let path = match parse_path("PID-3[2].4") {
            Ok(path) => path,
            Err(err) => panic!("dash path should parse: {err}"),
        };

        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 3);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, Some(4));
    }

    #[test]
    fn parse_path_accepts_segment_repetition() {
        let path = match parse_located_path("OBX[3]-5") {
            Ok(path) => path,
            Err(err) => panic!("segment repetition should parse: {err}"),
        };

        assert_eq!(path.segment_repetition, Some(3));
        assert_eq!(path.path.segment, "OBX");
        assert_eq!(path.path.field, 5);
        assert_eq!(path.to_path_string(), "OBX[3].5");
    }

    #[test]
    fn parse_path_accepts_dot_segment_repetition_for_compatibility() {
        let path = match parse_located_path("NTE[1].3") {
            Ok(path) => path,
            Err(err) => panic!("dot segment repetition should parse: {err}"),
        };

        assert_eq!(path.segment_repetition, Some(1));
        assert_eq!(path.path.segment, "NTE");
        assert_eq!(path.path.field, 3);
    }

    #[test]
    fn parse_path_rejects_segment_repetition_without_dropping_it() {
        assert!(matches!(
            parse_path("OBX[3]-5"),
            Err(PathError::InvalidFormat(_))
        ));
    }

    #[test]
    fn parse_path_rejects_zero_segment_repetition() {
        assert!(matches!(
            parse_located_path("OBX[0]-5"),
            Err(PathError::InvalidRepetitionIndex(_))
        ));
    }

    #[test]
    fn parse_path_rejects_extra_components() {
        assert!(matches!(
            parse_path("PID.5.1.2.3"),
            Err(PathError::InvalidFormat(_))
        ));
    }

    #[test]
    fn parse_path_reports_invalid_subcomponent_number_as_component_error() {
        assert!(matches!(
            parse_path("PID.5.1.abc"),
            Err(PathError::InvalidComponentNumber(_))
        ));
        assert!(matches!(
            parse_path("PID.5.1.0"),
            Err(PathError::InvalidComponentNumber(_))
        ));
    }
}
