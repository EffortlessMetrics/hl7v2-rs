//! Deterministic HL7 v2 message generator.
//!
//! This crate provides functionality for generating synthetic HL7 v2
//! messages based on templates and profiles.

mod ack;
mod corpus;
mod delimiters;
mod generate;
mod hash;
mod template;

pub use ack::{ack, AckCode};
pub use corpus::{generate_corpus, generate_distributed_corpus, generate_diverse_corpus};
pub use generate::generate;
pub use hash::{generate_golden_hashes, verify_golden_hashes};
pub use template::{Template, ValueSource};

#[cfg(test)]
mod tests;
