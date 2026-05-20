//! Batch and file-batch parsing for BHS/BTS and FHS/FTS envelopes.

#![expect(
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::uninlined_format_args,
    reason = "batch parsing preserves existing envelope behavior while parser responsibilities are split into SRP submodules"
)]

use crate::model::{Batch, Delims, Error, FileBatch};

use super::message::{SegmentLine, parse, segment_line_spans};
use super::segment::parse_segment;

/// Parse HL7 v2 batch from bytes.
///
/// # Arguments
///
/// * `bytes` - The raw HL7 batch bytes
///
/// # Returns
///
/// The parsed `Batch`, or an error if parsing fails
pub fn parse_batch(bytes: &[u8]) -> Result<Batch, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    let lines = segment_line_spans(text);

    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }

    let Some(first_line) = lines.first() else {
        return Err(Error::InvalidSegmentId);
    };
    if first_line.text.starts_with("BHS") {
        parse_batch_with_header(text, &lines)
    } else if first_line.text.starts_with("MSH") {
        let message = parse(bytes)?;
        Ok(Batch {
            header: None,
            messages: vec![message],
            trailer: None,
        })
    } else {
        Err(Error::InvalidSegmentId)
    }
}

/// Parse HL7 v2 file batch from bytes.
///
/// # Arguments
///
/// * `bytes` - The raw HL7 file batch bytes
///
/// # Returns
///
/// The parsed `FileBatch`, or an error if parsing fails
pub fn parse_file_batch(bytes: &[u8]) -> Result<FileBatch, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    let lines = segment_line_spans(text);

    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }

    let Some(first_line) = lines.first() else {
        return Err(Error::InvalidSegmentId);
    };
    if first_line.text.starts_with("FHS") {
        parse_file_batch_with_header(text, &lines)
    } else if first_line.text.starts_with("BHS") || first_line.text.starts_with("MSH") {
        let batch_data = parse_batch(bytes)?;
        Ok(FileBatch {
            header: None,
            batches: vec![batch_data],
            trailer: None,
        })
    } else {
        Err(Error::InvalidSegmentId)
    }
}

fn parse_batch_with_header(source: &str, lines: &[SegmentLine<'_>]) -> Result<Batch, Error> {
    if !lines
        .first()
        .is_some_and(|line| line.text.starts_with("BHS"))
    {
        return Err(Error::InvalidBatchHeader {
            details: "Batch must start with BHS segment".to_string(),
        });
    }

    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {}", e),
    })?;

    let mut header = None;
    let mut messages = Vec::new();
    let mut trailer = None;
    let mut current_message_lines = Vec::new();

    for line in lines {
        if line.text.starts_with("BHS") {
            let bhs_segment =
                parse_segment(line.text, &delims).map_err(|e| Error::InvalidBatchHeader {
                    details: format!("Failed to parse BHS segment: {}", e),
                })?;
            header = Some(bhs_segment);
        } else if line.text.starts_with("BTS") {
            let bts_segment =
                parse_segment(line.text, &delims).map_err(|e| Error::InvalidBatchTrailer {
                    details: format!("Failed to parse BTS segment: {}", e),
                })?;
            trailer = Some(bts_segment);
        } else if line.text.starts_with("MSH") {
            push_pending_message(source, &mut messages, &mut current_message_lines, false)?;
            current_message_lines.push(*line);
        } else {
            current_message_lines.push(*line);
        }
    }

    push_pending_message(source, &mut messages, &mut current_message_lines, true)?;

    Ok(Batch {
        header,
        messages,
        trailer,
    })
}

fn push_pending_message(
    source: &str,
    messages: &mut Vec<crate::model::Message>,
    current_message_lines: &mut Vec<SegmentLine<'_>>,
    is_final: bool,
) -> Result<(), Error> {
    if current_message_lines.is_empty() {
        return Ok(());
    }

    let detail = if is_final {
        "Failed to parse final message in batch"
    } else {
        "Failed to parse message in batch"
    };
    let message = parse(segment_window(source, current_message_lines)?).map_err(|e| {
        Error::BatchParseError {
            details: format!("{}: {}", detail, e),
        }
    })?;
    messages.push(message);
    current_message_lines.clear();
    Ok(())
}

fn parse_file_batch_with_header(
    source: &str,
    lines: &[SegmentLine<'_>],
) -> Result<FileBatch, Error> {
    if !lines
        .first()
        .is_some_and(|line| line.text.starts_with("FHS"))
    {
        return Err(Error::InvalidBatchHeader {
            details: "File batch must start with FHS segment".to_string(),
        });
    }

    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {}", e),
    })?;

    let mut header = None;
    let mut batches = Vec::new();
    let mut trailer = None;
    let mut current_batch_lines = Vec::new();

    for line in lines {
        if line.text.starts_with("FHS") {
            let fhs_segment =
                parse_segment(line.text, &delims).map_err(|e| Error::InvalidBatchHeader {
                    details: format!("Failed to parse FHS segment: {}", e),
                })?;
            header = Some(fhs_segment);
        } else if line.text.starts_with("FTS") {
            let fts_segment =
                parse_segment(line.text, &delims).map_err(|e| Error::InvalidBatchTrailer {
                    details: format!("Failed to parse FTS segment: {}", e),
                })?;
            trailer = Some(fts_segment);
        } else if line.text.starts_with("BHS") {
            push_pending_batch(source, &mut batches, &mut current_batch_lines)?;
            current_batch_lines.push(*line);
        } else {
            current_batch_lines.push(*line);
        }
    }

    push_pending_batch(source, &mut batches, &mut current_batch_lines)?;

    Ok(FileBatch {
        header,
        batches,
        trailer,
    })
}

fn push_pending_batch(
    source: &str,
    batches: &mut Vec<Batch>,
    current_batch_lines: &mut Vec<SegmentLine<'_>>,
) -> Result<(), Error> {
    if current_batch_lines.is_empty() {
        return Ok(());
    }

    let batch_bytes = segment_window(source, current_batch_lines)?;
    match parse_batch(batch_bytes) {
        Ok(batch) => batches.push(batch),
        Err(e) => {
            let message = parse(batch_bytes).map_err(|_| e)?;
            batches.push(Batch {
                header: None,
                messages: vec![message],
                trailer: None,
            });
        }
    }
    current_batch_lines.clear();
    Ok(())
}

fn find_and_parse_delimiters(lines: &[SegmentLine<'_>]) -> Result<Delims, Error> {
    for line in lines {
        if line.text.starts_with("MSH") {
            return Delims::parse_from_msh(line.text);
        }
    }
    Ok(Delims::default())
}

fn segment_window<'a>(source: &'a str, lines: &[SegmentLine<'_>]) -> Result<&'a [u8], Error> {
    let Some(first) = lines.first() else {
        return Ok(&[]);
    };
    let Some(last) = lines.last() else {
        return Ok(&[]);
    };
    let window = source
        .get(first.start..last.end)
        .ok_or(Error::InvalidSegmentId)?;
    Ok(window.as_bytes())
}
