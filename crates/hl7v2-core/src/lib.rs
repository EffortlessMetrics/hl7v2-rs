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
    println!("DEBUG: Parsed delimiters - field:'{}' comp:'{}' rep:'{}' esc:'{}' sub:'{}'", 
             delims.field, delims.comp, delims.rep, delims.esc, delims.sub);
    
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
    
    // First, we need to unescape the MSH segment to get the correct delimiters
    // But we need to parse the escape character first to do that
    let esc = msh.chars().nth(6).ok_or(Error::BadDelimLength)?;
    
    let field = msh.chars().nth(3).ok_or(Error::BadDelimLength)?;
    let comp = msh.chars().nth(4).ok_or(Error::BadDelimLength)?;
    let rep = msh.chars().nth(5).ok_or(Error::BadDelimLength)?;
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
    let fields_str = if line.len() > 4 {
        &line[4..] // Skip segment ID and field separator
    } else {
        ""
    };
    
    let mut fields = parse_fields(fields_str, delims)?;
    
    // Special handling for MSH segment
    if &id == b"MSH" {
        println!("DEBUG: Processing MSH segment, parsed {} fields", fields.len());
        // MSH-2 (the encoding characters) should be treated as a single atomic value
        // Currently it's being parsed incorrectly, so we need to fix it
        if !fields.is_empty() {
            // Create a field with the encoding characters as a single atomic value
            let encoding_field = Field {
                reps: vec![Rep {
                    comps: vec![Comp {
                        subs: vec![Atom::Text(format!("{}{}{}{}", 
                            delims.comp, delims.rep, delims.esc, delims.sub))],
                    }],
                }],
            };
            // Replace the first field with the corrected encoding field
            fields[0] = encoding_field;
        }
        Ok(Segment {
            id,
            fields,
        })
    } else {
        Ok(Segment {
            id,
            fields,
        })
    }
}

/// Parse fields from a segment
fn parse_fields(fields_str: &str, delims: &Delims) -> Result<Vec<Field>, Error> {
    if fields_str.is_empty() {
        return Ok(vec![]);
    }
    
    println!("DEBUG: Parsing fields from: '{}'", fields_str);
    let field_strings: Vec<&str> = fields_str.split(delims.field).collect();
    println!("DEBUG: Found {} field strings: {:?}", field_strings.len(), field_strings);
    
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
            println!("DEBUG: Found escape character in '{}'", text);
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
                println!("DEBUG: No closing escape found, treating as-is");
                result.push(delims.esc);
                result.push_str(&escape_seq);
                continue;
            }
            
            println!("DEBUG: Processing escape sequence: '{}'", escape_seq);
            
            // Process escape sequence
            match escape_seq.as_str() {
                "F" => {
                    println!("DEBUG: Escaping F to field delimiter");
                    result.push(delims.field);
                },
                "S" => {
                    println!("DEBUG: Escaping S to comp delimiter");
                    result.push(delims.comp);
                },
                "R" => {
                    println!("DEBUG: Escaping R to rep delimiter");
                    result.push(delims.rep);
                },
                "E" => {
                    println!("DEBUG: Escaping E to esc delimiter");
                    result.push(delims.esc);
                },
                "T" => {
                    println!("DEBUG: Escaping T to sub delimiter");
                    result.push(delims.sub);
                },
                _ => {
                    // Unknown escape sequences are passed through
                    println!("DEBUG: Unknown escape sequence, passing through");
                    result.push(delims.esc);
                    result.push_str(&escape_seq);
                    result.push(delims.esc);
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    println!("DEBUG: Unescaped '{}' to '{}'", text, result);
    Ok(result)
}

/// Write message back to HL7 v2 format
pub fn write(msg: &Message) -> Vec<u8> {
    let mut result = Vec::new();
    
    for (segment_idx, segment) in msg.segments.iter().enumerate() {
        // Write segment ID
        println!("DEBUG: Writing segment {}: ID {:?}", segment_idx, std::str::from_utf8(&segment.id));
        result.extend_from_slice(&segment.id);
        
        // Write field separator
        result.push(msg.delims.field as u8);
        
        // Special handling for MSH segment
        if &segment.id == b"MSH" {
            println!("DEBUG: MSH segment detected, fields count: {}", segment.fields.len());
            // For MSH segment, write all fields (MSH-2 and beyond)
            // MSH-1 (the field separator) is implicit and not stored as a field
            for (i, field) in segment.fields.iter().enumerate() {
                if i > 0 {
                    result.push(msg.delims.field as u8);
                }
                println!("DEBUG: Writing MSH field {}: reps={}", i, field.reps.len());
                // Special handling: for MSH segment fields, don't escape the delimiter characters
                write_field_no_escape(field, &mut result, &msg.delims);
            }
        } else {
            println!("DEBUG: Non-MSH segment, ID: {:?}, fields count: {}", 
                     std::str::from_utf8(&segment.id), segment.fields.len());
            // For non-MSH segments, write all fields
            for (i, field) in segment.fields.iter().enumerate() {
                if i > 0 {
                    result.push(msg.delims.field as u8);
                }
                println!("DEBUG: Writing field {}: reps={}", i, field.reps.len());
                write_field(field, &mut result, &msg.delims);
            }
        }
        
        // Write segment terminator
        result.push(b'\r');
    }
    
    result
}

/// Write a field to bytes (with escaping)
fn write_field(field: &Field, output: &mut Vec<u8>, delims: &Delims) {
    for (i, rep) in field.reps.iter().enumerate() {
        if i > 0 {
            output.push(delims.rep as u8);
        }
        write_rep(rep, output, delims);
    }
}

/// Write a field to bytes (without escaping delimiter characters)
fn write_field_no_escape(field: &Field, output: &mut Vec<u8>, delims: &Delims) {
    for (i, rep) in field.reps.iter().enumerate() {
        if i > 0 {
            output.push(delims.rep as u8);
        }
        write_rep_no_escape(rep, output, delims);
    }
}

/// Write a repetition to bytes (with escaping)
fn write_rep(rep: &Rep, output: &mut Vec<u8>, delims: &Delims) {
    for (i, comp) in rep.comps.iter().enumerate() {
        if i > 0 {
            output.push(delims.comp as u8);
        }
        write_comp(comp, output, delims);
    }
}

/// Write a repetition to bytes (without escaping delimiter characters)
fn write_rep_no_escape(rep: &Rep, output: &mut Vec<u8>, delims: &Delims) {
    for (i, comp) in rep.comps.iter().enumerate() {
        if i > 0 {
            output.push(delims.comp as u8);
        }
        write_comp_no_escape(comp, output, delims);
    }
}

/// Write a component to bytes (with escaping)
fn write_comp(comp: &Comp, output: &mut Vec<u8>, delims: &Delims) {
    for (i, atom) in comp.subs.iter().enumerate() {
        if i > 0 {
            output.push(delims.sub as u8);
        }
        write_atom(atom, output, delims);
    }
}

/// Write a component to bytes (without escaping delimiter characters)
fn write_comp_no_escape(comp: &Comp, output: &mut Vec<u8>, delims: &Delims) {
    for (i, atom) in comp.subs.iter().enumerate() {
        if i > 0 {
            output.push(delims.sub as u8);
        }
        write_atom_no_escape(atom, output, delims);
    }
}

/// Write an atom to bytes (with escaping)
fn write_atom(atom: &Atom, output: &mut Vec<u8>, delims: &Delims) {
    match atom {
        Atom::Text(text) => {
            // Escape special characters
            let escaped = escape_text(text, delims);
            println!("DEBUG: Writing atom text: '{}' -> '{}'", text, escaped);
            output.extend_from_slice(escaped.as_bytes());
        }
        Atom::Null => {
            output.extend_from_slice(b"\"\"");
        }
    }
}

/// Write an atom to bytes (without escaping delimiter characters)
fn write_atom_no_escape(atom: &Atom, output: &mut Vec<u8>, _delims: &Delims) {
    match atom {
        Atom::Text(text) => {
            // Don't escape special characters for MSH segment fields
            println!("DEBUG: Writing atom text (no escape): '{}'", text);
            output.extend_from_slice(text.as_bytes());
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
        if field_index == 1 {
            // MSH-1 is the field separator character
            // We can't return a reference to a temporary string, so we don't support this case
            // Users should access msg.delims.field directly for the field separator
            return None;
        } else if field_index == 2 {
            // MSH-2 is the encoding characters
            // This should be the first parsed field (index 0)
            if segment.fields.is_empty() {
                return None;
            }
            let field = &segment.fields[0];
            // Get the repetition
            if rep_index == 0 || rep_index > field.reps.len() {
                return None;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                comp_part.parse::<usize>().ok()?
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return None;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return None;
            }
            match &comp.subs[0] {
                Atom::Text(text) => Some(text.as_str()),
                Atom::Null => None,
            }
        } else {
            // MSH-3 and beyond
            // Adjust index: MSH-3 maps to parsed field 1, MSH-4 to parsed field 2, etc.
            let adjusted_field_index = field_index - 2;
            if adjusted_field_index >= segment.fields.len() {
                return None;
            }
            let field = &segment.fields[adjusted_field_index];
            // Get the repetition
            if rep_index == 0 || rep_index > field.reps.len() {
                return None;
            }
            let rep = &field.reps[rep_index - 1];
            // Get the component
            let comp_index = if let Some(comp_part) = parts.next() {
                comp_part.parse::<usize>().ok()?
            } else {
                1
            };
            if comp_index == 0 || comp_index > rep.comps.len() {
                return None;
            }
            let comp = &rep.comps[comp_index - 1];
            // Get the subcomponent
            if comp.subs.is_empty() {
                return None;
            }
            match &comp.subs[0] {
                Atom::Text(text) => Some(text.as_str()),
                Atom::Null => None,
            }
        }
    } else {
        // For non-MSH segments, convert directly to 0-based indexing
        if field_index == 0 {
            return None;
        }
        let zero_based_field_index = field_index - 1;
        
        // Get the field
        if zero_based_field_index >= segment.fields.len() {
            return None;
        }
        let field = &segment.fields[zero_based_field_index];
        
        // Get the repetition (convert to 0-based indexing)
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
        
        // Get the component (convert to 0-based indexing)
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

#[cfg(test)]
mod tests;