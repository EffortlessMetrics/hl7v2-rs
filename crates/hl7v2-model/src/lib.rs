//! Core data model for HL7 v2 messages.
//!
//! This crate provides the foundational data structures for HL7 v2 messages,
//! including:
//! - Message, Segment, Field, Repetition, Component, Atom types
//! - Delimiter configuration
//! - Error types
//! - Presence semantics
//!
//! This crate has minimal dependencies and focuses solely on data representation.

use serde::{Deserialize, Serialize};

/// Error type for HL7 v2 operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid segment ID")]
    InvalidSegmentId,
    
    #[error("Bad delimiter length")]
    BadDelimLength,
    
    #[error("Duplicate delimiters")]
    DuplicateDelims,
    
    #[error("Unbalanced escape")]
    UnbalancedEscape,
    
    #[error("Invalid escape token")]
    InvalidEscapeToken,
    
    #[error("MSH field malformed")]
    MshFieldMalformed,
    
    #[error("MSH-10 missing")]
    Msh10Missing,
    
    #[error("Invalid processing ID")]
    InvalidProcessingId,
    
    #[error("Unrecognized version")]
    UnrecognizedVersion,
    
    #[error("Invalid charset")]
    InvalidCharset,
    
    #[error("Write failed")]
    WriteFailed,
    
    #[error("Parse error at segment {segment_id} field {field_index}: {source}")]
    ParseError {
        segment_id: String,
        field_index: usize,
        #[source]
        source: Box<Error>,
    },
    
    #[error("Invalid field format: {details}")]
    InvalidFieldFormat {
        details: String,
    },
    
    #[error("Invalid repetition format: {details}")]
    InvalidRepFormat {
        details: String,
    },
    
    #[error("Invalid component format: {details}")]
    InvalidCompFormat {
        details: String,
    },
    
    #[error("Invalid subcomponent format: {details}")]
    InvalidSubcompFormat {
        details: String,
    },
    
    #[error("Batch parsing error: {details}")]
    BatchParseError {
        details: String,
    },
    
    #[error("Invalid batch header: {details}")]
    InvalidBatchHeader {
        details: String,
    },
    
    #[error("Invalid batch trailer: {details}")]
    InvalidBatchTrailer {
        details: String,
    },
}

/// Delimiters used in HL7 v2 messages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delims {
    pub field: char,
    pub comp: char,
    pub rep: char,
    pub esc: char,
    pub sub: char,
}

impl Default for Delims {
    fn default() -> Self {
        Self {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        }
    }
}

impl Delims {
    /// Create default delimiters (|^~\&)
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Parse delimiters from an MSH segment
    pub fn parse_from_msh(msh: &str) -> Result<Self, Error> {
        if msh.len() < 8 {
            return Err(Error::BadDelimLength);
        }
        
        let field_sep = msh.chars().nth(3).ok_or(Error::BadDelimLength)?;
        let comp_char = msh.chars().nth(4).ok_or(Error::BadDelimLength)?;
        let rep_char = msh.chars().nth(5).ok_or(Error::BadDelimLength)?;
        let esc_char = msh.chars().nth(6).ok_or(Error::BadDelimLength)?;
        let sub_char = msh.chars().nth(7).ok_or(Error::BadDelimLength)?;
        
        // Check that all delimiters are distinct
        let delimiters = [field_sep, comp_char, rep_char, esc_char, sub_char];
        for i in 0..delimiters.len() {
            for j in (i + 1)..delimiters.len() {
                if delimiters[i] == delimiters[j] {
                    return Err(Error::DuplicateDelims);
                }
            }
        }
        
        Ok(Self {
            field: field_sep,
            comp: comp_char,
            rep: rep_char,
            esc: esc_char,
            sub: sub_char,
        })
    }
}

/// Main message structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub delims: Delims,
    pub segments: Vec<Segment>,
    /// Character sets used in the message (from MSH-18)
    #[serde(default)]
    pub charsets: Vec<String>,
}

impl Message {
    /// Create a new empty message with default delimiters
    pub fn new() -> Self {
        Self {
            delims: Delims::default(),
            segments: Vec::new(),
            charsets: Vec::new(),
        }
    }
    
    /// Create a message with the given segments
    pub fn with_segments(segments: Vec<Segment>) -> Self {
        Self {
            delims: Delims::default(),
            segments,
            charsets: Vec::new(),
        }
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

/// A batch of HL7 messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Batch {
    pub header: Option<Segment>, // BHS segment
    pub messages: Vec<Message>,
    pub trailer: Option<Segment>, // BTS segment
}

impl Default for Batch {
    fn default() -> Self {
        Self {
            header: None,
            messages: Vec::new(),
            trailer: None,
        }
    }
}

/// A file containing batches of HL7 messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileBatch {
    pub header: Option<Segment>, // FHS segment
    pub batches: Vec<Batch>,
    pub trailer: Option<Segment>, // FTS segment
}

impl Default for FileBatch {
    fn default() -> Self {
        Self {
            header: None,
            batches: Vec::new(),
            trailer: None,
        }
    }
}

/// A segment in an HL7 message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub id: [u8; 3],
    pub fields: Vec<Field>,
}

impl Segment {
    /// Create a new segment with the given ID
    pub fn new(id: &[u8; 3]) -> Self {
        Self {
            id: *id,
            fields: Vec::new(),
        }
    }
    
    /// Get the segment ID as a string
    pub fn id_str(&self) -> &str {
        std::str::from_utf8(&self.id).unwrap_or("???")
    }
    
    /// Add a field to the segment
    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }
}

/// A field in a segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub reps: Vec<Rep>,
}

impl Field {
    /// Create a new empty field
    pub fn new() -> Self {
        Self { reps: Vec::new() }
    }
    
    /// Create a field with a single text value
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            reps: vec![Rep::from_text(text)],
        }
    }
    
    /// Add a repetition to the field
    pub fn add_rep(&mut self, rep: Rep) {
        self.reps.push(rep);
    }
    
    /// Get the first value as text (convenience method)
    pub fn first_text(&self) -> Option<&str> {
        self.reps.first()?.comps.first()?.subs.first().and_then(|atom| {
            match atom {
                Atom::Text(t) => Some(t.as_str()),
                Atom::Null => None,
            }
        })
    }
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

/// A repetition of a field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rep {
    pub comps: Vec<Comp>,
}

impl Rep {
    /// Create a new empty repetition
    pub fn new() -> Self {
        Self { comps: Vec::new() }
    }
    
    /// Create a repetition with a single text value
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            comps: vec![Comp::from_text(text)],
        }
    }
    
    /// Add a component to the repetition
    pub fn add_comp(&mut self, comp: Comp) {
        self.comps.push(comp);
    }
}

impl Default for Rep {
    fn default() -> Self {
        Self::new()
    }
}

/// A component of a field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comp {
    pub subs: Vec<Atom>,
}

impl Comp {
    /// Create a new empty component
    pub fn new() -> Self {
        Self { subs: Vec::new() }
    }
    
    /// Create a component with a single text value
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            subs: vec![Atom::Text(text.into())],
        }
    }
    
    /// Add a subcomponent to the component
    pub fn add_sub(&mut self, atom: Atom) {
        self.subs.push(atom);
    }
}

impl Default for Comp {
    fn default() -> Self {
        Self::new()
    }
}

/// An atomic value in the message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Atom {
    Text(String),
    Null,
}

impl Atom {
    /// Create a text atom
    pub fn text(s: impl Into<String>) -> Self {
        Atom::Text(s.into())
    }
    
    /// Create a null atom
    pub fn null() -> Self {
        Atom::Null
    }
    
    /// Check if this is a null atom
    pub fn is_null(&self) -> bool {
        matches!(self, Atom::Null)
    }
    
    /// Get the text value if this is a text atom
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Atom::Text(s) => Some(s.as_str()),
            Atom::Null => None,
        }
    }
}

/// Presence semantics for HL7 v2 fields
#[derive(Debug, Clone, PartialEq)]
pub enum Presence {
    /// Field is not present in the message (index out of range)
    Missing,
    /// Field is present but empty (zero-length)
    Empty,
    /// Field contains a literal NULL value ("")
    Null,
    /// Field contains a value
    Value(String),
}

impl Presence {
    /// Check if the field is missing
    pub fn is_missing(&self) -> bool {
        matches!(self, Presence::Missing)
    }
    
    /// Check if the field is present (may be empty or have a value)
    pub fn is_present(&self) -> bool {
        !self.is_missing()
    }
    
    /// Check if the field has an actual value
    pub fn has_value(&self) -> bool {
        matches!(self, Presence::Value(_))
    }
    
    /// Get the value if present
    pub fn value(&self) -> Option<&str> {
        match self {
            Presence::Value(v) => Some(v.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delims_default() {
        let delims = Delims::default();
        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_delims_parse_from_msh() {
        let msh = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1";
        let delims = Delims::parse_from_msh(msh).unwrap();
        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_delims_rejects_duplicates() {
        let msh = "MSH||||SendingApp";
        let result = Delims::parse_from_msh(msh);
        assert!(matches!(result, Err(Error::DuplicateDelims)));
    }

    #[test]
    fn test_message_creation() {
        let message = Message::new();
        assert!(message.segments.is_empty());
        assert!(message.charsets.is_empty());
    }

    #[test]
    fn test_segment_creation() {
        let segment = Segment::new(b"MSH");
        assert_eq!(segment.id, *b"MSH");
        assert!(segment.fields.is_empty());
    }

    #[test]
    fn test_field_creation() {
        let field = Field::from_text("test");
        assert_eq!(field.first_text(), Some("test"));
    }

    #[test]
    fn test_atom_creation() {
        let text = Atom::text("hello");
        assert_eq!(text.as_text(), Some("hello"));
        assert!(!text.is_null());
        
        let null = Atom::null();
        assert!(null.is_null());
        assert_eq!(null.as_text(), None);
    }

    #[test]
    fn test_presence_semantics() {
        let missing = Presence::Missing;
        assert!(missing.is_missing());
        assert!(!missing.is_present());
        
        let empty = Presence::Empty;
        assert!(!empty.is_missing());
        assert!(empty.is_present());
        assert!(!empty.has_value());
        
        let value = Presence::Value("test".to_string());
        assert!(value.has_value());
        assert_eq!(value.value(), Some("test"));
    }
}