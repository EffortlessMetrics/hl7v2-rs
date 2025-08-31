//! Core parsing and data model for HL7 v2 messages.
//!
//! This crate provides the foundational data structures and parsing logic
//! for HL7 v2 messages, including:
//! - Message parsing from raw bytes
//! - Data model representation (Message, Segment, Field, etc.)
//! - Escape sequence handling
//! - JSON serialization

/// Delimiters used in HL7 v2 messages
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delims {
    pub field: char,
    pub comp: char,
    pub rep: char,
    pub esc: char,
    pub sub: char,
}

impl Delims {
    /// Create default delimiters (|^~\&)
    pub fn default() -> Self {
        Self {
            field: '|',
            comp: '^',
            rep: '~',
            esc: '\\',
            sub: '&',
        }
    }
}

/// Main message structure
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub delims: Delims,
    pub segments: Vec<Segment>,
}

/// A segment in an HL7 message
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub id: [u8; 3],
    pub fields: Vec<Field>,
}

/// A field in a segment
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub reps: Vec<Rep>,
}

/// A repetition of a field
#[derive(Debug, Clone, PartialEq)]
pub struct Rep {
    pub comps: Vec<Comp>,
}

/// A component of a field
#[derive(Debug, Clone, PartialEq)]
pub struct Comp {
    pub subs: Vec<Atom>,
}

/// An atomic value in the message
#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Text(String),
    Null,
}

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
    let delims = parse_delimiters(lines[0])?;
    
    // Parse all segments
    let mut segments = Vec::new();
    for line in lines {
        segments.push(parse_segment(line, &delims)?);
    }
    
    Ok(Message { delims, segments })
}

/// Parse delimiters from MSH segment
fn parse_delimiters(msh: &str) -> Result<Delims, Error> {
    if msh.len() < 8 {
        return Err(Error::BadDelimLength);
    }
    
    let field = msh.chars().nth(3).ok_or(Error::BadDelimLength)?;
    let comp = msh.chars().nth(4).ok_or(Error::BadDelimLength)?;
    let rep = msh.chars().nth(5).ok_or(Error::BadDelimLength)?;
    let esc = msh.chars().nth(6).ok_or(Error::BadDelimLength)?;
    let sub = msh.chars().nth(7).ok_or(Error::BadDelimLength)?;
    
    // Check that all delimiters are distinct
    let delimiters = [field, comp, rep, esc, sub];
    for i in 0..delimiters.len() {
        for j in (i + 1)..delimiters.len() {
            if delimiters[i] == delimiters[j] {
                return Err(Error::DuplicateDelims);
            }
        }
    }
    
    Ok(Delims {
        field,
        comp,
        rep,
        esc,
        sub,
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
    
    // Ensure segment ID is all uppercase ASCII letters
    for &byte in &id {
        if byte < b'A' || byte > b'Z' {
            return Err(Error::InvalidSegmentId);
        }
    }
    
    // Parse fields
    let fields_str = if line.len() > 3 {
        &line[4..] // Skip segment ID and field separator
    } else {
        ""
    };
    
    let fields = parse_fields(fields_str, delims)?;
    
    Ok(Segment {
        id,
        fields,
    })
}

/// Parse fields from a segment
fn parse_fields(fields_str: &str, delims: &Delims) -> Result<Vec<Field>, Error> {
    if fields_str.is_empty() {
        return Ok(vec![]);
    }
    
    let field_strings: Vec<&str> = fields_str.split(delims.field).collect();
    let mut fields = Vec::new();
    
    for field_str in field_strings {
        fields.push(parse_field(field_str, delims)?);
    }
    
    Ok(fields)
}

/// Parse a single field
fn parse_field(field_str: &str, delims: &Delims) -> Result<Field, Error> {
    let rep_strings: Vec<&str> = field_str.split(delims.rep).collect();
    let mut reps = Vec::new();
    
    for rep_str in rep_strings {
        reps.push(parse_rep(rep_str, delims)?);
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
    
    let comp_strings: Vec<&str> = rep_str.split(delims.comp).collect();
    let mut comps = Vec::new();
    
    for comp_str in comp_strings {
        comps.push(parse_comp(comp_str, delims)?);
    }
    
    Ok(Rep { comps })
}

/// Parse a component
fn parse_comp(comp_str: &str, delims: &Delims) -> Result<Comp, Error> {
    let sub_strings: Vec<&str> = comp_str.split(delims.sub).collect();
    let mut subs = Vec::new();
    
    for sub_str in sub_strings {
        subs.push(parse_atom(sub_str, delims)?);
    }
    
    Ok(Comp { subs })
}

/// Parse an atom (unescaped text or NULL)
fn parse_atom(atom_str: &str, delims: &Delims) -> Result<Atom, Error> {
    // Handle NULL value
    if atom_str == "\"\"" {
        return Ok(Atom::Null);
    }
    
    // Unescape the text
    let unescaped = unescape_text(atom_str, delims)?;
    Ok(Atom::Text(unescaped))
}

/// Unescape text according to HL7 v2 rules
fn unescape_text(text: &str, delims: &Delims) -> Result<String, Error> {
    let mut result = String::new();
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
                // If we don't find the closing escape character, treat the text as-is
                result.push(delims.esc);
                result.push_str(&escape_seq);
                continue;
            }
            
            // Process escape sequence
            match escape_seq.as_str() {
                "F" => result.push(delims.field),
                "S" => result.push(delims.comp),
                "R" => result.push(delims.rep),
                "E" => result.push(delims.esc),
                "T" => result.push(delims.sub),
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

/// Write message back to HL7 v2 format
pub fn write(msg: &Message) -> Vec<u8> {
    let mut result = Vec::new();
    
    for segment in &msg.segments {
        // Write segment ID
        result.extend_from_slice(&segment.id);
        
        // Write field separator
        result.push(msg.delims.field as u8);
        
        // Write fields
        for (i, field) in segment.fields.iter().enumerate() {
            if i > 0 {
                result.push(msg.delims.field as u8);
            }
            write_field(field, &mut result, &msg.delims);
        }
        
        // Write segment terminator
        result.push(b'\r');
    }
    
    result
}

/// Write a field to bytes
fn write_field(field: &Field, output: &mut Vec<u8>, delims: &Delims) {
    for (i, rep) in field.reps.iter().enumerate() {
        if i > 0 {
            output.push(delims.rep as u8);
        }
        write_rep(rep, output, delims);
    }
}

/// Write a repetition to bytes
fn write_rep(rep: &Rep, output: &mut Vec<u8>, delims: &Delims) {
    for (i, comp) in rep.comps.iter().enumerate() {
        if i > 0 {
            output.push(delims.comp as u8);
        }
        write_comp(comp, output, delims);
    }
}

/// Write a component to bytes
fn write_comp(comp: &Comp, output: &mut Vec<u8>, delims: &Delims) {
    for (i, atom) in comp.subs.iter().enumerate() {
        if i > 0 {
            output.push(delims.sub as u8);
        }
        write_atom(atom, output, delims);
    }
}

/// Write an atom to bytes
fn write_atom(atom: &Atom, output: &mut Vec<u8>, delims: &Delims) {
    match atom {
        Atom::Text(text) => {
            // Escape special characters
            let escaped = escape_text(text, delims);
            output.extend_from_slice(escaped.as_bytes());
        }
        Atom::Null => {
            output.extend_from_slice(b"\"\"");
        }
    }
}

/// Escape text according to HL7 v2 rules
fn escape_text(text: &str, delims: &Delims) -> String {
    let mut result = String::new();
    
    for ch in text.chars() {
        match ch {
            c if c == delims.field => {
                result.push(delims.esc);
                result.push('F');
                result.push(delims.esc);
            }
            c if c == delims.comp => {
                result.push(delims.esc);
                result.push('S');
                result.push(delims.esc);
            }
            c if c == delims.rep => {
                result.push(delims.esc);
                result.push('R');
                result.push(delims.esc);
            }
            c if c == delims.esc => {
                result.push(delims.esc);
                result.push('E');
                result.push(delims.esc);
            }
            c if c == delims.sub => {
                result.push(delims.esc);
                result.push('T');
                result.push(delims.esc);
            }
            _ => result.push(ch),
        }
    }
    
    result
}

/// Convert message to canonical JSON
pub fn to_json(msg: &Message) -> serde_json::Value {
    use serde_json::json;
    
    let segments: Vec<serde_json::Value> = msg
        .segments
        .iter()
        .map(|segment| {
            let segment_id = String::from_utf8_lossy(&segment.id).to_string();
            let fields: serde_json::Map<String, serde_json::Value> = segment
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    if field.reps.is_empty() {
                        None
                    } else {
                        let field_value = field_to_json(field);
                        Some(((index + 1).to_string(), field_value))
                    }
                })
                .collect();
            
            json!({
                "id": segment_id,
                "fields": fields
            })
        })
        .collect();
    
    json!({
        "meta": {
            "delims": {
                "field": msg.delims.field.to_string(),
                "comp": msg.delims.comp.to_string(),
                "rep": msg.delims.rep.to_string(),
                "esc": msg.delims.esc.to_string(),
                "sub": msg.delims.sub.to_string()
            }
        },
        "segments": segments
    })
}

/// Convert a field to JSON
fn field_to_json(field: &Field) -> serde_json::Value {
    use serde_json::json;
    
    let reps: Vec<serde_json::Value> = field
        .reps
        .iter()
        .map(|rep| {
            let comps: Vec<serde_json::Value> = rep
                .comps
                .iter()
                .map(|comp| {
                    let subs: Vec<serde_json::Value> = comp
                        .subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => json!(text),
                            Atom::Null => json!("__NULL__"),
                        })
                        .collect();
                    json!(subs)
                })
                .collect();
            json!(comps)
        })
        .collect();
    
    json!(reps)
}

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
}

/// Get value at path (e.g., "PID.5[1].1")
pub fn get<'a>(msg: &'a Message, path: &str) -> Option<&'a str> {
    // Simple implementation for now - will be expanded later
    None
}

#[cfg(test)]
mod tests;