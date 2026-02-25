//! HL7 v2 field path parsing and resolution.
//!
//! This crate provides path-based access to HL7 v2 message fields,
//! supporting the standard path notation (e.g., "PID.5.1", "MSH.9[1].2").
//!
//! # Path Format
//!
//! - `SEGMENT.FIELD` - Access a field (e.g., "PID.5")
//! - `SEGMENT.FIELD.COMPONENT` - Access a component (e.g., "PID.5.1")
//! - `SEGMENT.FIELD[REP].COMPONENT` - Access with repetition (e.g., "PID.5[2].1")
//! - `SEGMENT.FIELD.COMPONENT.SUBCOMPONENT` - Access subcomponent
//!
//! # Example
//!
//! ```
//! use hl7v2_path::{Path, parse_path};
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
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid segment ID: {0}")]
    InvalidSegmentId(String),
    
    #[error("Invalid field number: {0}")]
    InvalidFieldNumber(String),
    
    #[error("Invalid component number: {0}")]
    InvalidComponentNumber(String),
    
    #[error("Invalid repetition index: {0}")]
    InvalidRepetitionIndex(String),
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
        let mut result = format!("{}.{}", self.segment, self.field);
        
        if let Some(rep) = self.repetition {
            result.push_str(&format!("[{}]", rep));
        }
        
        if let Some(comp) = self.component {
            result.push_str(&format!(".{}", comp));
        }
        
        if let Some(sub) = self.subcomponent {
            result.push_str(&format!(".{}", sub));
        }
        
        result
    }
    
    /// Check if this path points to an MSH segment
    pub fn is_msh(&self) -> bool {
        self.segment == "MSH"
    }
    
    /// Get the adjusted field index for MSH segments
    /// MSH-1 is the field separator (not stored)
    /// MSH-2 is the encoding characters (stored in field 0)
    /// MSH-3+ are stored starting at index 1
    pub fn msh_adjusted_field(&self) -> usize {
        if self.field <= 2 {
            self.field - 1 // MSH-1 -> 0, MSH-2 -> 1
        } else {
            self.field - 2 // MSH-3 -> 1, MSH-4 -> 2, etc.
        }
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_path_string())
    }
}

/// Parse an HL7 field path string
///
/// # Supported Formats
///
/// - `SEGMENT.FIELD` - e.g., "PID.5"
/// - `SEGMENT.FIELD.COMPONENT` - e.g., "PID.5.1"
/// - `SEGMENT.FIELD[REP]` - e.g., "PID.5[2]"
/// - `SEGMENT.FIELD[REP].COMPONENT` - e.g., "PID.5[2].1"
/// - `SEGMENT.FIELD.COMPONENT.SUBCOMPONENT` - e.g., "PID.5.1.1"
///
/// # Example
///
/// ```
/// use hl7v2_path::parse_path;
///
/// let path = parse_path("MSH.9.1").unwrap();
/// assert_eq!(path.segment, "MSH");
/// assert_eq!(path.field, 9);
/// assert_eq!(path.component, Some(1));
/// ```
pub fn parse_path(s: &str) -> Result<Path, PathError> {
    let s = s.trim();
    
    if s.is_empty() {
        return Err(PathError::InvalidFormat("Path cannot be empty".to_string()));
    }
    
    // Split by '.'
    let parts: Vec<&str> = s.split('.').collect();
    
    if parts.len() < 2 {
        return Err(PathError::InvalidFormat(
            format!("Path must have at least SEGMENT.FIELD, got: {}", s)
        ));
    }
    
    // Parse segment ID (must be 3 characters, uppercase letters/digits)
    let segment = parts[0].to_uppercase();
    if segment.len() != 3 || !segment.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(PathError::InvalidSegmentId(segment));
    }
    
    // Parse field number (may include repetition)
    let (field, repetition) = parse_field_part(parts[1])?;
    
    let mut path = Path::new(&segment, field);
    if let Some(rep) = repetition {
        path = path.with_repetition(rep);
    }
    
    // Parse optional component
    if parts.len() > 2 {
        let comp = parts[2].parse::<usize>()
            .map_err(|_| PathError::InvalidComponentNumber(parts[2].to_string()))?;
        
        if comp == 0 {
            return Err(PathError::InvalidComponentNumber("Component must be >= 1".to_string()));
        }
        
        path = path.with_component(comp);
    }
    
    // Parse optional subcomponent
    if parts.len() > 3 {
        let sub = parts[3].parse::<usize>()
            .map_err(|_| PathError::InvalidComponentNumber(parts[3].to_string()))?;
        
        if sub == 0 {
            return Err(PathError::InvalidComponentNumber("Subcomponent must be >= 1".to_string()));
        }
        
        path = path.with_subcomponent(sub);
    }
    
    Ok(path)
}

/// Parse a field part which may include repetition index
/// Returns (field_number, optional_repetition)
fn parse_field_part(s: &str) -> Result<(usize, Option<usize>), PathError> {
    if s.contains('[') {
        // Has repetition: "5[2]" or "5[1]"
        if !s.ends_with(']') {
            return Err(PathError::InvalidFormat(
                format!("Invalid field format, missing ']': {}", s)
            ));
        }
        
        let bracket_pos = s.find('[').unwrap();
        let field_str = &s[..bracket_pos];
        let rep_str = &s[bracket_pos + 1..s.len() - 1];
        
        let field = field_str.parse::<usize>()
            .map_err(|_| PathError::InvalidFieldNumber(field_str.to_string()))?;
        
        if field == 0 {
            return Err(PathError::InvalidFieldNumber("Field must be >= 1".to_string()));
        }
        
        let rep = rep_str.parse::<usize>()
            .map_err(|_| PathError::InvalidRepetitionIndex(rep_str.to_string()))?;
        
        if rep == 0 {
            return Err(PathError::InvalidRepetitionIndex("Repetition must be >= 1".to_string()));
        }
        
        Ok((field, Some(rep)))
    } else {
        // No repetition
        let field = s.parse::<usize>()
            .map_err(|_| PathError::InvalidFieldNumber(s.to_string()))?;
        
        if field == 0 {
            return Err(PathError::InvalidFieldNumber("Field must be >= 1".to_string()));
        }
        
        Ok((field, None))
    }
}

#[cfg(test)]
mod tests;