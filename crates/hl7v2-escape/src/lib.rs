//! Deprecated compatibility crate for HL7 v2 escape sequence handling.
//!
//! New code should import these APIs from `hl7v2::escape`.

pub use hl7v2::escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};
