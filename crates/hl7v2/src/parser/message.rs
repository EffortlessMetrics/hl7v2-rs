//! Top-level HL7 v2 message parsing entry points.

#![expect(
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    reason = "parser entry points preserve existing error behavior while parser responsibilities are split into SRP submodules"
)]

use crate::model::{Delims, Error, Message};

use super::charset::extract_charsets;
use super::segment::parse_segment;

/// Parse HL7 v2 message from bytes.
///
/// This is the primary entry point for parsing HL7 messages.
///
/// # Arguments
///
/// * `bytes` - The raw HL7 message bytes
///
/// # Returns
///
/// The parsed `Message`, or an error if parsing fails
///
/// # Example
///
/// ```
/// use hl7v2::parser::parse;
///
/// let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
/// let message = parse(hl7).unwrap();
/// assert_eq!(message.segments.len(), 2);
/// ```
pub fn parse(bytes: &[u8]) -> Result<Message, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();

    if lines.is_empty() {
        std::hint::cold_path();
        return Err(Error::InvalidSegmentId);
    }

    let Some(first_line) = lines.first() else {
        std::hint::cold_path();
        return Err(Error::InvalidSegmentId);
    };

    if !first_line.starts_with("MSH") {
        std::hint::cold_path();
        return Err(Error::InvalidSegmentId);
    }

    let delims = Delims::parse_from_msh(first_line).map_err(|e| Error::ParseError {
        segment_id: "MSH".to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;

    let mut segments = Vec::new();
    for line in lines {
        let segment = parse_segment(line, &delims).map_err(|e| Error::ParseError {
            segment_id: line.get(..3).unwrap_or(line).to_string(),
            field_index: 0,
            source: Box::new(e),
        })?;
        segments.push(segment);
    }

    let charsets = extract_charsets(&segments);

    Ok(Message {
        delims,
        segments,
        charsets,
    })
}

/// Parse HL7 v2 message from MLLP framed bytes.
///
/// This function first removes the MLLP framing and then parses the message.
///
/// # Arguments
///
/// * `bytes` - The MLLP-framed HL7 message bytes
///
/// # Returns
///
/// The parsed `Message`, or an error if parsing fails
///
/// # Example
///
/// ```
/// use hl7v2::parser::parse_mllp;
/// use hl7v2::wrap_mllp;
///
/// let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
/// let framed = wrap_mllp(hl7);
/// let message = parse_mllp(&framed).unwrap();
/// assert_eq!(message.segments.len(), 1);
/// ```
pub fn parse_mllp(bytes: &[u8]) -> Result<Message, Error> {
    let hl7_content =
        crate::transport::mllp::unwrap_mllp(bytes).map_err(|e| Error::Framing(e.to_string()))?;
    parse(hl7_content)
}
