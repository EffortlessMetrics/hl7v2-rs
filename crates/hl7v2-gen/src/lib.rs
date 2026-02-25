//! Deterministic HL7 v2 message generator.
//!
//! This crate provides functionality for generating synthetic HL7 v2
//! messages based on templates and profiles.
//!
//! # Template-Based Generation
//!
//! Template-based message generation functionality is available through
//! the [`hl7v2_template`] crate and re-exported here for convenience.
//! See the [`hl7v2_template`] documentation for details on template
//! structure and value sources.
//!
//! # ACK Generation
//!
//! ACK (acknowledgment) generation functionality is available through
//! the [`hl7v2_ack`] crate and re-exported here for convenience.
//!
//! # Faker Data Generation
//!
//! Realistic test data generation (names, addresses, medical codes, etc.)
//! is available through the [`hl7v2_faker`] crate and re-exported here
//! for convenience.
//!
//! # Example
//!
//! ```
//! use hl7v2_gen::{Template, generate, ack, AckCode, Faker, FakerValue};
//! ```

// Re-export template functionality from hl7v2-template crate for backward compatibility
pub use hl7v2_template::{
    Template, ValueSource,
    generate, generate_corpus, generate_diverse_corpus, generate_distributed_corpus,
    generate_golden_hashes, verify_golden_hashes,
};

// Re-export ACK functionality from hl7v2-ack crate for backward compatibility
pub use hl7v2_ack::{ack, ack_with_error, AckCode};

// Re-export faker functionality from hl7v2-faker crate for backward compatibility
pub use hl7v2_faker::{
    Faker, FakerValue, DateError, GaussianError, GenerateError,
};

// Re-export core types that are commonly used with this crate
pub use hl7v2_core::{Message, Delims, Error, Segment, Field, Rep, Comp, Atom};

#[cfg(test)]
mod tests;
