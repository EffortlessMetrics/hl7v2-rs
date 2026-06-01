//! HL7 v2 message normalization.
//!
//! This module provides normalization for raw HL7 v2 bytes by parsing and
//! writing messages in a consistent format. Optionally, delimiters can be
//! rewritten to canonical HL7 delimiters (`|^~\&`).
//!
//! # Example
//!
//! ```
//! use hl7v2::normalize;
//!
//! let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
//! let normalized = normalize(hl7, true).unwrap();
//! assert!(normalized.starts_with(b"MSH|^~\\&|"));
//! ```

use crate::model::{Delims, Error};
use crate::parser::parse;
use crate::writer::write;

/// Normalize HL7 v2 bytes.
///
/// The message is parsed for validation. When `canonical_delims` is `false`,
/// the original bytes are returned unchanged. When `canonical_delims` is `true`,
/// the output message delimiters are rewritten to canonical HL7 delimiters
/// (`|^~\&`) using `hl7v2::writer`.
///
/// # Errors
///
/// Returns an error when the input bytes cannot be parsed as an HL7 v2 message.
pub fn normalize(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    let mut message = parse(bytes)?;

    if !canonical_delims {
        return Ok(bytes.to_vec());
    }

    message.delims = Delims::default();

    Ok(write(&message))
}

#[cfg(test)]
mod tests;
