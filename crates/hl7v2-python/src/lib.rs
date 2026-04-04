//! Python bindings for the hl7v2-rs library.
//!
//! This crate provides Python bindings via PyO3 for parsing, validating,
//! normalizing, and generating HL7 v2 messages.
//!
//! # Python API
//!
//! ```python
//! import hl7v2
//!
//! # Parse an HL7 message
//! message = hl7v2.parse("MSH|^~\\&|SendingApp|SendingFac...")
//!
//! # Validate the message
//! is_valid = hl7v2.validate(message, "2.5.1")
//!
//! # Normalize the message
//! normalized = hl7v2.normalize(message)
//!
//! # Generate HL7 string from message
//! hl7_string = hl7v2.generate(message)
//! ```

#![allow(unsafe_op_in_unsafe_fn)] // PyO3 proc-macro compatibility with Rust 2024 edition

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Python wrapper for HL7v2 Message
#[pyclass(name = "Message")]
#[derive(Clone)]
pub struct PyMessage {
    inner: hl7v2_model::Message,
}

#[pymethods]
impl PyMessage {
    /// Get the HL7 version from MSH-12
    fn version(&self) -> Option<String> {
        hl7v2_parser::get(&self.inner, "MSH.12").map(|s| s.to_string())
    }

    /// Get the message type from MSH-9
    fn message_type(&self) -> Option<String> {
        hl7v2_parser::get(&self.inner, "MSH.9.1").map(|s| s.to_string())
    }

    /// Get the trigger event from MSH-9.2
    fn trigger_event(&self) -> Option<String> {
        hl7v2_parser::get(&self.inner, "MSH.9.2").map(|s| s.to_string())
    }

    /// Get a field value by path (e.g., "PID.5.1")
    fn get(&self, path: &str) -> Option<String> {
        hl7v2_parser::get(&self.inner, path).map(|s| s.to_string())
    }

    /// Get the number of segments in the message
    fn segment_count(&self) -> usize {
        self.inner.segments.len()
    }

    /// Get all segment IDs in the message
    fn segment_ids(&self) -> Vec<String> {
        self.inner
            .segments
            .iter()
            .map(|s| String::from_utf8_lossy(&s.id).to_string())
            .collect()
    }

    /// Convert the message to a JSON string
    fn to_json(&self) -> String {
        hl7v2_writer::to_json_string(&self.inner)
    }

    /// Get string representation of the message (as HL7)
    fn __repr__(&self) -> String {
        format!(
            "<Message type={:?} version={:?} segments={}>",
            self.message_type(),
            self.version(),
            self.segment_count()
        )
    }

    /// String representation for Python
    fn __str__(&self) -> String {
        String::from_utf8_lossy(&hl7v2_writer::write(&self.inner)).to_string()
    }
}

/// Parse an HL7 v2 message from a string.
///
/// # Arguments
/// * `message` - The HL7 message string to parse
///
/// # Returns
/// A `Message` object representing the parsed HL7 message.
///
/// # Raises
/// * `ValueError` - If the message cannot be parsed
///
/// # Example
/// ```python
/// import hl7v2
/// message = hl7v2.parse("MSH|^~\\&|SendingApp|SendingFac|...")
/// ```
#[pyfunction]
fn parse(message: &str) -> PyResult<PyMessage> {
    let inner = hl7v2_parser::parse(message.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("Failed to parse HL7 message: {}", e)))?;
    Ok(PyMessage { inner })
}

/// Validate an HL7 v2 message against a version specification.
///
/// # Arguments
/// * `message` - The Message object to validate
/// * `version` - The HL7 version to validate against (e.g., "2.5.1")
///
/// # Returns
/// `True` if the message is valid, `False` otherwise.
///
/// # Example
/// ```python
/// import hl7v2
/// message = hl7v2.parse("MSH|^~\\&|...")
/// is_valid = hl7v2.validate(message, "2.5.1")
/// ```
#[pyfunction]
fn validate(message: &PyMessage, version: &str) -> PyResult<bool> {
    // Check version matches MSH-12 if present
    if let Some(msg_version) = hl7v2_parser::get(&message.inner, "MSH.12")
        && msg_version != version
    {
        return Ok(false);
    }

    // Validate that the message has required MSH fields
    let required_msh_fields = [
        "MSH.3", "MSH.4", "MSH.5", "MSH.6", "MSH.7", "MSH.9", "MSH.10", "MSH.11", "MSH.12",
    ];
    for field in required_msh_fields {
        if hl7v2_parser::get(&message.inner, field).is_none() {
            return Ok(false);
        }
    }

    // Check that MSH-9 (message type) is valid
    if let Some(msg_type) = hl7v2_parser::get(&message.inner, "MSH.9.1")
        && msg_type.is_empty()
    {
        return Ok(false);
    }

    Ok(true)
}

/// Normalize an HL7 v2 message.
///
/// This parses the message and rewrites it with canonical delimiters (^~\&).
///
/// # Arguments
/// * `message` - The Message object to normalize
///
/// # Returns
/// A new normalized `Message` object.
///
/// # Raises
/// * `RuntimeError` - If normalization fails
///
/// # Example
/// ```python
/// import hl7v2
/// message = hl7v2.parse("MSH|^~\\&|...")
/// normalized = hl7v2.normalize(message)
/// ```
#[pyfunction]
fn normalize(message: &PyMessage) -> PyResult<PyMessage> {
    let normalized_bytes = hl7v2_normalize::normalize(
        &hl7v2_writer::write(&message.inner),
        true, // Use canonical delimiters
    )
    .map_err(|e| PyRuntimeError::new_err(format!("Failed to normalize message: {}", e)))?;

    let inner = hl7v2_parser::parse(&normalized_bytes).map_err(|e| {
        PyRuntimeError::new_err(format!("Failed to parse normalized message: {}", e))
    })?;

    Ok(PyMessage { inner })
}

/// Generate an HL7 v2 message string from a Message object.
///
/// # Arguments
/// * `message` - The Message object to serialize
///
/// # Returns
/// The HL7 message as a string.
///
/// # Example
/// ```python
/// import hl7v2
/// message = hl7v2.parse("MSH|^~\\&|...")
/// hl7_string = hl7v2.generate(message)
/// ```
#[pyfunction]
fn generate(message: &PyMessage) -> String {
    String::from_utf8_lossy(&hl7v2_writer::write(&message.inner)).to_string()
}

/// Parse an HL7 batch message.
///
/// # Arguments
/// * `batch` - The batch message string to parse
///
/// # Returns
/// A list of `Message` objects.
///
/// # Raises
/// * `ValueError` - If the batch cannot be parsed
///
/// # Example
/// ```python
/// import hl7v2
/// messages = hl7v2.parse_batch("BHS|^~\\&|...")
/// ```
#[pyfunction]
fn parse_batch(batch: &str) -> PyResult<Vec<PyMessage>> {
    let batch_result = hl7v2_parser::parse_batch(batch.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("Failed to parse HL7 batch: {}", e)))?;

    let messages: Vec<PyMessage> = batch_result
        .messages
        .into_iter()
        .map(|inner| PyMessage { inner })
        .collect();

    Ok(messages)
}

/// Information about the hl7v2-python library.
///
/// Returns a dictionary with version information.
#[pyfunction]
fn info(py: Python<'_>) -> PyResult<PyObject> {
    let info = [
        ("name", "hl7v2"),
        ("version", env!("CARGO_PKG_VERSION")),
        ("rust_version", env!("CARGO_PKG_RUST_VERSION")),
        ("edition", "2024"),
    ];
    let dict = pyo3::types::PyDict::new_bound(py);
    for (key, value) in &info {
        dict.set_item(*key, *value)?;
    }
    Ok(dict.into())
}

/// hl7v2 - Python bindings for HL7 v2 message processing
///
/// This module provides Python bindings for the hl7v2-rs library,
/// enabling parsing, validation, normalization, and generation of
/// HL7 v2 messages.
///
/// Core Functions:
/// - parse: Parse an HL7 message from a string
/// - validate: Validate a message against a version specification
/// - normalize: Normalize a message with canonical delimiters
/// - generate: Generate an HL7 string from a Message object
/// - parse_batch: Parse an HL7 batch containing multiple messages
/// - info: Get library information
///
/// Classes:
/// - Message: Represents a parsed HL7 message with methods for
///   accessing fields, segments, and converting to various formats.
#[pymodule]
fn hl7v2(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMessage>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(generate, m)?)?;
    m.add_function(wrap_pyfunction!(parse_batch, m)?)?;
    m.add_function(wrap_pyfunction!(info, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[cfg(test)]
mod tests;
