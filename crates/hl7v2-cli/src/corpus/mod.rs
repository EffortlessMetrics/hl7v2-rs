//! Corpus command orchestration.
//!
//! The corpus CLI is split into small modules so command dispatch, report
//! rendering, and profile-aware validation counting each have one reason to
//! change.

mod profile_validation;
mod render;

use self::profile_validation::fingerprint_validation_issue_counts;
pub(super) use self::render::{
    format_corpus_diff, format_corpus_fingerprint, format_corpus_summary,
};
use super::{OutputOptions, ReportFormat};
use hl7v2::synthetic::corpus::{
    diff_corpus_fingerprints, diff_corpus_paths, fingerprint_corpus_path, summarize_corpus_path,
};
use std::path::PathBuf;

fn ensure_schema_format_support(
    schema_version: u8,
    format: &ReportFormat,
    error_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *format == ReportFormat::Text {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, error_message).into());
    }

    Ok(())
}

pub(super) fn summarize_command(
    path: &PathBuf,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_schema_format_support(
        schema_version,
        format,
        "corpus summary schema version is only available with --format json or --format yaml",
    )?;

    let summary = summarize_corpus_path(path)?;
    let output = format_corpus_summary(&summary, format, schema_version)?;
    output_options.emit(&output)?;
    Ok(())
}

pub(super) fn diff_command(
    before: &PathBuf,
    after: &PathBuf,
    profile: Option<&PathBuf>,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_schema_format_support(
        schema_version,
        format,
        "corpus diff schema v2 is available only with --format json or --format yaml",
    )?;

    let diff = if let Some(profile_path) = profile {
        let mut before_fingerprint = fingerprint_corpus_path(before)?;
        let mut after_fingerprint = fingerprint_corpus_path(after)?;
        let (profile_metadata, before_issue_counts) =
            fingerprint_validation_issue_counts(before, profile_path)?;
        let (_, after_issue_counts) = fingerprint_validation_issue_counts(after, profile_path)?;
        before_fingerprint.profile = Some(profile_metadata.clone());
        before_fingerprint.validation_issue_code_counts = before_issue_counts;
        after_fingerprint.profile = Some(profile_metadata);
        after_fingerprint.validation_issue_code_counts = after_issue_counts;
        diff_corpus_fingerprints(&before_fingerprint, &after_fingerprint)
    } else {
        diff_corpus_paths(before, after)?
    };
    let output = format_corpus_diff(&diff, format, schema_version)?;
    output_options.emit(&output)?;
    Ok(())
}

pub(super) fn fingerprint_command(
    path: &PathBuf,
    profile: Option<&PathBuf>,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_schema_format_support(
        schema_version,
        format,
        "corpus fingerprint schema v2 is available only with --format json or --format yaml",
    )?;

    let mut fingerprint = fingerprint_corpus_path(path)?;

    if let Some(profile_path) = profile {
        let (profile_metadata, issue_counts) =
            fingerprint_validation_issue_counts(path, profile_path)?;
        fingerprint.profile = Some(profile_metadata);
        fingerprint.validation_issue_code_counts = issue_counts;
    }

    let output = format_corpus_fingerprint(&fingerprint, format, schema_version)?;
    output_options.emit(&output)?;
    Ok(())
}
