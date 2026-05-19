//! Request and response models for the HTTP API.
//!
//! Models are organized into SRP-oriented submodules:
//! - `general`: shared API request/response shapes
//! - `ack`: ACK generation and policy models

mod ack;
mod general;

pub use ack::*;
pub use general::*;
