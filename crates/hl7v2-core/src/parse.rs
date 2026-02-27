use crate::{Atom, Batch, Comp, Delims, Error, Field, FileBatch, Message, Rep, Segment};

/// Parse HL7 v2 message from bytes
pub fn parse(bytes: &[u8]) -> Result<Message, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;

    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();

    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }

    // First segment must be MSH
    if !lines[0].starts_with("MSH") {
        return Err(Error::InvalidSegmentId);
    }

    // Parse delimiters from MSH segment
    let delims = parse_delimiters(lines[0]).map_err(|e| Error::ParseError {
        segment_id: "MSH".to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;

    // Parse all segments
    let mut segments = Vec::new();
    for line in lines {
        let segment = parse_segment(line, &delims).map_err(|e| Error::ParseError {
            segment_id: if line.len() >= 3 { line[..3].to_string() } else { line.to_string() },
            field_index: 0,
            source: Box::new(e),
        })?;
        segments.push(segment);
    }

    // Extract charset information from MSH-18 if present
    let charsets = extract_charsets(&segments);

    Ok(Message {
        delims,
        segments,
        charsets,
    })
}

/// Extract character sets from MSH-18 field
pub(crate) fn extract_charsets(segments: &[Segment]) -> Vec<String> {
    // Look for the MSH segment (should be the first one)
    if let Some(msh_segment) = segments.first() {
        // Check if this is an MSH segment
        if &msh_segment.id == b"MSH" {
            // MSH-18 is field index 17 (1-based indexing)
            // In parsed fields, this would be index 17 (0-based indexing)
            // But we need to account for the special MSH handling:
            // - MSH-1 (field separator) is not a parsed field
            // - MSH-2 (encoding characters) is parsed field 0
            // - MSH-3 is parsed field 1
            // - ...
            // - MSH-18 is parsed field 17

            // So we need at least 18 parsed fields (indices 0-17)
            if msh_segment.fields.len() > 17 {
                let field_18 = &msh_segment.fields[17];

                // Get the first repetition
                if !field_18.reps.is_empty() {
                    let rep = &field_18.reps[0];

                    // For MSH-18, we collect all components and filter out empty ones
                    let mut charsets = Vec::new();
                    for comp in &rep.comps {
                        if !comp.subs.is_empty() {
                            match &comp.subs[0] {
                                Atom::Text(text) => {
                                    if !text.is_empty() {
                                        charsets.push(text.clone());
                                    }
                                }
                                Atom::Null => continue, // Skip NULL values
                            }
                        }
                    }

                    return charsets;
                }
            }
        }
    }
    vec![]
}

/// Parse HL7 v2 message from MLLP framed bytes
pub fn parse_mllp(bytes: &[u8]) -> Result<Message, Error> {
    // Check if this is MLLP framed (starts with 0x0B)
    if bytes.is_empty() || bytes[0] != 0x0B {
        return Err(Error::InvalidCharset);
    }

    // Find the end sequence (0x1C 0x0D)
    let end_pos = bytes
        .windows(2)
        .position(|window| window[0] == 0x1C && window[1] == 0x0D);

    if let Some(end_pos) = end_pos {
        // Extract the HL7 message content (excluding framing bytes)
        let hl7_content = &bytes[1..end_pos];

        // Parse the HL7 message
        parse(hl7_content)
    } else {
        Err(Error::InvalidCharset)
    }
}

/// Parse HL7 v2 batch from bytes
pub fn parse_batch(bytes: &[u8]) -> Result<Batch, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;

    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();

    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }

    // Check if this is a batch (starts with BHS) or regular message (starts with MSH)
    let first_line = lines[0];
    if first_line.starts_with("BHS") {
        parse_batch_with_header(&lines)
    } else if first_line.starts_with("MSH") {
        // This is a single message, wrap it in a batch
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

/// Parse HL7 v2 file batch from bytes
pub fn parse_file_batch(bytes: &[u8]) -> Result<FileBatch, Error> {
    // Convert bytes to string
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidCharset)?;

    // Split into lines (segments)
    let lines: Vec<&str> = text.split('\r').filter(|line| !line.is_empty()).collect();

    if lines.is_empty() {
        return Err(Error::InvalidSegmentId);
    }

    // Check if this is a file batch (starts with FHS)
    let first_line = lines[0];
    if first_line.starts_with("FHS") {
        parse_file_batch_with_header(&lines)
    } else if first_line.starts_with("BHS") || first_line.starts_with("MSH") {
        // This is a batch or single message, wrap it in a file batch
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

/// Parse a batch that starts with BHS
fn parse_batch_with_header(lines: &[&str]) -> Result<Batch, Error> {
    // First line should be BHS
    if !lines[0].starts_with("BHS") {
        return Err(Error::InvalidBatchHeader {
            details: "Batch must start with BHS segment".to_string(),
        });
    }

    // Parse delimiters from the first MSH segment we find
    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {e}"),
    })?;

    let mut header = None;
    let mut messages = Vec::new();
    let mut trailer = None;
    let mut current_message_lines = Vec::new();

    for &line in lines {
        if line.starts_with("BHS") {
            // Parse BHS segment
            let bhs_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchHeader {
                details: format!("Failed to parse BHS segment: {e}"),
            })?;
            header = Some(bhs_segment);
        } else if line.starts_with("BTS") {
            // Parse BTS segment
            let bts_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchTrailer {
                details: format!("Failed to parse BTS segment: {e}"),
            })?;
            trailer = Some(bts_segment);
        } else if line.starts_with("MSH") {
            // Start of a new message
            if !current_message_lines.is_empty() {
                // Parse the previous message
                let message_text = current_message_lines.join("\r");
                let message = parse(message_text.as_bytes()).map_err(|e| Error::BatchParseError {
                    details: format!("Failed to parse message in batch: {e}"),
                })?;
                messages.push(message);
                current_message_lines.clear();
            }
            current_message_lines.push(line);
        } else {
            // Part of current message
            current_message_lines.push(line);
        }
    }

    // Parse the last message
    if !current_message_lines.is_empty() {
        let message_text = current_message_lines.join("\r");
        let message = parse(message_text.as_bytes()).map_err(|e| Error::BatchParseError {
            details: format!("Failed to parse final message in batch: {e}"),
        })?;
        messages.push(message);
    }

    Ok(Batch {
        header,
        messages,
        trailer,
    })
}

/// Parse a file batch that starts with FHS
fn parse_file_batch_with_header(lines: &[&str]) -> Result<FileBatch, Error> {
    // First line should be FHS
    if !lines[0].starts_with("FHS") {
        return Err(Error::InvalidBatchHeader {
            details: "File batch must start with FHS segment".to_string(),
        });
    }

    // Parse delimiters from the first MSH segment we find
    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {e}"),
    })?;

    let mut header = None;
    let mut batches = Vec::new();
    let mut trailer = None;
    let mut current_batch_lines = Vec::new();

    for &line in lines {
        if line.starts_with("FHS") {
            // Parse FHS segment
            let fhs_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchHeader {
                details: format!("Failed to parse FHS segment: {e}"),
            })?;
            header = Some(fhs_segment);
        } else if line.starts_with("FTS") {
            // Parse FTS segment
            let fts_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchTrailer {
                details: format!("Failed to parse FTS segment: {e}"),
            })?;
            trailer = Some(fts_segment);
        } else if line.starts_with("BHS") {
            // Start of a new batch
            if !current_batch_lines.is_empty() {
                // Parse the previous batch
                let batch_text = current_batch_lines.join("\r");
                match parse_batch(batch_text.as_bytes()) {
                    Ok(batch) => batches.push(batch),
                    Err(e) => {
                        // If parsing as batch fails, try as single message
                        let message = parse(batch_text.as_bytes()).map_err(|_| e)?;
                        batches.push(Batch {
                            header: None,
                            messages: vec![message],
                            trailer: None,
                        });
                    }
                }
                current_batch_lines.clear();
            }
            current_batch_lines.push(line);
        } else if line.starts_with("MSH") && current_batch_lines.is_empty() {
            // Start of a message when no batch has started
            current_batch_lines.push(line);
        } else {
            // Part of current batch
            current_batch_lines.push(line);
        }
    }

    // Parse the last batch
    if !current_batch_lines.is_empty() {
        let batch_text = current_batch_lines.join("\r");
        match parse_batch(batch_text.as_bytes()) {
            Ok(batch) => batches.push(batch),
            Err(e) => {
                // If parsing as batch fails, try as single message
                let message = parse(batch_text.as_bytes()).map_err(|_| e)?;
                batches.push(Batch {
                    header: None,
                    messages: vec![message],
                    trailer: None,
                });
            }
        }
    }

    Ok(FileBatch {
        header,
        batches,
        trailer,
    })
}

/// Find and parse delimiters from the first MSH segment in the lines
fn find_and_parse_delimiters(lines: &[&str]) -> Result<Delims, Error> {
    for line in lines {
        if line.starts_with("MSH") {
            return parse_delimiters(line);
        }
    }
    // If no MSH segment found, use default delimiters
    Ok(Delims::default())
}

/// Parse delimiters from MSH segment
fn parse_delimiters(msh: &str) -> Result<Delims, Error> {
    if msh.len() < 8 {
        return Err(Error::BadDelimLength);
    }

    // Extract the encoding characters directly without parsing them as regular fields
    // MSH has a special format: MSH|^~\&|... where ^~\& are the encoding characters
    let field_sep = msh.chars().nth(3).ok_or(Error::BadDelimLength)?;
    let comp_char = msh.chars().nth(4).ok_or(Error::BadDelimLength)?;
    let rep_char = msh.chars().nth(5).ok_or(Error::BadDelimLength)?;
    let esc_char = msh.chars().nth(6).ok_or(Error::BadDelimLength)?;
    let sub_char = msh.chars().nth(7).ok_or(Error::BadDelimLength)?;

    // Check that all delimiters are distinct
    let delimiters = [field_sep, comp_char, rep_char, esc_char, sub_char];
    for i in 0..delimiters.len() {
        for j in (i + 1)..delimiters.len() {
            if delimiters[i] == delimiters[j] {
                return Err(Error::DuplicateDelims);
            }
        }
    }

    Ok(Delims {
        field: field_sep,
        comp: comp_char,
        rep: rep_char,
        esc: esc_char,
        sub: sub_char,
    })
}

/// Parse a single segment
fn parse_segment(line: &str, delims: &Delims) -> Result<Segment, Error> {
    if line.len() < 3 {
        return Err(Error::InvalidSegmentId);
    }

    // Parse segment ID
    let id_bytes = line[0..3].as_bytes();
    let mut id = [0u8; 3];
    id.copy_from_slice(id_bytes);

    // Ensure segment ID is all uppercase ASCII letters or digits
    for &byte in &id {
        if !((byte >= b'A' && byte <= b'Z') || (byte >= b'0' && byte <= b'9')) {
            return Err(Error::InvalidSegmentId);
        }
    }

    // Parse fields
    let fields_str = if line.len() > 4 { &line[4..] } else { "" };

    let mut fields = parse_fields(fields_str, delims).map_err(|e| Error::ParseError {
        segment_id: String::from_utf8_lossy(&id).to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;

    // Special handling for MSH segment
    if &id == b"MSH" {
        // MSH-2 (the encoding characters) should be treated as a single atomic value
        // Currently it's being parsed incorrectly, so we need to fix it
        if !fields.is_empty() {
            // Create a field with the encoding characters as a single atomic value
            // Use direct string construction instead of format! to avoid allocation
            let encoding_chars = String::from_iter([delims.comp, delims.rep, delims.esc, delims.sub]);

            let encoding_field = Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text(encoding_chars)],
                    }],
                }],
            };
            // Replace the first field with the corrected encoding field
            fields[0] = encoding_field;
        }
    }

    Ok(Segment { id, fields })
}

/// Parse fields from a segment
fn parse_fields(fields_str: &str, delims: &Delims) -> Result<Vec<Field>, Error> {
    if fields_str.is_empty() {
        return Ok(vec![]);
    }

    // Count fields first to pre-allocate the vector
    let field_count = fields_str.matches(delims.field).count() + 1;
    let mut fields = Vec::with_capacity(field_count);

    // Use split iterator directly instead of collecting into intermediate vector
    for (i, field_str) in fields_str.split(delims.field).enumerate() {
        let field = parse_field(field_str, delims).map_err(|e| Error::ParseError {
            segment_id: "UNKNOWN".to_string(), // This will be filled in by the caller
            field_index: i,
            source: Box::new(e),
        })?;
        fields.push(field);
    }

    Ok(fields)
}

/// Parse a single field
fn parse_field(field_str: &str, delims: &Delims) -> Result<Field, Error> {
    // Validate field format
    if field_str.contains('\n') || field_str.contains('\r') {
        return Err(Error::InvalidFieldFormat {
            details: "Field contains invalid line break characters".to_string(),
        });
    }

    // Count repetitions first to pre-allocate the vector
    let rep_count = field_str.matches(delims.rep).count() + 1;
    let mut reps = Vec::with_capacity(rep_count);

    // Use split iterator directly instead of collecting into intermediate vector
    for (i, rep_str) in field_str.split(delims.rep).enumerate() {
        let rep = parse_rep(rep_str, delims).map_err(|e| match e {
            Error::InvalidRepFormat { .. } => e,
            _ => Error::InvalidRepFormat {
                details: format!("Repetition {i}: {e}"),
            },
        })?;
        reps.push(rep);
    }

    Ok(Field { reps })
}

/// Parse a repetition
fn parse_rep(rep_str: &str, delims: &Delims) -> Result<Rep, Error> {
    // Handle NULL value
    if rep_str == "\"\"" {
        return Ok(Rep {
            comps: vec![Comp {
                subs: vec![Atom::Null],
            }],
        });
    }

    // Validate repetition format
    if rep_str.contains('\n') || rep_str.contains('\r') {
        return Err(Error::InvalidRepFormat {
            details: "Repetition contains invalid line break characters".to_string(),
        });
    }

    // Count components first to pre-allocate the vector
    let comp_count = rep_str.matches(delims.comp).count() + 1;
    let mut comps = Vec::with_capacity(comp_count);

    // Use split iterator directly instead of collecting into intermediate vector
    for (i, comp_str) in rep_str.split(delims.comp).enumerate() {
        let comp = parse_comp(comp_str, delims).map_err(|e| match e {
            Error::InvalidCompFormat { .. } => e,
            _ => Error::InvalidCompFormat {
                details: format!("Component {i}: {e}"),
            },
        })?;
        comps.push(comp);
    }

    Ok(Rep { comps })
}

/// Parse a component
fn parse_comp(comp_str: &str, delims: &Delims) -> Result<Comp, Error> {
    // Validate component format
    if comp_str.contains('\n') || comp_str.contains('\r') {
        return Err(Error::InvalidCompFormat {
            details: "Component contains invalid line break characters".to_string(),
        });
    }

    // Count subcomponents first to pre-allocate the vector
    let sub_count = comp_str.matches(delims.sub).count() + 1;
    let mut subs = Vec::with_capacity(sub_count);

    // Use split iterator directly instead of collecting into intermediate vector
    for (i, sub_str) in comp_str.split(delims.sub).enumerate() {
        let atom = parse_atom(sub_str, delims).map_err(|e| match e {
            Error::InvalidSubcompFormat { .. } => e,
            _ => Error::InvalidSubcompFormat {
                details: format!("Subcomponent {i}: {e}"),
            },
        })?;
        subs.push(atom);
    }

    Ok(Comp { subs })
}

/// Parse an atom (unescaped text or NULL)
fn parse_atom(atom_str: &str, delims: &Delims) -> Result<Atom, Error> {
    // Handle NULL value
    if atom_str == "\"\"" {
        return Ok(Atom::Null);
    }

    // Validate atom format
    if atom_str.contains('\n') || atom_str.contains('\r') {
        return Err(Error::InvalidSubcompFormat {
            details: "Subcomponent contains invalid line break characters".to_string(),
        });
    }

    // Unescape the text
    let unescaped = unescape_text(atom_str, delims)?;
    Ok(Atom::Text(unescaped))
}

/// Unescape text according to HL7 v2 rules
pub fn unescape_text(text: &str, delims: &Delims) -> Result<String, Error> {
    // Pre-allocate result with estimated capacity to reduce reallocations
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == delims.esc {
            // Start of escape sequence
            let mut escape_seq = String::new();
            let mut found_end = false;

            while let Some(esc_ch) = chars.next() {
                if esc_ch == delims.esc {
                    found_end = true;
                    break;
                }
                escape_seq.push(esc_ch);
            }

            if !found_end {
                // If we don't find the closing escape character, this might be a literal backslash
                // in the encoding characters. Let's check if this is the special case of the
                // MSH encoding characters "^~\&"
                // Use direct comparison instead of format! to avoid allocation
                if text.len() == 4
                    && text.chars().nth(0) == Some(delims.comp)
                    && text.chars().nth(1) == Some(delims.rep)
                    && text.chars().nth(2) == Some(delims.esc)
                    && text.chars().nth(3) == Some(delims.sub)
                {
                    // This is the MSH encoding characters, treat as literal
                    result.push(delims.comp);
                    result.push(delims.rep);
                    result.push(delims.esc);
                    result.push(delims.sub);
                    // Skip the rest of the processing since we've handled the special case
                    return Ok(result);
                }

                // For other cases, treat the text as-is
                result.push(delims.esc);
                result.push_str(&escape_seq);
                continue;
            }

            // Process escape sequence
            match escape_seq.as_str() {
                "F" => {
                    result.push(delims.field);
                }
                "S" => {
                    result.push(delims.comp);
                }
                "R" => {
                    result.push(delims.rep);
                }
                "E" => {
                    result.push(delims.esc);
                }
                "T" => {
                    result.push(delims.sub);
                }
                _ => {
                    // Unknown escape sequences are passed through
                    result.push(delims.esc);
                    result.push_str(&escape_seq);
                    result.push(delims.esc);
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}
