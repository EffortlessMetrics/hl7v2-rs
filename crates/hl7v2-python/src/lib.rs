//! Python bindings for HL7v2 via PyO3.

use hl7v2_core::{parse as rust_parse, to_json as rust_to_json, Message};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// HL7v2 Message wrapper for Python
#[pyclass]
pub struct PyMessage {
    inner: Message,
}

#[pymethods]
impl PyMessage {
    /// Parse an HL7 message from string
    #[staticmethod]
    pub fn parse(content: &str) -> PyResult<Self> {
        let message = rust_parse(content.as_bytes())
            .map_err(|e| PyValueError::new_err(format!("Parse error: {}", e)))?;
        Ok(PyMessage { inner: message })
    }

    /// Convert message to JSON string
    pub fn to_json(&self) -> PyResult<String> {
        let json = rust_to_json(&self.inner);
        Ok(json.to_string())
    }

    /// Get segment count
    pub fn segment_count(&self) -> usize {
        self.inner.segments.len()
    }
}

/// HL7v2 module for Python
#[pymodule]
fn hl7v2(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMessage>()?;
    Ok(())
}
