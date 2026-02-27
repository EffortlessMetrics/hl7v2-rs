/// Delimiters used in HL7 v2 messages
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delims {
    pub field: char,
    pub comp: char,
    pub rep: char,
    pub esc: char,
    pub sub: char,
}

impl Delims {
    /// Create default delimiters (|^~\&)
    pub fn default() -> Self {
        Self {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        }
    }
}
