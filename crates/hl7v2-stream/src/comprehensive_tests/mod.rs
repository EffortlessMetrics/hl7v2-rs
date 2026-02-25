//! Comprehensive tests for the hl7v2-stream crate.
//!
//! This module contains unit tests and property-based tests for the streaming
//! HL7 v2 parser.

mod unit_tests;
mod property_tests;

// Re-export for convenience
pub use unit_tests::*;
pub use property_tests::*;
