/// Event enum for streaming parser
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Start of a new message with discovered delimiters
    StartMessage { delims: crate::Delims },
    /// A segment with its ID
    Segment { id: Vec<u8> },
    /// A field with its number (1-based) and raw content
    Field { num: u16, raw: Vec<u8> },
    /// End of message
    EndMessage,
}
