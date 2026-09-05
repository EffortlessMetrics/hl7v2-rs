from pathlib import Path


def replace_between(text: str, start: str, end: str, replacement: str) -> str:
    if text.count(start) != 1:
        raise SystemExit(f"expected one start marker: {start!r}")
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    return text[:start_index] + replacement.rstrip() + "\n\n" + text[end_index:]


batch_path = Path("crates/hl7v2/src/parser/batch.rs")
batch = batch_path.read_text()

batch = replace_between(
    batch,
    "fn parse_batch_inner(bytes: &[u8]) -> Result<Batch, Error> {",
    "/// Parse HL7 v2 file batch from bytes.",
    '''fn parse_batch_inner(bytes: &[u8]) -> Result<Batch, Error> {
    parse_batch_inner_with_delims(bytes, None)
}

fn parse_batch_inner_with_delims(
    bytes: &[u8],
    inherited_delims: Option<&Delims>,
) -> Result<Batch, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;
    let lines = segment_line_spans(text);

    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }

    let Some(first_line) = lines.first() else {
        return Err(Error::InvalidSegmentId);
    };
    if first_line.text.starts_with("BHS") {
        parse_batch_with_header(text, &lines, inherited_delims)
    } else if first_line.text.starts_with("MSH") {
        let message = parse_inner(bytes)?;
        if let Some(expected_delims) = inherited_delims {
            ensure_delimiters_match(expected_delims, &message.delims, "MSH")?;
        }
        Ok(Batch {
            header: None,
            messages: vec![message],
            trailer: None,
        })
    } else {
        Err(Error::InvalidSegmentId)
    }
}''',
)

batch = replace_between(
    batch,
    "fn parse_batch_with_header(source: &str, lines: &[SegmentLine<'_>]) -> Result<Batch, Error> {",
    "fn push_pending_message(",
    '''fn parse_batch_with_header(
    source: &str,
    lines: &[SegmentLine<'_>],
    inherited_delims: Option<&Delims>,
) -> Result<Batch, Error> {
    let Some(first_line) = lines.first() else {
        return Err(Error::InvalidBatchHeader {
            details: "Batch must start with BHS segment".to_string(),
        });
    };
    if !first_line.text.starts_with("BHS") {
        return Err(Error::InvalidBatchHeader {
            details: "Batch must start with BHS segment".to_string(),
        });
    }

    let declared_delims =
        Delims::parse_from_msh(first_line.text).map_err(|e| Error::BatchParseError {
            details: format!("Failed to parse BHS delimiter declaration: {}", e),
        })?;
    let delims = if let Some(expected_delims) = inherited_delims {
        ensure_delimiters_match(expected_delims, &declared_delims, "BHS")?;
        expected_delims.clone()
    } else {
        declared_delims
    };

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
            push_pending_message(
                source,
                &mut messages,
                &mut current_message_lines,
                &delims,
                false,
            )?;
            current_message_lines.push(*line);
        } else {
            current_message_lines.push(*line);
        }
    }

    push_pending_message(
        source,
        &mut messages,
        &mut current_message_lines,
        &delims,
        true,
    )?;

    Ok(Batch {
        header,
        messages,
        trailer,
    })
}''',
)

batch = replace_between(
    batch,
    "fn push_pending_message(",
    "fn parse_file_batch_with_header(",
    '''fn push_pending_message(
    source: &str,
    messages: &mut Vec<crate::model::Message>,
    current_message_lines: &mut Vec<SegmentLine<'_>>,
    expected_delims: &Delims,
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
    let message = parse_inner(segment_window(source, current_message_lines)?).map_err(|e| {
        Error::BatchParseError {
            details: format!("{}: {}", detail, e),
        }
    })?;
    ensure_delimiters_match(expected_delims, &message.delims, "MSH")?;
    messages.push(message);
    current_message_lines.clear();
    Ok(())
}''',
)

batch = replace_between(
    batch,
    "fn parse_file_batch_with_header(",
    "fn push_pending_batch(",
    '''fn parse_file_batch_with_header(
    source: &str,
    lines: &[SegmentLine<'_>],
) -> Result<FileBatch, Error> {
    let Some(first_line) = lines.first() else {
        return Err(Error::InvalidBatchHeader {
            details: "File batch must start with FHS segment".to_string(),
        });
    };
    if !first_line.text.starts_with("FHS") {
        return Err(Error::InvalidBatchHeader {
            details: "File batch must start with FHS segment".to_string(),
        });
    }

    let delims = Delims::parse_from_msh(first_line.text).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse FHS delimiter declaration: {}", e),
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
            push_pending_batch(
                source,
                &mut batches,
                &mut current_batch_lines,
                &delims,
            )?;
            current_batch_lines.push(*line);
        } else {
            current_batch_lines.push(*line);
        }
    }

    push_pending_batch(
        source,
        &mut batches,
        &mut current_batch_lines,
        &delims,
    )?;

    Ok(FileBatch {
        header,
        batches,
        trailer,
    })
}''',
)

batch = replace_between(
    batch,
    "fn push_pending_batch(",
    "fn find_and_parse_delimiters(",
    '''fn push_pending_batch(
    source: &str,
    batches: &mut Vec<Batch>,
    current_batch_lines: &mut Vec<SegmentLine<'_>>,
    expected_delims: &Delims,
) -> Result<(), Error> {
    if current_batch_lines.is_empty() {
        return Ok(());
    }

    let batch_bytes = segment_window(source, current_batch_lines)?;
    let batch = parse_batch_inner_with_delims(batch_bytes, Some(expected_delims))?;
    batches.push(batch);
    current_batch_lines.clear();
    Ok(())
}''',
)

batch = replace_between(
    batch,
    "fn find_and_parse_delimiters(",
    "fn segment_window<'a>",
    '''fn ensure_delimiters_match(
    expected: &Delims,
    declared: &Delims,
    declaration: &str,
) -> Result<(), Error> {
    if expected == declared {
        return Ok(());
    }

    Err(Error::BatchParseError {
        details: format!(
            "{} delimiter declaration conflicts with the enclosing envelope",
            declaration
        ),
    })
}''',
)

if "file_batch_inherits_custom_delimiters_into_message_less_batch" in batch:
    raise SystemExit("batch parser envelope regressions already exist")

batch += r'''

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_batch_inherits_custom_delimiters_into_message_less_batch() -> Result<(), Error> {
        let input = b"FHS!*~\\&!FILE\rBHS!*~\\&!EMPTY\rBTS!0\rBHS!*~\\&!FULL\rMSH!*~\\&!APP\rBTS!1\rFTS!2\r";
        let file_batch = parse_file_batch(input)?;

        assert_eq!(file_batch.batches.len(), 2);
        let first_batch = file_batch.batches.first().ok_or(Error::InvalidSegmentId)?;
        assert!(first_batch.messages.is_empty());
        let first_header = first_batch.header.as_ref().ok_or(Error::InvalidSegmentId)?;
        let first_encoding = first_header.fields.first().ok_or(Error::InvalidSegmentId)?;
        assert_eq!(first_encoding.first_text(), Some("*~\\&"));

        let second_batch = file_batch.batches.get(1).ok_or(Error::InvalidSegmentId)?;
        let message = second_batch.messages.first().ok_or(Error::InvalidSegmentId)?;
        assert_eq!(message.delims.field, '!');
        assert_eq!(message.delims.comp, '*');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');

        Ok(())
    }

    #[test]
    fn file_batch_rejects_conflicting_batch_delimiter_declaration() -> Result<(), Error> {
        let input = b"FHS|^~\\&|FILE\rBHS!*~\\&!BATCH\rBTS!0\rFTS|1\r";
        let error = parse_file_batch(input).err().ok_or(Error::InvalidSegmentId)?;

        match error {
            Error::BatchParseError { details } => {
                assert!(details.contains("BHS delimiter declaration conflicts"));
                Ok(())
            }
            other => Err(other),
        }
    }

    #[test]
    fn file_batch_rejects_conflicting_message_delimiter_declaration() -> Result<(), Error> {
        let input = b"FHS|^~\\&|FILE\rBHS|^~\\&|BATCH\rMSH!*~\\&!APP\rBTS|1\rFTS|1\r";
        let error = parse_file_batch(input).err().ok_or(Error::InvalidSegmentId)?;

        match error {
            Error::BatchParseError { details } => {
                assert!(details.contains("MSH delimiter declaration conflicts"));
                Ok(())
            }
            other => Err(other),
        }
    }
}
'''

batch_path.write_text(batch)

segment_path = Path("crates/hl7v2/src/parser/segment.rs")
segment = segment_path.read_text()
old_condition = '''    if &id == b"MSH"
        && let Some(first_field) = fields.first_mut()
    {
        *first_field = msh_encoding_field(delims);
    }
'''
new_condition = '''    if (&id == b"MSH" || &id == b"BHS" || &id == b"FHS")
        && let Some(first_field) = fields.first_mut()
    {
        *first_field = encoding_characters_field(delims);
    }
'''
if segment.count(old_condition) != 1:
    raise SystemExit("expected one MSH encoding-field normalization block")
segment = segment.replace(old_condition, new_condition, 1)
if segment.count("fn msh_encoding_field(delims: &Delims) -> Field {") != 1:
    raise SystemExit("expected one MSH encoding-field helper")
segment = segment.replace(
    "fn msh_encoding_field(delims: &Delims) -> Field {",
    "fn encoding_characters_field(delims: &Delims) -> Field {",
    1,
)

if "envelope_headers_store_encoding_characters_as_one_logical_field" in segment:
    raise SystemExit("segment envelope regression already exists")
segment += r'''

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_headers_store_encoding_characters_as_one_logical_field() -> Result<(), Error> {
        let delims = Delims::default();

        for segment_id in ["MSH", "BHS", "FHS"] {
            let line = format!("{}|^~\\&|APP", segment_id);
            let segment = parse_segment(&line, &delims)?;
            let field = segment.fields.first().ok_or(Error::InvalidSegmentId)?;

            assert_eq!(field.first_text(), Some("^~\\&"));
            assert_eq!(field.reps.len(), 1);
            let repetition = field.reps.first().ok_or(Error::InvalidSegmentId)?;
            assert_eq!(repetition.comps.len(), 1);
            let component = repetition.comps.first().ok_or(Error::InvalidSegmentId)?;
            assert_eq!(component.subs.len(), 1);
        }

        Ok(())
    }
}
'''
segment_path.write_text(segment)
