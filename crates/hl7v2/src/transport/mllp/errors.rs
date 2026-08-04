/// MLLP-specific error types.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MllpError {
    /// Invalid MLLP frame structure - the frame does not conform to MLLP specification.
    #[error("Invalid MLLP frame structure: {details}")]
    InvalidFrame {
        /// Detailed description of what makes the frame invalid.
        details: String,
    },

    /// Missing start block (SB) character (0x0B).
    #[error("Missing MLLP start block character (0x0B)")]
    MissingStartBlock,

    /// Missing end block (EB) character sequence (0x1C 0x0D).
    #[error("Missing MLLP end block sequence (0x1C 0x0D)")]
    MissingEndBlock,

    /// IO error during MLLP operation.
    #[error("IO error: {0}")]
    IoError(String),

    /// Connection timeout.
    #[error("Connection timeout")]
    Timeout,

    /// The streaming frame buffer would exceed its configured limit.
    #[error("MLLP buffer limit exceeded: attempted {attempted_size} bytes, maximum {max_size}")]
    BufferLimitExceeded {
        /// Maximum number of bytes allowed in the streaming buffer.
        max_size: usize,
        /// Number of bytes that would have been buffered.
        attempted_size: usize,
    },
}

impl From<std::io::Error> for MllpError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}
