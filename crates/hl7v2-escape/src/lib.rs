//! HL7 v2 escape sequence handling.
//!
//! This crate provides functions for escaping and unescaping HL7 v2 text
//! according to the standard escape sequences defined in the HL7 v2 specification.
//!
//! # Escape Sequences
//!
//! HL7 v2 uses escape sequences to represent delimiter characters within field values:
//! - `\F\` - Field separator
//! - `\S\` - Component separator
//! - `\R\` - Repetition separator
//! - `\E\` - Escape character
//! - `\T\` - Subcomponent separator
//!
//! # Example
//!
//! ```
//! use hl7v2_escape::{escape_text, unescape_text};
//! use hl7v2_model::Delims;
//!
//! let delims = Delims::default();
//! let text = "test|value";
//! let escaped = escape_text(text, &delims);
//! assert_eq!(escaped, "test\\F\\value");
//!
//! let unescaped = unescape_text(&escaped, &delims).unwrap();
//! assert_eq!(unescaped, text);
//! ```

use hl7v2_model::{Delims, Error};

/// Escape text according to HL7 v2 rules.
///
/// This function replaces delimiter characters with their escape sequences.
///
/// # Arguments
///
/// * `text` - The text to escape
/// * `delims` - The delimiter configuration
///
/// # Returns
///
/// The escaped text string
///
/// # Example
///
/// ```
/// use hl7v2_escape::escape_text;
/// use hl7v2_model::Delims;
///
/// let delims = Delims::default();
/// let escaped = escape_text("a|b^c", &delims);
/// assert_eq!(escaped, "a\\F\\b\\S\\c");
/// ```
pub fn escape_text(text: &str, delims: &Delims) -> String {
    // Fast path: find the first character that needs escaping
    // Use str::find which handles UTF-8 correctly and avoids multi-byte truncation panics.
    let first_esc_byte_idx = text.find(&[
        delims.field,
        delims.comp,
        delims.rep,
        delims.esc,
        delims.sub,
    ][..]);

    let first_esc_byte_idx = match first_esc_byte_idx {
        Some(idx) => idx,
        None => return text.to_string(), // Fast path: nothing to escape
    };

    // Pre-calculate maximum possible size to reduce reallocations
    // In worst case, every character might need escaping (3 chars each)
    let prefix = &text[..first_esc_byte_idx];
    let remaining_text = &text[first_esc_byte_idx..];
    let max_size = prefix.len() + remaining_text.len() * 3;
    let mut result = String::with_capacity(max_size);

    // Fast path: bulk copy the clean prefix
    result.push_str(prefix);

    // Process only the remaining characters
    for ch in remaining_text.chars() {
        match ch {
            x if x == delims.field => {
                result.push(delims.esc);
                result.push('F');
                result.push(delims.esc);
            }
            x if x == delims.comp => {
                result.push(delims.esc);
                result.push('S');
                result.push(delims.esc);
            }
            x if x == delims.rep => {
                result.push(delims.esc);
                result.push('R');
                result.push(delims.esc);
            }
            x if x == delims.esc => {
                result.push(delims.esc);
                result.push('E');
                result.push(delims.esc);
            }
            x if x == delims.sub => {
                result.push(delims.esc);
                result.push('T');
                result.push(delims.esc);
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Unescape text according to HL7 v2 rules.
///
/// This function replaces escape sequences with their actual characters.
///
/// # Arguments
///
/// * `text` - The text to unescape
/// * `delims` - The delimiter configuration
///
/// # Returns
///
/// The unescaped text string, or an error if the escape sequence is malformed
///
/// # Example
///
/// ```
/// use hl7v2_escape::unescape_text;
/// use hl7v2_model::Delims;
///
/// let delims = Delims::default();
/// let unescaped = unescape_text("a\\F\\b", &delims).unwrap();
/// assert_eq!(unescaped, "a|b");
/// ```
pub fn unescape_text(text: &str, delims: &Delims) -> Result<String, Error> {
    // Fast path: finding the first escape character using `str::find`
    // prevents unnecessary allocations and memory operations. If no escape
    // character is found, we can safely return the original string.
    if let Some(first_esc_byte_idx) = text.find(delims.esc) {
        let prefix = &text[..first_esc_byte_idx];
        let remaining_text = &text[first_esc_byte_idx..];

        // Pre-allocate result with estimated capacity to reduce reallocations
        let mut result = String::with_capacity(text.len());

        // Fast path: bulk copy the clean prefix
        result.push_str(prefix);

        let mut chars = remaining_text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == delims.esc {
                // Start of escape sequence
                let mut escape_seq = String::new();
                let mut found_end = false;

                for esc_ch in chars.by_ref() {
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
                    if text.len() == 4 {
                        let chars: Vec<char> = text.chars().collect();
                        if chars[0] == delims.comp
                            && chars[1] == delims.rep
                            && chars[2] == delims.esc
                            && chars[3] == delims.sub
                        {
                            // This is the MSH encoding characters, treat as literal
                            result.push(delims.comp);
                            result.push(delims.rep);
                            result.push(delims.esc);
                            result.push(delims.sub);
                            // Skip the rest of the processing since we've handled the special case
                            return Ok(result);
                        }
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

        return Ok(result);
    }

    // No escape sequences found, string is clean
    Ok(text.to_string())
}

/// Check if text contains any characters that need escaping.
///
/// # Arguments
///
/// * `text` - The text to check
/// * `delims` - The delimiter configuration
///
/// # Returns
///
/// `true` if the text contains any delimiter characters
pub fn needs_escaping(text: &str, delims: &Delims) -> bool {
    text.contains(&[
        delims.field,
        delims.comp,
        delims.rep,
        delims.esc,
        delims.sub,
    ][..])
}

/// Check if text contains any escape sequences.
///
/// # Arguments
///
/// * `text` - The text to check
/// * `delims` - The delimiter configuration
///
/// # Returns
///
/// `true` if the text contains escape sequences
pub fn needs_unescaping(text: &str, delims: &Delims) -> bool {
    text.contains(delims.esc)
}

#[cfg(test)]
mod tests;
