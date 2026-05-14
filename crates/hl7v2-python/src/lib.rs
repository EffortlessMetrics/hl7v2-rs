//! Python bindings for HL7v2 via PyO3.

use ::hl7v2::evidence::{
    replay_evidence_bundle as rust_replay_evidence_bundle,
    write_safe_analysis_bundle_with_schema_version as rust_write_safe_analysis_bundle,
};
use ::hl7v2::redact::redact_hl7_safe_analysis as rust_redact_hl7_safe_analysis;
use ::hl7v2::synthetic::corpus::{
    CorpusCount, CorpusFingerprintProfile, compute_sha256, diff_corpus_fingerprints,
    diff_corpus_paths, fingerprint_corpus_path, summarize_corpus_path,
};
use ::hl7v2::synthetic::generate::{Template as RustTemplate, generate as rust_generate};
use ::hl7v2::{
    AckCode as RustAckCode, Message, ProfileTestReport, ValidationReport,
    ValidationReportProfileIdentity, ack as rust_ack, explain_profile as rust_explain_profile,
    is_mllp_framed, lint_profile_yaml as rust_lint_profile_yaml, load_profile_checked,
    normalize as rust_normalize, parse as rust_parse, parse_mllp as rust_parse_mllp,
    run_profile_fixture_tests as rust_run_profile_fixture_tests, to_json as rust_to_json,
    validate as rust_validate, write as rust_write,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    profile_identity: Option<ValidationReportProfileIdentity>,
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
    #[pyo3(signature = (schema_version = 1))]
    pub fn to_json(&self, schema_version: u8) -> PyResult<String> {
        let report = self.report_for_schema_version(schema_version)?;
        serde_json::to_string(&report)
            .map_err(|e| PyValueError::new_err(format!("Report serialization error: {e}")))
    }

    /// Convert the validation report to a Python dict.
    #[pyo3(signature = (schema_version = 1))]
    pub fn to_dict<'py>(&self, py: Python<'py>, schema_version: u8) -> PyResult<Bound<'py, PyAny>> {
        let json = self.to_json(schema_version)?;
        py.import("json")?.call_method1("loads", (json,))
    }
}

impl PyValidationReport {
    fn report_for_schema_version(&self, schema_version: u8) -> PyResult<serde_json::Value> {
        match schema_version {
            1 => serde_json::to_value(&self.inner)
                .map_err(|e| PyValueError::new_err(format!("Report serialization error: {e}"))),
            2 => {
                let report = self.inner.to_v2(
                    "hl7v2-python",
                    env!("CARGO_PKG_VERSION"),
                    self.profile_identity.clone(),
                );
                serde_json::to_value(report)
                    .map_err(|e| PyValueError::new_err(format!("Report serialization error: {e}")))
            }
            _ => Err(PyValueError::new_err(
                "validation report schema_version must be 1 or 2",
            )),
        }
    }
}

fn value_error(context: &str, error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(format!("{context}: {error}"))
}

fn py_json_loads<'py>(py: Python<'py>, json: String) -> PyResult<Bound<'py, PyAny>> {
    py.import("json")?.call_method1("loads", (json,))
}

fn report_to_dict<'py>(
    py: Python<'py>,
    report: &impl Serialize,
    context: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let json = serde_json::to_string(report).map_err(|e| value_error(context, e))?;
    py_json_loads(py, json)
}

fn profile_issue_counts_for_path(
    path: &Path,
    profile_yaml: &str,
) -> PyResult<(CorpusFingerprintProfile, Vec<CorpusCount>)> {
    let profile =
        load_profile_checked(profile_yaml).map_err(|e| value_error("Profile load error", e))?;
    let profile_metadata = CorpusFingerprintProfile {
        path: "<inline-profile>".to_string(),
        sha256: compute_sha256(profile_yaml),
        version: profile.version.clone(),
        message_structure: profile.message_structure.clone(),
    };

    let mut files = Vec::new();
    collect_python_corpus_files(path, &mut files)?;
    files.sort();

    let mut counts = BTreeMap::new();
    for file in files {
        let bytes = fs::read(&file).map_err(|e| value_error("Corpus read error", e))?;
        let parsed = if is_mllp_framed(&bytes) {
            rust_parse_mllp(&bytes)
        } else {
            rust_parse(&bytes)
        };
        let Ok(message) = parsed else {
            continue;
        };
        let issues = rust_validate(&message, &profile);
        let report = ValidationReport::from_issues(
            &message,
            Some(profile.message_structure.clone()),
            issues,
        );
        for issue in report.issues {
            let count = counts.entry(issue.code).or_insert(0usize);
            *count = count.saturating_add(1);
        }
    }

    Ok((profile_metadata, counts_to_corpus_counts(counts)))
}

fn collect_python_corpus_files(path: &Path, files: &mut Vec<PathBuf>) -> PyResult<()> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(PyValueError::new_err(format!(
            "{} is not a file or directory",
            path.display()
        )));
    }

    for entry in fs::read_dir(path).map_err(|e| value_error("Corpus read error", e))? {
        let entry = entry.map_err(|e| value_error("Corpus read error", e))?;
        let child = entry.path();
        if child.is_dir() {
            collect_python_corpus_files(&child, files)?;
        } else if child.is_file() {
            files.push(child);
        }
    }

    Ok(())
}

fn counts_to_corpus_counts(counts: BTreeMap<String, usize>) -> Vec<CorpusCount> {
    counts
        .into_iter()
        .map(|(value, count)| CorpusCount { value, count })
        .collect()
}

fn parse_ack_code(code: &str) -> PyResult<RustAckCode> {
    match code.to_ascii_uppercase().as_str() {
        "AA" => Ok(RustAckCode::AA),
        "AE" => Ok(RustAckCode::AE),
        "AR" => Ok(RustAckCode::AR),
        "CA" => Ok(RustAckCode::CA),
        "CE" => Ok(RustAckCode::CE),
        "CR" => Ok(RustAckCode::CR),
        _ => Err(PyValueError::new_err(
            "ack code must be one of AA, AE, AR, CA, CE, CR",
        )),
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

/// Generate an HL7 ACK message for a raw HL7 message.
#[pyfunction(signature = (content, code = "AA"))]
pub fn ack(content: &str, code: &str) -> PyResult<String> {
    let message = rust_parse(content.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("Parse error: {e}")))?;
    let ack_code = parse_ack_code(code)?;
    let ack_message = rust_ack(&message, ack_code)
        .map_err(|e| PyValueError::new_err(format!("ACK error: {e}")))?;
    String::from_utf8(rust_write(&ack_message))
        .map_err(|e| PyValueError::new_err(format!("ACK UTF-8 error: {e}")))
}

/// Generate deterministic HL7 messages from a template YAML string.
#[pyfunction(signature = (template_yaml, seed = 42, count = 1))]
pub fn generate(template_yaml: &str, seed: u64, count: usize) -> PyResult<Vec<String>> {
    let template: RustTemplate = serde_yaml::from_str(template_yaml)
        .map_err(|e| PyValueError::new_err(format!("Template parse error: {e}")))?;
    let messages = rust_generate(&template, seed, count)
        .map_err(|e| PyValueError::new_err(format!("Generate error: {e}")))?;

    messages
        .into_iter()
        .map(|message| {
            String::from_utf8(rust_write(&message))
                .map_err(|e| PyValueError::new_err(format!("Generate UTF-8 error: {e}")))
        })
        .collect()
}

/// Lint a profile YAML string and return a Python dict report.
#[pyfunction(signature = (profile_yaml, schema_version = 1))]
pub fn profile_lint<'py>(
    py: Python<'py>,
    profile_yaml: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    let report = rust_lint_profile_yaml(profile_yaml);
    match schema_version {
        1 => report_to_dict(py, &report, "Profile lint serialization error"),
        2 => report_to_dict(
            py,
            &report.to_v2("hl7v2-python", env!("CARGO_PKG_VERSION")),
            "Profile lint v2 serialization error",
        ),
        _ => Err(PyValueError::new_err(
            "profile lint schema_version must be 1 or 2",
        )),
    }
}

/// Explain a profile YAML string and return a Python dict report.
#[pyfunction(signature = (profile_yaml, profile_name = "<inline-profile>", schema_version = 1))]
pub fn profile_explain<'py>(
    py: Python<'py>,
    profile_yaml: &str,
    profile_name: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    let profile = load_profile_checked(profile_yaml)
        .map_err(|e| PyValueError::new_err(format!("Profile load error: {e}")))?;
    let lint_report = rust_lint_profile_yaml(profile_yaml);
    let report = rust_explain_profile(
        profile_name.to_string(),
        profile_yaml,
        &profile,
        &lint_report,
    );
    match schema_version {
        1 => report_to_dict(py, &report, "Profile explain serialization error"),
        2 => report_to_dict(
            py,
            &report.to_v2("hl7v2-python", env!("CARGO_PKG_VERSION")),
            "Profile explain v2 serialization error",
        ),
        _ => Err(PyValueError::new_err(
            "profile explain schema_version must be 1 or 2",
        )),
    }
}

/// Test profile fixtures and return a Python dict report.
#[pyfunction(signature = (profile_yaml, fixture_dir, profile_name = "<inline-profile>", schema_version = 1))]
pub fn profile_test<'py>(
    py: Python<'py>,
    profile_yaml: &str,
    fixture_dir: &str,
    profile_name: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    if !matches!(schema_version, 1 | 2) {
        return Err(PyValueError::new_err(
            "profile test schema_version must be 1 or 2",
        ));
    }

    let profile = load_profile_checked(profile_yaml)
        .map_err(|e| PyValueError::new_err(format!("Profile load error: {e}")))?;
    let mut report =
        rust_run_profile_fixture_tests(Path::new(profile_name), Path::new(fixture_dir), &profile)
            .map_err(|e| PyValueError::new_err(format!("Profile test error: {e}")))?;
    sanitize_profile_test_report(&mut report, Path::new(fixture_dir));

    match schema_version {
        1 => report_to_dict(py, &report, "Profile test serialization error"),
        2 => report_to_dict(
            py,
            &report.to_v2("hl7v2-python", env!("CARGO_PKG_VERSION")),
            "Profile test v2 serialization error",
        ),
        _ => Err(PyValueError::new_err(
            "profile test schema_version must be 1 or 2",
        )),
    }
}

fn sanitize_profile_test_report(report: &mut ProfileTestReport, fixture_dir: &Path) {
    report.profile = public_path_label(&report.profile, "profile.yaml");
    report.fixtures = "fixtures".to_string();

    for case in &mut report.cases {
        case.path = case.name.clone();
        if let Some(validation_report) = &mut case.validation_report {
            validation_report.profile = Some(report.profile.clone());
        }
        if let Some(expected_report) = &mut case.expected_report {
            expected_report.path = relative_or_redacted_path(fixture_dir, &expected_report.path);
        }
    }
}

fn public_path_label(path: &str, fallback: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn relative_or_redacted_path(root: &Path, path: &str) -> String {
    let path = Path::new(path);
    path.strip_prefix(root).map_or_else(
        |_| "<expected-report>".to_string(),
        |relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        },
    )
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
        profile_identity: Some(ValidationReportProfileIdentity {
            label: "<inline-profile>".to_string(),
            message_structure: Some(profile.message_structure.clone()),
            version: Some(profile.version.clone()),
            sha256: Some(compute_sha256(profile_yaml)),
        }),
    })
}

/// Summarize a file or directory corpus and return a Python dict.
#[pyfunction(signature = (path, schema_version = 1))]
pub fn corpus_summary<'py>(
    py: Python<'py>,
    path: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    let summary =
        summarize_corpus_path(path).map_err(|e| value_error("Corpus summary error", e))?;
    match schema_version {
        1 => report_to_dict(py, &summary, "Corpus summary serialization error"),
        2 => {
            let summary_v2 = summary.to_v2("hl7v2-python", env!("CARGO_PKG_VERSION"));
            report_to_dict(py, &summary_v2, "Corpus summary v2 serialization error")
        }
        _ => Err(PyValueError::new_err(
            "corpus summary schema_version must be 1 or 2",
        )),
    }
}

/// Fingerprint a file or directory corpus and return a Python dict.
#[pyfunction(signature = (path, profile_yaml = None, schema_version = 1))]
pub fn corpus_fingerprint<'py>(
    py: Python<'py>,
    path: &str,
    profile_yaml: Option<&str>,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    let corpus_path = Path::new(path);
    let mut fingerprint = fingerprint_corpus_path(corpus_path)
        .map_err(|e| value_error("Corpus fingerprint error", e))?;

    if let Some(profile_yaml) = profile_yaml {
        let (profile_metadata, issue_counts) =
            profile_issue_counts_for_path(corpus_path, profile_yaml)?;
        fingerprint.profile = Some(profile_metadata);
        fingerprint.validation_issue_code_counts = issue_counts;
    }

    match schema_version {
        1 => report_to_dict(py, &fingerprint, "Corpus fingerprint serialization error"),
        2 => {
            let fingerprint_v2 = fingerprint.to_v2("hl7v2-python");
            report_to_dict(
                py,
                &fingerprint_v2,
                "Corpus fingerprint v2 serialization error",
            )
        }
        _ => Err(PyValueError::new_err(
            "corpus fingerprint schema_version must be 1 or 2",
        )),
    }
}

/// Diff two file or directory corpora and return a Python dict.
#[pyfunction(signature = (before, after, profile_yaml = None, schema_version = 1))]
pub fn corpus_diff<'py>(
    py: Python<'py>,
    before: &str,
    after: &str,
    profile_yaml: Option<&str>,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    let before_path = Path::new(before);
    let after_path = Path::new(after);
    let diff = if let Some(profile_yaml) = profile_yaml {
        let mut before_fingerprint = fingerprint_corpus_path(before_path)
            .map_err(|e| value_error("Corpus fingerprint error", e))?;
        let mut after_fingerprint = fingerprint_corpus_path(after_path)
            .map_err(|e| value_error("Corpus fingerprint error", e))?;
        let (profile_metadata, before_issue_counts) =
            profile_issue_counts_for_path(before_path, profile_yaml)?;
        let (_, after_issue_counts) = profile_issue_counts_for_path(after_path, profile_yaml)?;
        before_fingerprint.profile = Some(profile_metadata.clone());
        before_fingerprint.validation_issue_code_counts = before_issue_counts;
        after_fingerprint.profile = Some(profile_metadata);
        after_fingerprint.validation_issue_code_counts = after_issue_counts;
        diff_corpus_fingerprints(&before_fingerprint, &after_fingerprint)
    } else {
        diff_corpus_paths(before_path, after_path)
            .map_err(|e| value_error("Corpus diff error", e))?
    };

    match schema_version {
        1 => report_to_dict(py, &diff, "Corpus diff serialization error"),
        2 => {
            let diff_v2 = diff.to_v2("hl7v2-python");
            report_to_dict(py, &diff_v2, "Corpus diff v2 serialization error")
        }
        _ => Err(PyValueError::new_err(
            "corpus diff schema_version must be 1 or 2",
        )),
    }
}

/// Redact raw HL7 with a safe-analysis policy TOML string and return a Python dict.
#[pyfunction(signature = (content, policy_toml, schema_version = 1))]
pub fn redact<'py>(
    py: Python<'py>,
    content: &str,
    policy_toml: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    let output = rust_redact_hl7_safe_analysis(content.as_bytes(), policy_toml)
        .map_err(|e| value_error("Redaction error", e))?;
    match schema_version {
        1 => report_to_dict(py, &output, "Redaction serialization error"),
        2 => report_to_dict(
            py,
            &output.to_v2("hl7v2-python", env!("CARGO_PKG_VERSION")),
            "Redaction v2 serialization error",
        ),
        _ => Err(PyValueError::new_err(
            "redaction output schema_version must be 1 or 2",
        )),
    }
}

/// Write a redacted evidence bundle and return a Python dict summary.
#[pyfunction(signature = (content, profile_yaml, policy_toml, out_dir, schema_version = 1))]
pub fn bundle<'py>(
    py: Python<'py>,
    content: &str,
    profile_yaml: &str,
    policy_toml: &str,
    out_dir: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    if !matches!(schema_version, 1 | 2) {
        return Err(PyValueError::new_err(
            "bundle summary schema_version must be 1 or 2",
        ));
    }

    let summary = rust_write_safe_analysis_bundle(
        content.as_bytes(),
        profile_yaml,
        policy_toml,
        Path::new(out_dir),
        "hl7v2-python",
        schema_version,
    )
    .map_err(|e| value_error("Bundle error", e))?;
    if schema_version == 2 {
        report_to_dict(
            py,
            &summary.to_v2("hl7v2-python", env!("CARGO_PKG_VERSION")),
            "Bundle summary v2 serialization error",
        )
    } else {
        report_to_dict(py, &summary, "Bundle summary serialization error")
    }
}

/// Replay and verify an evidence bundle directory.
#[pyfunction(signature = (bundle_dir, schema_version = 1))]
pub fn replay<'py>(
    py: Python<'py>,
    bundle_dir: &str,
    schema_version: u8,
) -> PyResult<Bound<'py, PyAny>> {
    if !matches!(schema_version, 1 | 2) {
        return Err(PyValueError::new_err(
            "evidence replay schema_version must be 1 or 2",
        ));
    }

    let report = rust_replay_evidence_bundle(Path::new(bundle_dir), "hl7v2-python");
    if schema_version == 2 {
        report_to_dict(py, &report.to_v2(), "Replay report v2 serialization error")
    } else {
        report_to_dict(py, &report, "Replay report serialization error")
    }
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
    m.add_function(wrap_pyfunction!(ack, m)?)?;
    m.add_function(wrap_pyfunction!(generate, m)?)?;
    m.add_function(wrap_pyfunction!(profile_lint, m)?)?;
    m.add_function(wrap_pyfunction!(profile_explain, m)?)?;
    m.add_function(wrap_pyfunction!(profile_test, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(corpus_summary, m)?)?;
    m.add_function(wrap_pyfunction!(corpus_fingerprint, m)?)?;
    m.add_function(wrap_pyfunction!(corpus_diff, m)?)?;
    m.add_function(wrap_pyfunction!(redact, m)?)?;
    m.add_function(wrap_pyfunction!(bundle, m)?)?;
    m.add_function(wrap_pyfunction!(replay, m)?)?;
    Ok(())
}
