//! Comprehensive tests for the hl7v2-stream crate.
//!
//! This module contains unit tests and property-based tests for the streaming
//! HL7 v2 parser.

mod property_tests;
mod unit_tests;

// Re-export for convenience (unused but kept for potential future use)
#[allow(unused_imports)]
pub use property_tests::*;
#[allow(unused_imports)]
pub use unit_tests::*;
