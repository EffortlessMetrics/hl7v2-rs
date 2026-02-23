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
                if text.len() == 4 && 
                   text.chars().nth(0) == Some(delims.comp) &&
                   text.chars().nth(1) == Some(delims.rep) &&
                   text.chars().nth(2) == Some(delims.esc) &&
                   text.chars().nth(3) == Some(delims.sub) {
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
                },
                "S" => {
                    result.push(delims.comp);
                },
                "R" => {
                    result.push(delims.rep);
                },
                "E" => {
                    result.push(delims.esc);
                },
                "T" => {
                    result.push(delims.sub);
                },
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
    text.chars().any(|c| {
        c == delims.field ||
        c == delims.comp ||
        c == delims.rep ||
        c == delims.esc ||
        c == delims.sub
    })
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
mod tests {
    use super::*;
    
    #[test]
    fn test_escape_field_separator() {
        let delims = Delims::default();
        let escaped = escape_text("a|b", &delims);
        assert_eq!(escaped, "a\\F\\b");
    }
    
    #[test]
    fn test_escape_component_separator() {
        let delims = Delims::default();
        let escaped = escape_text("a^b", &delims);
        assert_eq!(escaped, "a\\S\\b");
    }
    
    #[test]
    fn test_escape_repetition_separator() {
        let delims = Delims::default();
        let escaped = escape_text("a~b", &delims);
        assert_eq!(escaped, "a\\R\\b");
    }
    
    #[test]
    fn test_escape_escape_character() {
        let delims = Delims::default();
        let escaped = escape_text("a\\b", &delims);
        assert_eq!(escaped, "a\\E\\b");
    }
    
    #[test]
    fn test_escape_subcomponent_separator() {
        let delims = Delims::default();
        let escaped = escape_text("a&b", &delims);
        assert_eq!(escaped, "a\\T\\b");
    }
    
    #[test]
    fn test_escape_multiple_delimiters() {
        let delims = Delims::default();
        let escaped = escape_text("a|b^c~d\\e&f", &delims);
        assert_eq!(escaped, "a\\F\\b\\S\\c\\R\\d\\E\\e\\T\\f");
    }
    
    #[test]
    fn test_escape_no_special_chars() {
        let delims = Delims::default();
        let escaped = escape_text("normal text", &delims);
        assert_eq!(escaped, "normal text");
    }
    
    #[test]
    fn test_unescape_field_separator() {
        let delims = Delims::default();
        let unescaped = unescape_text("a\\F\\b", &delims).unwrap();
        assert_eq!(unescaped, "a|b");
    }
    
    #[test]
    fn test_unescape_component_separator() {
        let delims = Delims::default();
        let unescaped = unescape_text("a\\S\\b", &delims).unwrap();
        assert_eq!(unescaped, "a^b");
    }
    
    #[test]
    fn test_unescape_repetition_separator() {
        let delims = Delims::default();
        let unescaped = unescape_text("a\\R\\b", &delims).unwrap();
        assert_eq!(unescaped, "a~b");
    }
    
    #[test]
    fn test_unescape_escape_character() {
        let delims = Delims::default();
        let unescaped = unescape_text("a\\E\\b", &delims).unwrap();
        assert_eq!(unescaped, "a\\b");
    }
    
    #[test]
    fn test_unescape_subcomponent_separator() {
        let delims = Delims::default();
        let unescaped = unescape_text("a\\T\\b", &delims).unwrap();
        assert_eq!(unescaped, "a&b");
    }
    
    #[test]
    fn test_unescape_multiple_sequences() {
        let delims = Delims::default();
        let unescaped = unescape_text("a\\F\\b\\S\\c\\R\\d\\E\\e\\T\\f", &delims).unwrap();
        assert_eq!(unescaped, "a|b^c~d\\e&f");
    }
    
    #[test]
    fn test_unescape_unknown_sequence() {
        let delims = Delims::default();
        // Unknown escape sequences are passed through
        let unescaped = unescape_text("a\\X\\b", &delims).unwrap();
        assert_eq!(unescaped, "a\\X\\b");
    }
    
    #[test]
    fn test_roundtrip() {
        let delims = Delims::default();
        let original = "test|value^with~special\\chars&here";
        let escaped = escape_text(original, &delims);
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        assert_eq!(unescaped, original);
    }
    
    #[test]
    fn test_needs_escaping() {
        let delims = Delims::default();
        assert!(needs_escaping("a|b", &delims));
        assert!(needs_escaping("a^b", &delims));
        assert!(needs_escaping("a~b", &delims));
        assert!(needs_escaping("a\\b", &delims));
        assert!(needs_escaping("a&b", &delims));
        assert!(!needs_escaping("normal text", &delims));
    }
    
    #[test]
    fn test_needs_unescaping() {
        let delims = Delims::default();
        assert!(needs_unescaping("a\\F\\b", &delims));
        assert!(!needs_unescaping("normal text", &delims));
    }
    
    #[test]
    fn test_custom_delimiters() {
        let delims = Delims {
            field: '#',
            comp: ':',
            rep: '*',
            esc: '@',
            sub: '%',
        };
        
        let escaped = escape_text("a#b:c", &delims);
        assert_eq!(escaped, "a@F@b@S@c");
        
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        assert_eq!(unescaped, "a#b:c");
    }
}