//! # hl7v2
//!
//! Canonical Rust API for HL7 v2 parsing, writing, validation, transport,
//! acknowledgement, normalization, and generation.
//!
//! The implementation still lives in focused workspace crates during the
//! crate-surface migration, but public Rust consumers should depend on this
//! crate and import through `hl7v2`.
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

pub mod model {
    //! Core HL7 v2 data structures.

    pub use hl7v2_model::*;
}

pub mod escape {
    //! HL7 escape sequence handling.

    pub use hl7v2_escape::*;
}

pub mod transport {
    //! HL7 v2 transport helpers.

    pub mod mllp {
        //! Minimal Lower Layer Protocol framing helpers.

        pub use hl7v2_mllp::*;
    }

    #[cfg(feature = "network")]
    pub mod network {
        //! Async MLLP network client/server APIs.

        pub use hl7v2_network::*;
    }
}

pub mod parser {
    //! HL7 v2 parser APIs.

    pub use hl7v2_parser::*;
}

pub mod writer {
    //! HL7 v2 writer and serialization APIs.

    pub use hl7v2_writer::*;

    #[cfg(feature = "json")]
    pub mod json {
        //! JSON serialization helpers.

        pub use hl7v2_json::*;
    }
}

pub mod query {
    //! Message query helpers.

    pub mod path;

    pub use hl7v2_query::*;
}

#[cfg(feature = "profile")]
pub mod conformance {
    //! Profile, datatype, and validation APIs.

    pub mod profile {
        //! Profile loading and conformance validation.

        pub use hl7v2_prof::*;
    }

    pub mod validation {
        //! Reusable validation issue and rule primitives.

        pub use hl7v2_validation::*;
    }

    pub mod datatype {
        //! HL7 datatype validation.

        pub mod datetime {
            //! HL7 datetime parsing and validation.

            pub use hl7v2_datetime::*;
        }

        pub use hl7v2_datatype::*;
    }
}

#[cfg(feature = "ack")]
pub mod ack {
    //! ACK message generation.

    pub use hl7v2_ack::*;
}

#[cfg(feature = "normalize")]
pub mod normalize {
    //! Message normalization.

    pub use hl7v2_normalize::*;
}

#[cfg(feature = "batch")]
pub mod batch {
    //! Batch processing APIs.

    pub use hl7v2_batch::*;
}

#[cfg(feature = "stream")]
pub mod stream {
    //! Streaming/event-based parsing APIs.

    pub use hl7v2_stream::*;
}

#[cfg(feature = "synthetic")]
pub mod synthetic {
    //! Synthetic message generation APIs.

    pub mod template {
        //! Template-based message generation.

        pub use hl7v2_template::*;
    }

    pub mod values {
        //! Template value sources.

        pub use hl7v2_template_values::*;
    }

    pub mod faker {
        //! Faker-backed synthetic value generation.

        pub use hl7v2_faker::*;
    }

    pub mod corpus {
        //! Corpus metadata and hashing helpers.

        pub use hl7v2_corpus::*;
    }

    pub mod generate {
        //! Generation convenience facade.

        pub use hl7v2_gen::*;
    }
}

#[cfg(feature = "redact")]
pub mod redact;

#[cfg(feature = "lifecycle")]
pub mod lifecycle {
    //! Message lifecycle and archive metadata helpers.

    pub use hl7v2_lifecycle::*;
}

#[cfg(feature = "experimental-guard")]
pub mod experimental;

// Top-level convenience surface.
pub use hl7v2_escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};
pub use hl7v2_mllp::{
    MLLP_END_1, MLLP_END_2, MLLP_START, MllpFrameIterator, find_complete_mllp_message,
    is_mllp_framed, unwrap_mllp, unwrap_mllp_owned, wrap_mllp,
};
pub use hl7v2_model::{
    Atom, Batch, Comp, Delims, Error, Field, FileBatch, Message, Presence, Rep, Segment,
};
pub use hl7v2_parser::{get, get_presence, parse, parse_batch, parse_file_batch, parse_mllp};
pub use hl7v2_writer::{
    to_json, to_json_string, to_json_string_pretty, write, write_batch, write_file_batch,
    write_mllp,
};
pub use query::path::{Path, parse_path};

#[cfg(feature = "normalize")]
pub use hl7v2_normalize::normalize;

#[cfg(feature = "ack")]
pub use hl7v2_ack::{AckCode, ack, ack_with_error};

#[cfg(feature = "profile")]
pub use hl7v2_prof::{Profile, load_profile, load_profile_checked, validate};

#[cfg(feature = "profile")]
pub use hl7v2_validation::{Issue, Severity};

#[cfg(feature = "stream")]
pub use hl7v2_stream::{AsyncStreamParser, Event, StreamParser, StreamParserBuilder};
