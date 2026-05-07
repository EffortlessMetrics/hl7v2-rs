//! Deprecated compatibility crate for HL7 v2 MLLP framing.
//!
//! New code should import these APIs from `hl7v2::transport::mllp`.

pub use hl7v2::transport::mllp::{
    MLLP_END_1, MLLP_END_2, MLLP_START, MllpError, MllpFrameIterator, find_complete_mllp_message,
    is_mllp_framed, unwrap_mllp, unwrap_mllp_checked, unwrap_mllp_owned, unwrap_mllp_owned_checked,
    wrap_mllp,
};
