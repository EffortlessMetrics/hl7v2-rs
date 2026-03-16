//! # hl7v2
//!
//! HL7 v2 message parser and processor for Rust.
//!
//! This crate is the canonical entry point for the
//! [`hl7v2-rs`](https://github.com/EffortlessMetrics/hl7v2-rs) workspace.
//! It re-exports everything from [`hl7v2-core`](https://crates.io/crates/hl7v2-core).
//!
//! ## Quick start
//!
//! ```rust
//! use hl7v2::{parse, get};
//!
//! let msg = parse(b"MSH|^~\\&|App||Fac||20250128||ADT^A01|123|P|2.5.1\rPID|1||PAT123||Doe^John\r").unwrap();
//! assert_eq!(get(&msg, "PID.5.1"), Some("Doe"));
//! ```
//!
//! ## Features
//!
//! - `stream` — streaming/event-based parser
//! - `network` — async MLLP client/server

pub use hl7v2_core::*;
