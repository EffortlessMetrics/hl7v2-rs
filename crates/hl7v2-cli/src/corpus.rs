//! Corpus inspection command implementations.

use crate::{OutputOptions, ReportFormat};
use clap::Subcommand;
use hl7v2::synthetic::corpus::{
    CorpusCount, CorpusCountDiff, CorpusDiffReport, CorpusFieldCardinalityDiff,
    CorpusFieldPresenceDiff, CorpusFingerprint, CorpusFingerprintProfile, CorpusSummary,
    CorpusValueShapeStatsDiff, compute_sha256, diff_corpus_fingerprints, diff_corpus_paths,
    fingerprint_corpus_path, summarize_corpus_path,
};
use hl7v2::{ValidationReport, is_mllp_framed, load_profile_checked, parse, parse_mllp, validate};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Summarize a directory or file corpus of HL7 messages
    Summarize {
        /// Corpus directory or single HL7 file
        path: PathBuf,

        /// Output summary format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

        /// Evidence schema version for machine-readable summary reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the summary report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Create a deterministic feed fingerprint
    Fingerprint {
        /// Corpus directory or single HL7 file
        path: PathBuf,

        /// Optional profile YAML file for validation issue-code counts
        #[arg(long)]
        profile: Option<PathBuf>,

        /// Output fingerprint format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

        /// Evidence schema version for machine-readable fingerprint reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the fingerprint report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Diff two directory or file corpora of HL7 messages
    Diff {
        /// Before corpus directory or single HL7 file
        before: PathBuf,

        /// After corpus directory or single HL7 file
        after: PathBuf,

        /// Optional profile YAML file for validation issue-code deltas
        #[arg(long)]
        profile: Option<PathBuf>,

        /// Output diff format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

        /// Evidence schema version for machine-readable diff reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the diff report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },
}

pub(crate) fn summarize_command(
    path: &PathBuf,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *format == ReportFormat::Text {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "corpus summary schema version is only available with --format json or --format yaml",
        )
        .into());
    }

    let summary = summarize_corpus_path(path)?;
    let output = format_corpus_summary(&summary, format, schema_version)?;
    output_options.emit(&output)?;
    Ok(())
}

pub(crate) fn format_corpus_summary(
    summary: &CorpusSummary,
    format: &ReportFormat,
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => Ok(serde_json::to_string_pretty(
            &summary.to_v2("hl7v2-cli", env!("CARGO_PKG_VERSION")),
        )?),
        ReportFormat::Yaml if schema_version == 2 => Ok(serde_yaml::to_string(
            &summary.to_v2("hl7v2-cli", env!("CARGO_PKG_VERSION")),
        )?),
        ReportFormat::Json => Ok(serde_json::to_string_pretty(summary)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(summary)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("Corpus Summary:\n");
            output.push_str(&format!("  Path: {}\n", summary.root));
            output.push_str(&format!("  Files scanned: {}\n", summary.file_count));
            output.push_str(&format!("  Parsed messages: {}\n", summary.message_count));
            output.push_str(&format!("  Parse errors: {}\n", summary.parse_error_count));
            output.push_str(&format!("  Total bytes: {}\n", summary.total_bytes));

            output.push('\n');
            output.push_str("Message types:\n");
            append_counts(&mut output, &summary.message_types);

            output.push('\n');
            output.push_str("Segments:\n");
            append_counts(&mut output, &summary.segments);

            output.push('\n');
            output.push_str("Field presence:\n");
            if summary.field_presence.is_empty() {
                output.push_str("  <none>\n");
            } else {
                for field in &summary.field_presence {
                    output.push_str(&format!(
                        "  {}: {} message(s), {} occurrence(s)\n",
                        field.path, field.message_count, field.occurrence_count
                    ));
                }
            }

            if !summary.parse_errors.is_empty() {
                output.push('\n');
                output.push_str("Parse errors:\n");
                for error in &summary.parse_errors {
                    output.push_str(&format!("  {}: {}\n", error.path, error.error));
                }
            }

            Ok(output)
        }
    }
}

fn append_counts(output: &mut String, counts: &[hl7v2::synthetic::corpus::CorpusCount]) {
    if counts.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for count in counts {
        output.push_str(&format!("  {}: {}\n", count.value, count.count));
    }
}

pub(crate) fn diff_command(
    before: &PathBuf,
    after: &PathBuf,
    profile: Option<&PathBuf>,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *format == ReportFormat::Text {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "corpus diff schema v2 is available only with --format json or --format yaml",
        )));
    }

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

pub(crate) fn fingerprint_command(
    path: &PathBuf,
    profile: Option<&PathBuf>,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *format == ReportFormat::Text {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "corpus fingerprint schema v2 is available only with --format json or --format yaml",
        )));
    }

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

pub(crate) fn format_corpus_diff(
    diff: &CorpusDiffReport,
    format: &ReportFormat,
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => {
            let diff_v2 = diff.to_v2("hl7v2-cli");
            Ok(serde_json::to_string_pretty(&diff_v2)?)
        }
        ReportFormat::Yaml if schema_version == 2 => {
            let diff_v2 = diff.to_v2("hl7v2-cli");
            Ok(serde_yaml::to_string(&diff_v2)?)
        }
        ReportFormat::Json => Ok(serde_json::to_string_pretty(diff)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(diff)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("Corpus Diff:\n");
            output.push_str(&format!("  Diff version: {}\n", diff.diff_version));
            output.push_str(&format!("  Tool version: {}\n", diff.tool_version));
            output.push_str(&format!("  Before: {}\n", diff.before_root));
            output.push_str(&format!("  After: {}\n", diff.after_root));

            if let Some(profile) = &diff.profile {
                output.push('\n');
                output.push_str("Profile:\n");
                output.push_str(&format!("  Path: {}\n", profile.path));
                output.push_str(&format!("  SHA-256: {}\n", profile.sha256));
                output.push_str(&format!("  Version: {}\n", profile.version));
                output.push_str(&format!(
                    "  Message structure: {}\n",
                    profile.message_structure
                ));
            }

            output.push('\n');
            output.push_str("Totals:\n");
            output.push_str(&format!(
                "  Files scanned: {} -> {} ({})\n",
                diff.file_count.before,
                diff.file_count.after,
                format_signed_delta(diff.file_count.delta)
            ));
            output.push_str(&format!(
                "  Parsed messages: {} -> {} ({})\n",
                diff.message_count.before,
                diff.message_count.after,
                format_signed_delta(diff.message_count.delta)
            ));
            output.push_str(&format!(
                "  Parse errors: {} -> {} ({})\n",
                diff.parse_error_count.before,
                diff.parse_error_count.after,
                format_signed_delta(diff.parse_error_count.delta)
            ));
            output.push_str(&format!(
                "  New message types: {}\n",
                format_string_list(&diff.new_message_types)
            ));
            output.push_str(&format!(
                "  Removed message types: {}\n",
                format_string_list(&diff.removed_message_types)
            ));
            output.push_str(&format!(
                "  New segments: {}\n",
                format_string_list(&diff.new_segments)
            ));
            output.push_str(&format!(
                "  Removed segments: {}\n",
                format_string_list(&diff.removed_segments)
            ));

            output.push('\n');
            output.push_str("Message types:\n");
            append_count_diffs(&mut output, &diff.message_type_counts);

            output.push('\n');
            output.push_str("Segments:\n");
            append_count_diffs(&mut output, &diff.segment_counts);

            output.push('\n');
            output.push_str("Field presence:\n");
            append_field_presence_diffs(&mut output, &diff.field_presence);

            output.push('\n');
            output.push_str("Field cardinality:\n");
            append_field_cardinality_diffs(&mut output, &diff.field_cardinality);

            output.push('\n');
            output.push_str("Value shapes:\n");
            append_value_shape_diffs(&mut output, &diff.value_shape_stats);

            if diff.profile.is_some() {
                output.push('\n');
                output.push_str("Validation issue codes:\n");
                append_count_diffs(&mut output, &diff.validation_issue_code_counts);
            }

            Ok(output)
        }
    }
}

fn fingerprint_validation_issue_counts(
    path: &Path,
    profile_path: &Path,
) -> Result<(CorpusFingerprintProfile, Vec<CorpusCount>), Box<dyn std::error::Error>> {
    let profile_yaml = fs::read_to_string(profile_path)?;
    let profile = load_profile_checked(&profile_yaml)?;
    let profile_metadata = CorpusFingerprintProfile {
        path: profile_path.to_string_lossy().to_string(),
        sha256: compute_sha256(&profile_yaml),
        version: profile.version.clone(),
        message_structure: profile.message_structure.clone(),
    };

    let mut files = Vec::new();
    collect_cli_corpus_files(path, &mut files)?;
    files.sort();

    let mut counts = std::collections::BTreeMap::new();
    for file in files {
        let bytes = fs::read(&file)?;
        let parsed = if is_mllp_framed(&bytes) {
            parse_mllp(&bytes)
        } else {
            parse(&bytes)
        };
        let Ok(message) = parsed else {
            continue;
        };
        let issues = validate(&message, &profile);
        let report = ValidationReport::from_issues(
            &message,
            Some(profile_path.to_string_lossy().to_string()),
            issues,
        );
        for issue in report.issues {
            let count = counts.entry(issue.code).or_insert(0usize);
            *count = count.saturating_add(1);
        }
    }

    Ok((profile_metadata, counts_to_corpus_counts(counts)))
}

fn collect_cli_corpus_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a file or directory", path.display()),
        )));
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            collect_cli_corpus_files(&child, files)?;
        } else if child.is_file() {
            files.push(child);
        }
    }

    Ok(())
}

fn counts_to_corpus_counts(counts: std::collections::BTreeMap<String, usize>) -> Vec<CorpusCount> {
    counts
        .into_iter()
        .map(|(value, count)| CorpusCount { value, count })
        .collect()
}

pub(crate) fn format_corpus_fingerprint(
    fingerprint: &CorpusFingerprint,
    format: &ReportFormat,
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => {
            let fingerprint_v2 = fingerprint.to_v2("hl7v2-cli");
            Ok(serde_json::to_string_pretty(&fingerprint_v2)?)
        }
        ReportFormat::Yaml if schema_version == 2 => {
            let fingerprint_v2 = fingerprint.to_v2("hl7v2-cli");
            Ok(serde_yaml::to_string(&fingerprint_v2)?)
        }
        ReportFormat::Json => Ok(serde_json::to_string_pretty(fingerprint)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(fingerprint)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("Corpus Fingerprint:\n");
            output.push_str(&format!("  Path: {}\n", fingerprint.root));
            output.push_str(&format!(
                "  Fingerprint version: {}\n",
                fingerprint.fingerprint_version
            ));
            output.push_str(&format!("  Tool version: {}\n", fingerprint.tool_version));
            output.push_str(&format!("  Files scanned: {}\n", fingerprint.file_count));
            output.push_str(&format!(
                "  Parsed messages: {}\n",
                fingerprint.message_count
            ));
            output.push_str(&format!(
                "  Parse errors: {}\n",
                fingerprint.parse_error_count
            ));

            if let Some(profile) = &fingerprint.profile {
                output.push('\n');
                output.push_str("Profile:\n");
                output.push_str(&format!("  Path: {}\n", profile.path));
                output.push_str(&format!("  SHA-256: {}\n", profile.sha256));
                output.push_str(&format!("  Version: {}\n", profile.version));
                output.push_str(&format!(
                    "  Message structure: {}\n",
                    profile.message_structure
                ));
            }

            output.push('\n');
            output.push_str("Message types:\n");
            append_counts(&mut output, &fingerprint.message_type_counts);

            output.push('\n');
            output.push_str("Segments:\n");
            append_counts(&mut output, &fingerprint.segment_counts);

            output.push('\n');
            output.push_str("Field presence:\n");
            append_fingerprint_field_presence(&mut output, fingerprint);

            output.push('\n');
            output.push_str("Value shapes:\n");
            append_value_shape_stats(&mut output, fingerprint);

            if fingerprint.profile.is_some() {
                output.push('\n');
                output.push_str("Validation issue codes:\n");
                append_counts(&mut output, &fingerprint.validation_issue_code_counts);
            }

            Ok(output)
        }
    }
}

fn append_count_diffs(output: &mut String, counts: &[CorpusCountDiff]) {
    if counts.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for count in counts {
        output.push_str(&format!(
            "  {}: {} -> {} ({})\n",
            count.value,
            count.before,
            count.after,
            format_signed_delta(count.delta)
        ));
    }
}

fn append_field_presence_diffs(output: &mut String, fields: &[CorpusFieldPresenceDiff]) {
    if fields.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for field in fields {
        output.push_str(&format!(
            "  {}: messages {} -> {} ({}), occurrences {} -> {} ({})\n",
            field.path,
            field.before_message_count,
            field.after_message_count,
            format_signed_delta(field.message_count_delta),
            field.before_occurrence_count,
            field.after_occurrence_count,
            format_signed_delta(field.occurrence_count_delta)
        ));
    }
}

fn append_field_cardinality_diffs(output: &mut String, fields: &[CorpusFieldCardinalityDiff]) {
    if fields.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for field in fields {
        output.push_str(&format!(
            "  {}: min {} -> {} ({}), max {} -> {} ({}), total {} -> {} ({})\n",
            field.path,
            field.before_min_per_message,
            field.after_min_per_message,
            format_signed_delta(field.min_per_message_delta),
            field.before_max_per_message,
            field.after_max_per_message,
            format_signed_delta(field.max_per_message_delta),
            field.before_total_occurrences,
            field.after_total_occurrences,
            format_signed_delta(field.total_occurrences_delta)
        ));
    }
}

fn append_value_shape_diffs(output: &mut String, shapes: &[CorpusValueShapeStatsDiff]) {
    if shapes.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for shape in shapes {
        output.push_str(&format!(
            "  {}: coded {} -> {} ({}), timestamp {} -> {} ({}), numeric {} -> {} ({}), null {} -> {} ({}), text {} -> {} ({})\n",
            shape.path,
            shape.coded_count.before,
            shape.coded_count.after,
            format_signed_delta(shape.coded_count.delta),
            shape.timestamp_count.before,
            shape.timestamp_count.after,
            format_signed_delta(shape.timestamp_count.delta),
            shape.numeric_count.before,
            shape.numeric_count.after,
            format_signed_delta(shape.numeric_count.delta),
            shape.null_count.before,
            shape.null_count.after,
            format_signed_delta(shape.null_count.delta),
            shape.text_count.before,
            shape.text_count.after,
            format_signed_delta(shape.text_count.delta)
        ));
    }
}

fn append_fingerprint_field_presence(output: &mut String, fingerprint: &CorpusFingerprint) {
    if fingerprint.field_presence.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for field in &fingerprint.field_presence {
        if let Some(cardinality) = fingerprint
            .field_cardinality
            .iter()
            .find(|candidate| candidate.path == field.path)
        {
            output.push_str(&format!(
                "  {}: {} message(s), {} occurrence(s), min {}, max {}\n",
                field.path,
                field.message_count,
                field.occurrence_count,
                cardinality.min_per_message,
                cardinality.max_per_message
            ));
        } else {
            output.push_str(&format!(
                "  {}: {} message(s), {} occurrence(s)\n",
                field.path, field.message_count, field.occurrence_count
            ));
        }
    }
}

fn append_value_shape_stats(output: &mut String, fingerprint: &CorpusFingerprint) {
    if fingerprint.value_shape_stats.is_empty() {
        output.push_str("  <none>\n");
        return;
    }

    for stats in &fingerprint.value_shape_stats {
        output.push_str(&format!(
            "  {}: coded {}, timestamp {}, numeric {}, null {}, text {}\n",
            stats.path,
            stats.coded_count,
            stats.timestamp_count,
            stats.numeric_count,
            stats.null_count,
            stats.text_count
        ));
    }
}

fn format_string_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(", ")
    }
}

fn format_signed_delta(delta: i128) -> String {
    if delta > 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}
