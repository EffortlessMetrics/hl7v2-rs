//! Deprecated compatibility facade.
//!
//! Use the `hl7v2` crate for new Rust code. `hl7v2-core` is retained
//! temporarily as a compatibility shim and must not define independent
//! behavior.

#![deprecated(note = "Use the `hl7v2` crate instead.")]

pub use hl7v2::*;

#[cfg(feature = "network")]
pub mod network {
    //! Compatibility alias for the old `hl7v2_core::network` path.

    pub use hl7v2::transport::network::*;
}
