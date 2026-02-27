//! Core parsing and data model for HL7 v2 messages.
//!
//! This crate provides the foundational data structures and parsing logic
//! for HL7 v2 messages, including:
//! - Message parsing from raw bytes
//! - Data model representation (Message, Segment, Field, etc.)
//! - Escape sequence handling
//! - JSON serialization
//! - Batch message handling (FHS/BHS/BTS/FTS)

#[cfg(feature = "network")]
pub mod network;

mod delims;
mod error;
mod event;
mod json;
mod model;
mod normalize;
mod parse;
mod path;
mod stream;
mod write;

pub use delims::Delims;
pub use error::Error;
pub use event::Event;
pub use json::to_json;
pub use model::{Atom, Batch, Comp, Field, FileBatch, Message, Presence, Rep, Segment};
pub use normalize::{normalize, normalize_batch, normalize_file_batch};
pub use parse::{parse, parse_batch, parse_file_batch, parse_mllp, unescape_text};
pub(crate) use parse::extract_charsets;
pub use path::{get, get_presence};
pub use stream::StreamParser;
pub use write::{escape_text, wrap_mllp, write, write_batch, write_file_batch, write_mllp};

#[cfg(test)]
mod tests;
