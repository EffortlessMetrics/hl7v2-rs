//! Core data model for HL7 v2 messages.
//!
//! This module provides the foundational data structures for HL7 v2 messages,
//! including:
//! - Message, Segment, Field, Repetition, Component, Atom types
//! - Delimiter configuration
//! - Error types
//! - Presence semantics
//!
//! This module has minimal dependencies and focuses solely on data representation.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::error_impl_error,
    clippy::indexing_slicing,
    clippy::missing_errors_doc,
    reason = "pre-existing model implementation debt moved from staged microcrate into hl7v2; cleanup is split from topology collapse"
)]

use serde::{Deserialize, Serialize};

/// Error type for HL7 v2 operations
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// Encountered an invalid segment ID during parsing.
    #[error("Invalid segment ID")]
    InvalidSegmentId,

    /// The delimiter sequence had an invalid length.
    #[error("Bad delimiter length")]
    BadDelimLength,

    /// Two or more delimiter characters are the same.
    #[error("Duplicate delimiters")]
    DuplicateDelims,

    /// Escape sequence markers were unbalanced.
    #[error("Unbalanced escape")]
    UnbalancedEscape,

    /// An escape token could not be parsed.
    #[error("Invalid escape token")]
    InvalidEscapeToken,

    /// The MSH segment format does not match the HL7 requirement.
    #[error("MSH field malformed")]
    MshFieldMalformed,

    /// The MSH-10 control ID field is missing.
    #[error("MSH-10 missing")]
    Msh10Missing,

    /// The message processing ID is invalid.
    #[error("Invalid processing ID")]
    InvalidProcessingId,

    /// The message version was not recognized.
    #[error("Unrecognized version")]
    UnrecognizedVersion,

    /// Message charset is unsupported or invalid.
    #[error("Invalid charset")]
    InvalidCharset,

    /// A framing error occurred while reading or writing the message.
    #[error("Framing error: {0}")]
    Framing(String),

    /// The write operation failed for the underlying transport or writer.
    #[error("Write failed")]
    WriteFailed,

    /// Parsing failed for the specified segment and field.
    #[error("Parse error at segment {segment_id} field {field_index}: {source}")]
    ParseError {
        /// Segment where the parse failed.
        segment_id: String,
        /// Index of the field in the segment.
        field_index: usize,
        #[source]
        /// Underlying error that caused the parse failure.
        source: Box<Error>,
    },

    /// Field text does not satisfy the configured field format.
    #[error("Invalid field format: {details}")]
    InvalidFieldFormat {
        /// Human-readable details for the parse error.
        details: String,
    },

    /// Repetition text does not satisfy the configured repetition format.
    #[error("Invalid repetition format: {details}")]
    InvalidRepFormat {
        /// Human-readable details for the parse error.
        details: String,
    },

    /// Component text does not satisfy the configured component format.
    #[error("Invalid component format: {details}")]
    InvalidCompFormat {
        /// Human-readable details for the parse error.
        details: String,
    },

    /// Subcomponent text does not satisfy the configured subcomponent format.
    #[error("Invalid subcomponent format: {details}")]
    InvalidSubcompFormat {
        /// Human-readable details for the parse error.
        details: String,
    },

    /// The batch parsing operation failed.
    #[error("Batch parsing error: {details}")]
    BatchParseError {
        /// Human-readable details for the batch parsing failure.
        details: String,
    },

    /// The batch header could not be read or interpreted.
    #[error("Invalid batch header: {details}")]
    InvalidBatchHeader {
        /// Human-readable details for the batch header failure.
        details: String,
    },

    /// The batch trailer could not be read or interpreted.
    #[error("Invalid batch trailer: {details}")]
    InvalidBatchTrailer {
        /// Human-readable details for the batch trailer failure.
        details: String,
    },
}

/// Delimiters used in HL7 v2 messages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delims {
    /// Field separator (`MSH-1` and list separators).
    pub field: char,
    /// Component separator (`MSH-2` first character).
    pub comp: char,
    /// Repetition separator (`MSH-2` second character).
    pub rep: char,
    /// Escape character (`MSH-2` third character).
    pub esc: char,
    /// Subcomponent separator (`MSH-2` fourth character).
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

        if let Some(next_char) = msh.chars().nth(8)
            && next_char != field_sep
        {
            return Err(Error::MshFieldMalformed);
        }

        let delimiters = [field_sep, comp_char, rep_char, esc_char, sub_char];
        if delimiters
            .iter()
            .any(|delimiter| !delimiter.is_ascii() || matches!(delimiter, '\r' | '\n'))
        {
            return Err(Error::BadDelimLength);
        }

        // Check that all delimiters are distinct
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
    /// Delimiters used for parsing and rendering message fields.
    pub delims: Delims,
    /// Segment list in message order.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Batch {
    /// Optional BHS header segment for the batch.
    pub header: Option<Segment>, // BHS segment
    /// HL7 messages contained in this batch.
    pub messages: Vec<Message>,
    /// Optional BTS trailer segment for the batch.
    pub trailer: Option<Segment>, // BTS segment
}

/// A file containing batches of HL7 messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FileBatch {
    /// Optional FHS header segment for the file batch.
    pub header: Option<Segment>, // FHS segment
    /// Batches nested under this file batch.
    pub batches: Vec<Batch>,
    /// Optional FTS trailer segment for the file batch.
    pub trailer: Option<Segment>, // FTS segment
}

/// A segment in an HL7 message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    /// Three-character segment ID (for example `MSH`, `PID`).
    pub id: [u8; 3],
    /// Fields contained in the segment.
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
    /// Repetitions contained in this field.
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
        self.reps
            .first()?
            .comps
            .first()?
            .subs
            .first()
            .and_then(|atom| match atom {
                Atom::Text(t) => Some(t.as_str()),
                Atom::Null => None,
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
    /// Components contained in this repetition.
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

    /// Get the first text value in this repetition
    pub fn first_text(&self) -> Option<&str> {
        self.comps.first()?.first_text()
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
    /// Subcomponents that make up this component.
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

    /// Get the first text value in this component
    pub fn first_text(&self) -> Option<&str> {
        self.subs.first()?.as_text()
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
    /// Plain text value.
    Text(String),
    /// Explicit NULL marker.
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
mod tests;
