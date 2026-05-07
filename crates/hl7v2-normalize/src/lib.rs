//! Deprecated compatibility crate.
//!
//! Use `hl7v2::normalize` instead.

pub use hl7v2::normalize::normalize;

#[cfg(test)]
mod tests;
