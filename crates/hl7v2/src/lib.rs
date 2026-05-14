//! # hl7v2
//!
//! Canonical Rust API for HL7 v2 parsing, writing, validation, transport,
//! acknowledgement, normalization, and generation.
//!
//! The implementation lives in this crate as focused modules. Public Rust
//! consumers should depend on this crate and import through `hl7v2`.
//!
//! ## Quick start
//!
//! ```rust
//! use hl7v2::{get, parse};
//!
//! let msg = parse(b"MSH|^~\\&|App||Fac||20250128||ADT^A01|123|P|2.5.1\rPID|1||PAT123||Doe^John\r").unwrap();
//! assert_eq!(get(&msg, "PID.5.1"), Some("Doe"));
//! ```
//!
//! ## Features
//!
//! - `json` - JSON serialization helpers.
//! - `profile` - profile loading and conformance validation.
//! - `ack` - ACK message generation.
//! - `normalize` - message normalization.
//! - `batch` - batch parsing and writing helpers.
//! - `stream` - streaming/event-based parser.
//! - `network` - async MLLP client/server.
//! - `synthetic` - template, faker, corpus, and generation APIs.
//! - `redact` - redaction helpers.
//! - `lifecycle` - lifecycle and archive metadata helpers.
//! - `experimental-guard` - experimental guard/anomaly detection APIs.

pub mod model;

pub mod escape;

pub mod transport;

pub mod parser;

pub mod writer;

pub mod query;

#[cfg(any(feature = "profile", feature = "profile-core"))]
pub mod conformance {
    //! Profile, datatype, and validation APIs.

    pub mod datatype;
    pub mod profile;
    pub mod validation;
}

#[cfg(feature = "ack")]
pub mod ack;

#[cfg(feature = "normalize")]
pub mod normalize;

#[cfg(feature = "batch")]
pub mod batch;

#[cfg(feature = "stream")]
pub mod stream;

#[cfg(feature = "synthetic")]
pub mod synthetic;

#[cfg(feature = "redact")]
pub mod redact;

#[cfg(all(feature = "profile", feature = "redact"))]
pub mod evidence;

#[cfg(feature = "lifecycle")]
pub mod lifecycle;

#[cfg(feature = "experimental-guard")]
pub mod experimental;

// Top-level convenience surface.
pub use escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};
pub use model::{
    Atom, Batch, Comp, Delims, Error, Field, FileBatch, Message, Presence, Rep, Segment,
};
pub use parser::{parse, parse_batch, parse_file_batch, parse_mllp};
pub use query::path::{Path, parse_path};
pub use query::{get, get_presence};
pub use transport::mllp::{
    MLLP_END_1, MLLP_END_2, MLLP_START, MllpFrameIterator, find_complete_mllp_message,
    is_mllp_framed, unwrap_mllp, unwrap_mllp_owned, wrap_mllp,
};
pub use writer::{
    to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch,
    write_mllp,
};

#[cfg(feature = "normalize")]
pub use normalize::normalize;

#[cfg(feature = "ack")]
pub use ack::{AckCode, ack, ack_with_error};

#[cfg(feature = "profile")]
pub use conformance::profile::{
    ExpectedReportComparison, Profile, ProfileExplainConstraint, ProfileExplainDatatypeRule,
    ProfileExplainExpressionGuardrails, ProfileExplainLengthRule, ProfileExplainLintSummary,
    ProfileExplainReport, ProfileExplainReportV2, ProfileExplainRequiredField, ProfileExplainRule,
    ProfileExplainRules, ProfileExplainSegment, ProfileExplainSummary, ProfileExplainTable,
    ProfileExplainValueSet, ProfileFixtureExpectation, ProfileLintIssue, ProfileLintReport,
    ProfileLintReportV2, ProfileLintSeverity, ProfileTestCaseReport, ProfileTestReport,
    ProfileTestReportV2, explain_profile, lint_profile_yaml, load_profile, load_profile_checked,
    run_profile_fixture_tests, validate,
};

#[cfg(feature = "profile")]
pub use conformance::validation::{
    Issue, Severity, ValidationReport, ValidationReportIssue, ValidationReportProfileIdentity,
    ValidationReportSeverity, ValidationReportV2,
};

#[cfg(feature = "stream")]
pub use stream::{AsyncStreamParser, Event, StreamParser, StreamParserBuilder};
