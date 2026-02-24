//! HL7 v2 message normalization.
//!
//! This crate provides normalization for raw HL7 v2 bytes by parsing and
//! writing messages in a consistent format. Optionally, delimiters can be
//! rewritten to canonical HL7 delimiters (`|^~\&`).
//!
//! # Example
//!
//! ```
//! use hl7v2_normalize::normalize;
//!
//! let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
//! let normalized = normalize(hl7, true).unwrap();
//! assert!(normalized.starts_with(b"MSH|^~\\&|"));
//! ```

use hl7v2_model::{Delims, Error};
use hl7v2_parser::parse;
use hl7v2_writer::write;

/// Normalize HL7 v2 bytes.
///
/// The message is parsed and rewritten using `hl7v2_writer`. When
/// `canonical_delims` is `true`, the output message delimiters are rewritten
/// to canonical HL7 delimiters (`|^~\&`).
pub fn normalize(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    let mut message = parse(bytes)?;

    if canonical_delims {
        message.delims = Delims::default();
    }

    Ok(write(&message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_preserves_custom_delimiters_when_not_canonical() {
        let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\rPID*1**123456%%%HOSP%MR**Doe%John\r";

        let normalized = normalize(hl7, false).unwrap();

        assert!(normalized.starts_with(b"MSH*%$!?*"));
        assert!(normalized.contains(&b'*'));
    }

    #[test]
    fn normalize_converts_to_canonical_delimiters() {
        let hl7 = b"MSH*%$!?*SendingApp*SendingFac*ReceivingApp*ReceivingFac*20250128152312**ADT%A01*ABC123*P*2.5.1\rPID*1**123456%%%HOSP%MR**Doe%John\r";

        let normalized = normalize(hl7, true).unwrap();
        let normalized_str = String::from_utf8(normalized).unwrap();

        assert!(normalized_str.starts_with("MSH|^~\\&|"));
        assert!(normalized_str.contains("PID|1||123456^^^HOSP^MR||Doe^John\r"));
    }

    #[test]
    fn normalize_rejects_invalid_message() {
        let invalid = b"PID|1||12345\r";
        let err = normalize(invalid, true).unwrap_err();

        assert!(matches!(err, Error::InvalidSegmentId));
    }

    #[test]
    fn normalize_roundtrips_valid_message() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";

        let normalized = normalize(hl7, false).unwrap();
        let reparsed = hl7v2_parser::parse(&normalized).unwrap();

        assert_eq!(reparsed.segments.len(), 2);
        assert_eq!(&reparsed.segments[0].id, b"MSH");
        assert_eq!(&reparsed.segments[1].id, b"PID");
    }
}
