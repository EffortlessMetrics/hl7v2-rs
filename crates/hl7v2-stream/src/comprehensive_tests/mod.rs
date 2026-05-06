//! Comprehensive tests for the hl7v2-stream crate.
//!
//! This module contains unit tests and property-based tests for the streaming
//! HL7 v2 parser.

mod property_tests;
mod unit_tests;

// Re-export for convenience (unused but kept for potential future use)
#[expect(
    unused_imports,
    reason = "tracked by the workspace lint policy rollout"
)]
pub use property_tests::*;
#[expect(
    unused_imports,
    reason = "tracked by the workspace lint policy rollout"
)]
pub use unit_tests::*;
