//! Shared test utilities for the hl7v2-rs workspace.
//!
//! This crate provides common test fixtures, builders, assertions, and utilities
//! used across all microcrates in the workspace. It is designed to be used as a
//! dev-dependency in other crates.
//!
//! # Modules
//!
//! - [`fixtures`] - Sample HL7 messages and test data
//! - [`builders`] - Fluent builders for constructing test messages
//! - [`assertions`] - HL7-specific assertion macros and functions
//! - [`mocks`] - Mock implementations for testing network code
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use hl7v2_test_utils::{fixtures, builders, assertions};
//!
//! // Load a sample message
//! let message = fixtures::SampleMessages::adt_a01();
//!
//! // Build a custom test message
//! let custom = builders::MessageBuilder::new()
//!     .with_msh("App", "Fac", "RecvApp", "RecvFac", "ADT", "A01")
//!     .with_pid("MRN123", "Doe", "John")
//!     .build();
//!
//! // Assert message validity
//! assertions::assert_message_valid(custom.as_bytes());
//! ```

pub mod fixtures;
pub mod builders;
pub mod assertions;
pub mod mocks;

// Re-exports for convenience
pub use fixtures::SampleMessages;
pub use builders::{MessageBuilder, SegmentBuilder};
pub use assertions::{
    assert_message_valid,
    assert_segment_equals,
    assert_field_equals,
    assert_parse_fails,
    assert_hl7_roundtrips,
};
pub use mocks::{MockMllpServer, MockMessageHandler};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::fixtures::SampleMessages;
    pub use crate::builders::{MessageBuilder, SegmentBuilder};
    pub use crate::assertions::{
        assert_message_valid,
        assert_segment_equals,
        assert_field_equals,
        assert_parse_fails,
        assert_hl7_roundtrips,
    };
    pub use crate::mocks::{MockMllpServer, MockMessageHandler};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_messages_load() {
        let adt_a01 = SampleMessages::adt_a01();
        assert!(adt_a01.contains("ADT^A01"));
    }

    #[test]
    fn test_message_builder_creates_valid_message() {
        let bytes = MessageBuilder::new()
            .with_msh("TestApp", "TestFac", "RecvApp", "RecvFac", "ADT", "A01")
            .build_bytes();
        
        assert_message_valid(&bytes);
    }
}
