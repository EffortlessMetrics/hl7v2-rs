use crate::Delims;

/// Main message structure
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub delims: Delims,
    pub segments: Vec<Segment>,
    /// Character sets used in the message (from MSH-18)
    pub charsets: Vec<String>,
}

/// A batch of HL7 messages
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub header: Option<Segment>, // BHS segment
    pub messages: Vec<Message>,
    pub trailer: Option<Segment>, // BTS segment
}

/// A file containing batches of HL7 messages
#[derive(Debug, Clone, PartialEq)]
pub struct FileBatch {
    pub header: Option<Segment>, // FHS segment
    pub batches: Vec<Batch>,
    pub trailer: Option<Segment>, // FTS segment
}

/// A segment in an HL7 message
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub id: [u8; 3],
    pub fields: Vec<Field>,
}

/// A field in a segment
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub reps: Vec<Rep>,
}

/// A repetition of a field
#[derive(Debug, Clone, PartialEq)]
pub struct Rep {
    pub comps: Vec<Comp>,
}

/// A component of a field
#[derive(Debug, Clone, PartialEq)]
pub struct Comp {
    pub subs: Vec<Atom>,
}

/// An atomic value in the message
#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Text(String),
    Null,
}

/// Presence semantics for HL7 v2 fields
#[derive(Debug, Clone, PartialEq)]
pub enum Presence {
    /// Field is not present in the message (index out of range)
    Missing,
    /// Field is present but empty (zero-length)
    Empty,
    /// Field contains a literal NULL value (""")
    Null,
    /// Field contains a value
    Value(String),
}
