//! Profile validation for HL7 v2 messages.
//!
//! This module provides functionality for loading and applying
//! conformance profiles to HL7 v2 messages. It builds on
//! `hl7v2::conformance::validation` for core validation logic.

#![expect(
    clippy::collapsible_if,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::manual_let_else,
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::string_slice,
    clippy::uninlined_format_args,
    reason = "Pre-existing profile implementation debt moved from hl7v2-prof; cleanup is separate from this behavior-preserving module collapse."
)]
//!
//! # Features
//!
//! - Profile loading from YAML
//! - Profile inheritance and merging
//! - Profile-based message validation
//! - Cross-field validation rules
//! - Temporal validation rules
//! - Contextual validation rules
//!
//! # Example
//!
//! ```ignore
//! use hl7v2::{load_profile, validate, Profile};
//!
//! let yaml = r#"
//! message_structure: ADT_A01
//! version: "2.5.1"
//! segments:
//!   - id: MSH
//! constraints:
//!   - path: MSH.9
//!     required: true
//! "#;
//!
//! let profile = load_profile(yaml)?;
//! let issues = validate(&message, &profile);
//! ```

// Re-export validation types for compatibility with the old profile facade.
pub use super::validation::{
    Issue, ParsedTimestamp, RuleAction, RuleCondition, Severity, TimestampPrecision,
    ValidationResult, Validator, check_rule_condition, compare_timestamps_for_before, get_nonempty,
    is_coded_value, is_date, is_email, is_extended_id, is_formatted_text, is_hierarchic_designator,
    is_identifier, is_numeric, is_person_name, is_phone_number, is_sequence_id, is_ssn, is_string,
    is_text_data, is_time, is_timestamp, is_valid_age_range, is_valid_birth_date, is_within_range,
    matches_complex_pattern, matches_format, parse_datetime, parse_hl7_ts,
    parse_hl7_ts_with_precision, truncate_to_precision, validate_checksum, validate_data_type,
    validate_luhn_checksum, validate_mathematical_relationship, validate_mod10_checksum,
};

use crate::model::{Error, Message};
use crate::parser::parse;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod fixture_tests;
mod lint;
pub mod loader;
mod loading;
#[cfg(feature = "persistent-cache")]
pub mod persistent_cache;
mod types;
mod validator;

pub use fixture_tests::run_profile_fixture_tests;
pub use lint::{explain_profile, lint_profile_yaml};
pub use loading::{
    load_profile, load_profile_checked, load_profile_from_file, load_profile_with_inheritance,
};
pub use types::*;
pub use validator::validate;

#[cfg(test)]
mod tests;
