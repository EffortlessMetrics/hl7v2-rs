//! Deprecated compatibility crate.
//!
//! Use `hl7v2::conformance::profile` instead.

pub use hl7v2::conformance::profile::*;
pub use hl7v2::conformance::validation::*;

#[cfg(test)]
mod tests;
