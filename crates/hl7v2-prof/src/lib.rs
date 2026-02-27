//! Profile validation for HL7 v2 messages.
//!
//! This crate provides functionality for loading and applying
//! conformance profiles to HL7 v2 messages.

mod load;
mod merge;
mod model;
mod validate;

pub use load::{load_profile, load_profile_with_inheritance};
pub use model::*;
pub use validate::validate;

#[cfg(test)]
mod tests;
