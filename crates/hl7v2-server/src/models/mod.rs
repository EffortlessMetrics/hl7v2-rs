//! Request and response models for the HTTP API.
//!
//! These models follow JSON:API conventions where appropriate and align
//! with the OpenAPI specification in `api/openapi/hl7v2-api-v1.yaml`.

mod ack_and_errors;
mod core;
mod evidence;

pub use ack_and_errors::*;
pub use core::*;
pub use evidence::*;
