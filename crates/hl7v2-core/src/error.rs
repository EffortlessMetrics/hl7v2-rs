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

    // New comprehensive error types
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
