//! HL7 v2 message parser.
//!
//! This crate provides parsing functionality for HL7 v2 messages,
//! including:
//! - Message parsing from raw bytes
//! - Batch message handling (FHS/BHS/BTS/FTS)
//! - MLLP-framed message parsing
//! - Path-based field access
//!
//! # Example
//!
//! ```
//! use hl7v2_parser::parse;
//!
//! let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
//! let message = parse(hl7).unwrap();
//!
//! assert_eq!(message.segments.len(), 2);
//! ```

use hl7v2_escape::unescape_text;
use hl7v2_model::*;
use hl7v2_mllp;

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
/// use hl7v2_parser::parse;
///
/// let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
/// let message = parse(hl7).unwrap();
/// assert_eq!(message.segments.len(), 2);
/// ```
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
    let delims = Delims::parse_from_msh(lines[0]).map_err(|e| Error::ParseError {
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
    
    Ok(Message { delims, segments, charsets })
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
/// use hl7v2_parser::parse_mllp;
/// use hl7v2_mllp::wrap_mllp;
///
/// let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
/// let framed = wrap_mllp(hl7);
/// let message = parse_mllp(&framed).unwrap();
/// assert_eq!(message.segments.len(), 1);
/// ```
pub fn parse_mllp(bytes: &[u8]) -> Result<Message, Error> {
    let hl7_content = hl7v2_mllp::unwrap_mllp(bytes)?;
    parse(hl7_content)
}

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

/// Get value at path (e.g., "PID.5[1].1")
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - The path to the field (e.g., "PID.5.1", "PID.5[1].1", "MSH.9")
///
/// # Returns
///
/// The value at the path, or `None` if not found
///
/// # Example
///
/// ```
/// use hl7v2_parser::{parse, get};
///
/// let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
/// let message = parse(hl7).unwrap();
///
/// // Get the patient's last name
/// let last_name = get(&message, "PID.5.1").unwrap();
/// assert_eq!(last_name, "Doe");
/// ```
pub fn get<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    // Parse the path
    // Format: SEGMENT.FIELD[REP].COMPONENT
    // Examples: "PID.5.1", "PID.5[1].1", "MSH.9"
    
    let mut parts = path.split('.');
    let segment_id = parts.next()?;
    
    // Find the segment
    let segment = msg.segments.iter().find(|s| {
        std::str::from_utf8(&s.id).map_or(false, |id| id == segment_id)
    })?;
    
    // Parse field index (1-based)
    let field_part = parts.next()?;
    let (field_index, rep_index) = parse_field_and_rep(field_part)?;
    
    // Special handling for MSH segments
    if segment_id == "MSH" {
        get_msh_field(msg, segment, field_index, rep_index, parts)
    } else {
        get_field(segment, field_index, rep_index, parts)
    }
}

/// Get presence semantics for a field at path.
///
/// # Arguments
///
/// * `msg` - The message to query
/// * `path` - The path to the field
///
/// # Returns
///
/// The presence status of the field
pub fn get_presence(msg: &Message, path: &str) -> Presence {
    // Parse the path
    let mut parts = path.split('.');
    let segment_id = match parts.next() {
        Some(id) => id,
        None => return Presence::Missing,
    };
    
    // Find the segment
    let segment = match msg.segments.iter().find(|s| {
        std::str::from_utf8(&s.id).map_or(false, |id| id == segment_id)
    }) {
        Some(seg) => seg,
        None => return Presence::Missing,
    };
    
    // Parse field index (1-based)
    let field_part = match parts.next() {
        Some(part) => part,
        None => return Presence::Missing,
    };
    
    let (field_index, rep_index) = match parse_field_and_rep(field_part) {
        Some(indices) => indices,
        None => return Presence::Missing,
    };
    
    // Special handling for MSH segments
    if segment_id == "MSH" {
        get_msh_field_presence(msg, segment, field_index, rep_index, parts)
    } else {
        get_field_presence(segment, field_index, rep_index, parts)
    }
}

// ============================================================================
// Internal parsing functions
// ============================================================================

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
    let fields_str = if line.len() > 4 {
        &line[4..] // Skip segment ID and field separator
    } else {
        ""
    };
    
    let mut fields = parse_fields(fields_str, delims).map_err(|e| Error::ParseError {
        segment_id: String::from_utf8_lossy(&id).to_string(),
        field_index: 0,
        source: Box::new(e),
    })?;
    
    // Special handling for MSH segment
    if &id == b"MSH" {
        // MSH-2 (the encoding characters) should be treated as a single atomic value
        if !fields.is_empty() {
            let encoding_chars = String::from_iter([
                delims.comp, delims.rep, delims.esc, delims.sub
            ]);
            
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
        Ok(Segment { id, fields })
    } else {
        Ok(Segment { id, fields })
    }
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
            segment_id: "UNKNOWN".to_string(),
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
    
    for (i, rep_str) in field_str.split(delims.rep).enumerate() {
        let rep = parse_rep(rep_str, delims).map_err(|e| {
            match e {
                Error::InvalidRepFormat { .. } => e,
                _ => Error::InvalidRepFormat {
                    details: format!("Repetition {}: {}", i, e),
                }
            }
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
    
    for (i, comp_str) in rep_str.split(delims.comp).enumerate() {
        let comp = parse_comp(comp_str, delims).map_err(|e| {
            match e {
                Error::InvalidCompFormat { .. } => e,
                _ => Error::InvalidCompFormat {
                    details: format!("Component {}: {}", i, e),
                }
            }
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
    
    for (i, sub_str) in comp_str.split(delims.sub).enumerate() {
        let atom = parse_atom(sub_str, delims).map_err(|e| {
            match e {
                Error::InvalidSubcompFormat { .. } => e,
                _ => Error::InvalidSubcompFormat {
                    details: format!("Subcomponent {}: {}", i, e),
                }
            }
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

/// Extract character sets from MSH-18 field
fn extract_charsets(segments: &[Segment]) -> Vec<String> {
    // Look for the MSH segment (should be the first one)
    if let Some(msh_segment) = segments.first() {
        if &msh_segment.id == b"MSH" {
            // MSH-18 is parsed field index 17
            if msh_segment.fields.len() > 17 {
                let field_18 = &msh_segment.fields[17];
                
                if !field_18.reps.is_empty() {
                    let rep = &field_18.reps[0];
                    
                    let mut charsets = Vec::new();
                    for comp in &rep.comps {
                        if !comp.subs.is_empty() {
                            match &comp.subs[0] {
                                Atom::Text(text) => {
                                    if !text.is_empty() {
                                        charsets.push(text.clone());
                                    }
                                },
                                Atom::Null => continue,
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

/// Parse field and repetition indices from a string like "5" or "5[1]"
fn parse_field_and_rep(field_str: &str) -> Option<(usize, usize)> {
    if let Some(bracket_pos) = field_str.find('[') {
        // Has repetition index
        let field_index = field_str[..bracket_pos].parse::<usize>().ok()?;
        let rep_part = &field_str[bracket_pos + 1..];
        if let Some(end_bracket) = rep_part.find(']') {
            let rep_index = rep_part[..end_bracket].parse::<usize>().ok()?;
            Some((field_index, rep_index))
        } else {
            None
        }
    } else {
        // No repetition index, default to 1
        let field_index = field_str.parse::<usize>().ok()?;
        Some((field_index, 1))
    }
}

/// Get field value from a non-MSH segment
fn get_field<'a>(
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Option<&'a str> {
    // Convert to 0-based indexing
    if field_index == 0 {
        return None;
    }
    let zero_based_field_index = field_index - 1;
    
    // Get the field
    if zero_based_field_index >= segment.fields.len() {
        return None;
    }
    let field = &segment.fields[zero_based_field_index];
    
    // Get the repetition
    if rep_index == 0 || rep_index > field.reps.len() {
        return None;
    }
    let rep = &field.reps[rep_index - 1];
    
    // Parse component index if provided
    let comp_index = if let Some(comp_part) = parts.next() {
        comp_part.parse::<usize>().ok()?
    } else {
        1 // Default to first component
    };
    
    // Get the component
    if comp_index == 0 || comp_index > rep.comps.len() {
        return None;
    }
    let comp = &rep.comps[comp_index - 1];
    
    // Get the first subcomponent as text
    if comp.subs.is_empty() {
        return None;
    }
    
    match &comp.subs[0] {
        Atom::Text(text) => Some(text.as_str()),
        Atom::Null => None,
    }
}

/// Get field value from an MSH segment
fn get_msh_field<'a>(
    _msg: &'a Message,
    segment: &'a Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Option<&'a str> {
    if field_index == 1 {
        // MSH-1 is the field separator character
        return None; // We can't return a reference to a temporary
    } else if field_index == 2 {
        // MSH-2 is the encoding characters
        if segment.fields.is_empty() {
            return None;
        }
        let field = &segment.fields[0];
        if rep_index == 0 || rep_index > field.reps.len() {
            return None;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            comp_part.parse::<usize>().ok()?
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return None;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return None;
        }
        match &comp.subs[0] {
            Atom::Text(text) => Some(text.as_str()),
            Atom::Null => None,
        }
    } else {
        // MSH-3 and beyond
        let adjusted_field_index = field_index - 2;
        if adjusted_field_index >= segment.fields.len() {
            return None;
        }
        let field = &segment.fields[adjusted_field_index];
        if rep_index == 0 || rep_index > field.reps.len() {
            return None;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            comp_part.parse::<usize>().ok()?
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return None;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return None;
        }
        match &comp.subs[0] {
            Atom::Text(text) => Some(text.as_str()),
            Atom::Null => None,
        }
    }
}

/// Get field presence from a non-MSH segment
fn get_field_presence(
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Presence {
    if field_index == 0 {
        return Presence::Missing;
    }
    let zero_based_field_index = field_index - 1;
    
    if zero_based_field_index >= segment.fields.len() {
        return Presence::Missing;
    }
    let field = &segment.fields[zero_based_field_index];
    
    if rep_index == 0 || rep_index > field.reps.len() {
        return Presence::Missing;
    }
    let rep = &field.reps[rep_index - 1];
    
    let comp_index = if let Some(comp_part) = parts.next() {
        match comp_part.parse::<usize>() {
            Ok(index) => index,
            Err(_) => return Presence::Missing,
        }
    } else {
        1
    };
    
    if comp_index == 0 || comp_index > rep.comps.len() {
        return Presence::Missing;
    }
    let comp = &rep.comps[comp_index - 1];
    
    if comp.subs.is_empty() {
        return Presence::Missing;
    }
    
    match &comp.subs[0] {
        Atom::Text(text) => {
            if text.is_empty() {
                Presence::Empty
            } else {
                Presence::Value(text.clone())
            }
        },
        Atom::Null => Presence::Null,
    }
}

/// Get field presence from an MSH segment
fn get_msh_field_presence(
    msg: &Message,
    segment: &Segment,
    field_index: usize,
    rep_index: usize,
    mut parts: std::str::Split<char>,
) -> Presence {
    if field_index == 1 {
        // MSH-1 is the field separator character
        return Presence::Value(msg.delims.field.to_string());
    } else if field_index == 2 {
        if segment.fields.is_empty() {
            return Presence::Missing;
        }
        let field = &segment.fields[0];
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            match comp_part.parse::<usize>() {
                Ok(index) => index,
                Err(_) => return Presence::Missing,
            }
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return Presence::Missing;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return Presence::Missing;
        }
        match &comp.subs[0] {
            Atom::Text(text) => {
                if text.is_empty() {
                    Presence::Empty
                } else {
                    Presence::Value(text.clone())
                }
            },
            Atom::Null => Presence::Null,
        }
    } else {
        let adjusted_field_index = field_index - 2;
        if adjusted_field_index >= segment.fields.len() {
            return Presence::Missing;
        }
        let field = &segment.fields[adjusted_field_index];
        if rep_index == 0 || rep_index > field.reps.len() {
            return Presence::Missing;
        }
        let rep = &field.reps[rep_index - 1];
        let comp_index = if let Some(comp_part) = parts.next() {
            match comp_part.parse::<usize>() {
                Ok(index) => index,
                Err(_) => return Presence::Missing,
            }
        } else {
            1
        };
        if comp_index == 0 || comp_index > rep.comps.len() {
            return Presence::Missing;
        }
        let comp = &rep.comps[comp_index - 1];
        if comp.subs.is_empty() {
            return Presence::Missing;
        }
        match &comp.subs[0] {
            Atom::Text(text) => {
                if text.is_empty() {
                    Presence::Empty
                } else {
                    Presence::Value(text.clone())
                }
            },
            Atom::Null => Presence::Null,
        }
    }
}

/// Parse a batch that starts with BHS
fn parse_batch_with_header(lines: &[&str]) -> Result<Batch, Error> {
    if !lines[0].starts_with("BHS") {
        return Err(Error::InvalidBatchHeader {
            details: "Batch must start with BHS segment".to_string(),
        });
    }
    
    // Parse delimiters from the first MSH segment we find
    let delims = find_and_parse_delimiters(lines).map_err(|e| Error::BatchParseError {
        details: format!("Failed to parse delimiters: {}", e),
    })?;
    
    let mut header = None;
    let mut messages = Vec::new();
    let mut trailer = None;
    let mut current_message_lines = Vec::new();
    
    for &line in lines {
        if line.starts_with("BHS") {
            let bhs_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchHeader {
                details: format!("Failed to parse BHS segment: {}", e),
            })?;
            header = Some(bhs_segment);
        } else if line.starts_with("BTS") {
            let bts_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchTrailer {
                details: format!("Failed to parse BTS segment: {}", e),
            })?;
            trailer = Some(bts_segment);
        } else if line.starts_with("MSH") {
            if !current_message_lines.is_empty() {
                let message_text = current_message_lines.iter().map(|s| *s).collect::<Vec<_>>().join("\r");
                let message = parse(message_text.as_bytes()).map_err(|e| Error::BatchParseError {
                    details: format!("Failed to parse message in batch: {}", e),
                })?;
                messages.push(message);
                current_message_lines.clear();
            }
            current_message_lines.push(line);
        } else {
            current_message_lines.push(line);
        }
    }
    
    if !current_message_lines.is_empty() {
        let message_text = current_message_lines.iter().map(|s| *s).collect::<Vec<_>>().join("\r");
        let message = parse(message_text.as_bytes()).map_err(|e| Error::BatchParseError {
            details: format!("Failed to parse final message in batch: {}", e),
        })?;
        messages.push(message);
    }
    
    Ok(Batch { header, messages, trailer })
}

/// Parse a file batch that starts with FHS
fn parse_file_batch_with_header(lines: &[&str]) -> Result<FileBatch, Error> {
    if !lines[0].starts_with("FHS") {
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
    
    for &line in lines {
        if line.starts_with("FHS") {
            let fhs_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchHeader {
                details: format!("Failed to parse FHS segment: {}", e),
            })?;
            header = Some(fhs_segment);
        } else if line.starts_with("FTS") {
            let fts_segment = parse_segment(line, &delims).map_err(|e| Error::InvalidBatchTrailer {
                details: format!("Failed to parse FTS segment: {}", e),
            })?;
            trailer = Some(fts_segment);
        } else if line.starts_with("BHS") {
            if !current_batch_lines.is_empty() {
                let batch_text = current_batch_lines.iter().map(|s| *s).collect::<Vec<_>>().join("\r");
                match parse_batch(batch_text.as_bytes()) {
                    Ok(batch) => batches.push(batch),
                    Err(e) => {
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
            current_batch_lines.push(line);
        } else {
            current_batch_lines.push(line);
        }
    }
    
    if !current_batch_lines.is_empty() {
        let batch_text = current_batch_lines.iter().map(|s| *s).collect::<Vec<_>>().join("\r");
        match parse_batch(batch_text.as_bytes()) {
            Ok(batch) => batches.push(batch),
            Err(e) => {
                let message = parse(batch_text.as_bytes()).map_err(|_| e)?;
                batches.push(Batch {
                    header: None,
                    messages: vec![message],
                    trailer: None,
                });
            }
        }
    }
    
    Ok(FileBatch { header, batches, trailer })
}

/// Find and parse delimiters from the first MSH segment in the lines
fn find_and_parse_delimiters(lines: &[&str]) -> Result<Delims, Error> {
    for line in lines {
        if line.starts_with("MSH") {
            return Delims::parse_from_msh(line);
        }
    }
    Ok(Delims::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_message() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7).unwrap();
        
        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');
        
        assert_eq!(message.segments.len(), 2);
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
    }

    #[test]
    fn test_get_simple_field() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7).unwrap();
        
        // Get patient's last name (PID.5.1)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        
        // Get patient's first name (PID.5.2)
        assert_eq!(get(&message, "PID.5.2"), Some("John"));
    }

    #[test]
    fn test_get_msh_fields() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r";
        let message = parse(hl7).unwrap();
        
        // Get sending application (MSH.3)
        assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));
        
        // Get message type (MSH.9)
        assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
        assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
    }

    #[test]
    fn test_get_with_repetitions() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r";
        let message = parse(hl7).unwrap();
        
        // Test first repetition (default)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        assert_eq!(get(&message, "PID.5.2"), Some("John"));
        
        // Test second repetition
        assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
        assert_eq!(get(&message, "PID.5[2].2"), Some("Jane"));
    }

    #[test]
    fn test_parse_mllp() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\r";
        let framed = hl7v2_mllp::wrap_mllp(hl7);
        let message = parse_mllp(&framed).unwrap();
        
        assert_eq!(message.segments.len(), 1);
    }

    #[test]
    fn test_presence_semantics() {
        let hl7 = b"MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John|||\r";
        let message = parse(hl7).unwrap();
        
        // Test existing field with value
        match get_presence(&message, "PID.5.1") {
            Presence::Value(val) => assert_eq!(val, "Doe"),
            _ => panic!("Expected Value"),
        }
        
        // Test existing field with empty value
        match get_presence(&message, "PID.8.1") {
            Presence::Empty => {},
            _ => panic!("Expected Empty"),
        }
        
        // Test missing field
        match get_presence(&message, "PID.50.1") {
            Presence::Missing => {},
            _ => panic!("Expected Missing"),
        }
    }
}