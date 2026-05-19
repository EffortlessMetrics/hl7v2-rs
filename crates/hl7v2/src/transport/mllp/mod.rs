//! MLLP (Minimal Lower Layer Protocol) framing for HL7 v2.
//!
//! This module provides functions for wrapping and unwrapping HL7 v2 messages
//! with MLLP framing, as defined in the HL7 v2 specification.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::missing_errors_doc,
    reason = "pre-existing MLLP implementation debt moved from staged microcrate into hl7v2; cleanup is split from topology collapse"
)]

mod constants;
mod errors;
mod framing;
mod streaming;

pub use constants::{MLLP_END_1, MLLP_END_2, MLLP_START};
pub use errors::MllpError;
pub use framing::{
    find_complete_mllp_message, is_mllp_framed, unwrap_mllp, unwrap_mllp_checked,
    unwrap_mllp_owned, unwrap_mllp_owned_checked, wrap_mllp,
};
pub use streaming::MllpFrameIterator;

#[cfg(test)]
mod tests;
