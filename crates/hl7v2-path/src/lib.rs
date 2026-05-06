//! Deprecated compatibility crate for HL7 v2 field path parsing.
//!
//! New Rust code should use `hl7v2::query::path` or the top-level
//! `hl7v2::{Path, parse_path}` exports instead.
//!
//! This crate is retained temporarily while implementation microcrates collapse
//! into modules under the canonical `hl7v2` crate. It does not define behavior
//! of its own.
//!
pub use hl7v2::query::path::*;
