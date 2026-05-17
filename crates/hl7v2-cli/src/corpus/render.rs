//! Corpus report rendering.

use super::ReportFormat;
use hl7v2::synthetic::corpus::{
    CorpusCountDiff, CorpusDiffReport, CorpusFieldCardinalityDiff, CorpusFieldPresenceDiff,
    CorpusFingerprint, CorpusSummary, CorpusValueShapeStatsDiff,
};

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
