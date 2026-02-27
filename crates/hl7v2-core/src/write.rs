use crate::{Atom, Batch, Comp, Delims, Field, FileBatch, Message, Rep, Segment};

/// Write HL7 message to bytes
pub fn write(msg: &Message) -> Vec<u8> {
    let mut buf = Vec::new();

    // Write segments
    for segment in &msg.segments {
        // Write segment ID
        buf.extend_from_slice(&segment.id);

        // Special handling for MSH segment
        if &segment.id == b"MSH" {
            // Write field separator
            buf.push(msg.delims.field as u8);

            // Write encoding characters as a single field
            buf.push(msg.delims.comp as u8);
            buf.push(msg.delims.rep as u8);
            buf.push(msg.delims.esc as u8);
            buf.push(msg.delims.sub as u8);

            // Write the rest of the fields
            for field in &segment.fields[1..] {
                // Skip the encoding characters field
                buf.push(msg.delims.field as u8);
                write_field(&mut buf, field, &msg.delims);
            }
        } else {
            // Write fields
            for field in &segment.fields {
                buf.push(msg.delims.field as u8);
                write_field(&mut buf, field, &msg.delims);
            }
        }

        // End segment with carriage return
        buf.push(b'\r');
    }

    buf
}

/// Wrap HL7 message bytes with MLLP framing
pub fn wrap_mllp(bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(bytes.len() + 3);

    // Add MLLP start byte (0x0B)
    buf.push(0x0B);

    // Add HL7 message content
    buf.extend_from_slice(bytes);

    // Add MLLP end sequence (0x1C 0x0D)
    buf.push(0x1C);
    buf.push(0x0D);

    buf
}

/// Write HL7 message with MLLP framing
pub fn write_mllp(msg: &Message) -> Vec<u8> {
    let hl7_bytes = write(msg);
    wrap_mllp(&hl7_bytes)
}

/// Write a field to bytes (with escaping)
fn write_field(output: &mut Vec<u8>, field: &Field, delims: &Delims) {
    for (i, rep) in field.reps.iter().enumerate() {
        if i > 0 {
            output.push(delims.rep as u8);
        }
        write_rep(output, rep, delims);
    }
}

/// Write a repetition to bytes (with escaping)
fn write_rep(output: &mut Vec<u8>, rep: &Rep, delims: &Delims) {
    for (i, comp) in rep.comps.iter().enumerate() {
        if i > 0 {
            output.push(delims.comp as u8);
        }
        write_comp(output, comp, delims);
    }
}

/// Write a component to bytes (with escaping)
fn write_comp(output: &mut Vec<u8>, comp: &Comp, delims: &Delims) {
    for (i, atom) in comp.subs.iter().enumerate() {
        if i > 0 {
            output.push(delims.sub as u8);
        }
        write_atom(output, atom, delims);
    }
}

/// Write an atom to bytes (with escaping)
fn write_atom(output: &mut Vec<u8>, atom: &Atom, delims: &Delims) {
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
pub fn escape_text(text: &str, delims: &Delims) -> String {
    // Pre-calculate maximum possible size to reduce reallocations
    // In worst case, every character might need escaping (3 chars each)
    let max_size = text.len() * 3;
    let mut result = String::with_capacity(max_size);

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

/// Write batch back to HL7 v2 format
pub fn write_batch(batch: &Batch) -> Vec<u8> {
    let mut result = Vec::new();

    // Write BHS if present
    if let Some(header) = &batch.header {
        result.extend_from_slice(&header.id);
        // We need to get delimiters from the first message or use defaults
        let delims = if let Some(first_msg) = batch.messages.first() {
            &first_msg.delims
        } else {
            &Delims::default()
        };
        result.push(delims.field as u8);
        write_segment_fields(header, &mut result, delims);
        result.push(b'\r');
    }

    // Write all messages
    for message in &batch.messages {
        result.extend(write(message));
    }

    // Write BTS if present
    if let Some(trailer) = &batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = if let Some(first_msg) = batch.messages.first() {
            &first_msg.delims
        } else {
            &Delims::default()
        };
        result.push(delims.field as u8);
        write_segment_fields(trailer, &mut result, delims);
        result.push(b'\r');
    }

    result
}

/// Write file batch back to HL7 v2 format
pub fn write_file_batch(file_batch: &FileBatch) -> Vec<u8> {
    let mut result = Vec::new();

    // Write FHS if present
    if let Some(header) = &file_batch.header {
        result.extend_from_slice(&header.id);
        // We need to get delimiters from the first message or use defaults
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(header, &mut result, &delims);
        result.push(b'\r');
    }

    // Write all batches
    for batch in &file_batch.batches {
        result.extend(write_batch(batch));
    }

    // Write FTS if present
    if let Some(trailer) = &file_batch.trailer {
        result.extend_from_slice(&trailer.id);
        let delims = get_delimiters_from_file_batch(file_batch);
        result.push(delims.field as u8);
        write_segment_fields(trailer, &mut result, &delims);
        result.push(b'\r');
    }

    result
}

/// Helper function to write segment fields
fn write_segment_fields(segment: &Segment, output: &mut Vec<u8>, delims: &Delims) {
    for (i, field) in segment.fields.iter().enumerate() {
        if i > 0 {
            output.push(delims.field as u8);
        }
        write_field(output, field, delims);
    }
}

/// Helper function to get delimiters from a file batch
fn get_delimiters_from_file_batch(file_batch: &FileBatch) -> Delims {
    // Try to get delimiters from the first message in the first batch
    if let Some(first_batch) = file_batch.batches.first() {
        if let Some(first_message) = first_batch.messages.first() {
            return first_message.delims.clone();
        }
    }
    // Fallback to default delimiters
    Delims::default()
}
