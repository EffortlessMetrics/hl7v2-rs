use crate::{parse, parse_batch, parse_file_batch, write, write_batch, write_file_batch, Delims, Error};

/// Normalize HL7 v2 message
///
/// This function parses and rewrites an HL7 message, optionally converting
/// it to canonical delimiters (|^~\&).
pub fn normalize(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the message
    let mut message = parse(bytes)?;

    // If canonical delimiters are requested, update the message delimiters
    if canonical_delims {
        message.delims = Delims::default();
    }

    // Write the normalized message
    Ok(write(&message))
}

/// Normalize HL7 v2 batch
///
/// This function parses and rewrites an HL7 batch message, optionally converting
/// it to canonical delimiters (|^~\&).
pub fn normalize_batch(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the batch
    let mut batch = parse_batch(bytes)?;

    // If canonical delimiters are requested, update all message delimiters
    if canonical_delims {
        let canonical = Delims::default();
        for message in &mut batch.messages {
            message.delims = canonical.clone();
        }
    }

    // Write the normalized batch
    Ok(write_batch(&batch))
}

/// Normalize HL7 v2 file batch
///
/// This function parses and rewrites an HL7 file batch message, optionally converting
/// it to canonical delimiters (|^~\&).
pub fn normalize_file_batch(bytes: &[u8], canonical_delims: bool) -> Result<Vec<u8>, Error> {
    // Parse the file batch
    let mut file_batch = parse_file_batch(bytes)?;

    // If canonical delimiters are requested, update all message delimiters
    if canonical_delims {
        let canonical = Delims::default();
        for batch in &mut file_batch.batches {
            for message in &mut batch.messages {
                message.delims = canonical.clone();
            }
        }
    }

    // Write the normalized file batch
    Ok(write_file_batch(&file_batch))
}
