//! Deterministic HL7 v2 message generator.
//!
//! This module provides functionality for generating synthetic HL7 v2
//! messages based on templates and profiles.
//!
//! # Template-Based Generation
//!
//! Template-based message generation functionality is available through
//! the [`crate::synthetic::template`] module and re-exported here for convenience.
//! See the [`crate::synthetic::template`] documentation for details on template
//! structure and value sources.
//!
//! # ACK Generation
//!
//! ACK (acknowledgment) generation functionality is available through
//! [`mod@crate::ack`] and re-exported here for convenience.
//!
//! # Faker Data Generation
//!
//! Realistic test data generation (names, addresses, medical codes, etc.)
//! is available through [`crate::synthetic::faker`] and re-exported here
//! for convenience.
//!
//! # Example
//!
//! ```
//! use hl7v2::synthetic::generate::{Template, generate, ack, AckCode, Faker, FakerValue};
//! ```

// Re-export template functionality for compatibility with the former hl7v2-gen facade.
pub use crate::synthetic::template::{
    Template, ValueSource, generate, generate_corpus, generate_distributed_corpus,
    generate_diverse_corpus, generate_golden_hashes, verify_golden_hashes,
};

// Re-export ACK functionality for compatibility with the former hl7v2-gen facade.
pub use crate::ack::{AckCode, ack, ack_with_error};

// Re-export faker functionality for compatibility with the former hl7v2-gen facade.
pub use crate::synthetic::faker::{DateError, Faker, FakerValue, GaussianError, GenerateError};

// Re-export core types that are commonly used with this crate
pub use crate::model::{Atom, Comp, Delims, Error, Field, Message, Rep, Segment};
