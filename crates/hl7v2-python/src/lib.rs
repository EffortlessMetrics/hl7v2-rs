//! Python bindings for HL7v2 via PyO3.

use ::hl7v2::{
    Message, ValidationReport, load_profile_checked, normalize as rust_normalize,
    parse as rust_parse, to_json as rust_to_json, validate as rust_validate,
};
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

/// Stable validation report wrapper for Python.
#[pyclass]
pub struct PyValidationReport {
    inner: ValidationReport,
}

#[pymethods]
impl PyValidationReport {
    /// Whether the message passed validation without error-level issues.
    #[getter]
    pub fn valid(&self) -> bool {
        self.inner.valid
    }

    /// HL7 trigger event from `MSH.9`, such as `ADT^A01`.
    #[getter]
    pub fn message_type(&self) -> String {
        self.inner.message_type.clone()
    }

    /// Profile identifier included in the report.
    #[getter]
    pub fn profile(&self) -> Option<String> {
        self.inner.profile.clone()
    }

    /// Number of parsed message segments.
    #[getter]
    pub fn segment_count(&self) -> usize {
        self.inner.segment_count
    }

    /// Number of validation issues in the report.
    #[getter]
    pub fn issue_count(&self) -> usize {
        self.inner.issue_count
    }

    /// Convert the validation report to a JSON string.
    pub fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Report serialization error: {e}")))
    }

    /// Convert the validation report to a Python dict.
    pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let json = self.to_json()?;
        py.import("json")?.call_method1("loads", (json,))
    }
}

/// Parse an HL7 message from a string.
#[pyfunction]
pub fn parse(content: &str) -> PyResult<PyMessage> {
    PyMessage::parse(content)
}

/// Parse an HL7 message and convert it to a JSON string.
#[pyfunction]
pub fn to_json(content: &str) -> PyResult<String> {
    let message = rust_parse(content.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("Parse error: {e}")))?;
    Ok(rust_to_json(&message).to_string())
}

/// Normalize an HL7 message.
#[pyfunction(signature = (content, canonical_delims = false))]
pub fn normalize(content: &str, canonical_delims: bool) -> PyResult<String> {
    let normalized = rust_normalize(content.as_bytes(), canonical_delims)
        .map_err(|e| PyValueError::new_err(format!("Normalize error: {e}")))?;
    String::from_utf8(normalized)
        .map_err(|e| PyValueError::new_err(format!("Normalize UTF-8 error: {e}")))
}

/// Validate an HL7 message against a profile YAML string.
#[pyfunction]
pub fn validate(content: &str, profile_yaml: &str) -> PyResult<PyValidationReport> {
    let message = rust_parse(content.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("Parse error: {e}")))?;
    let profile = load_profile_checked(profile_yaml)
        .map_err(|e| PyValueError::new_err(format!("Profile load error: {e}")))?;
    let issues = rust_validate(&message, &profile);
    Ok(PyValidationReport {
        inner: ValidationReport::from_issues(
            &message,
            Some(profile.message_structure.clone()),
            issues,
        ),
    })
}

/// HL7v2 module for Python
#[pymodule]
fn hl7v2(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<PyMessage>()?;
    m.add_class::<PyValidationReport>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(to_json, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    Ok(())
}
