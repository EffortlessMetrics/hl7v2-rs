//! Statistics command implementation for the HL7 v2 CLI.

use crate::{ReportFormat, monitor};
use hl7v2::{Message, parse, parse_mllp};
use std::fs;
use std::path::PathBuf;

/// Statistics report structure for JSON/YAML output
#[derive(serde::Serialize)]
pub(crate) struct StatsReport {
    pub(crate) input_file: String,
    pub(crate) file_size: usize,
    pub(crate) segment_count: usize,
    pub(crate) segments: Vec<SegmentStats>,
    pub(crate) field_distributions: Option<Vec<FieldDistribution>>,
}

#[derive(serde::Serialize)]
pub(crate) struct SegmentStats {
    pub(crate) segment_id: String,
    pub(crate) count: usize,
}

#[derive(serde::Serialize)]
pub(crate) struct FieldDistribution {
    pub(crate) path: String,
    pub(crate) unique_values: usize,
    pub(crate) sample_values: Vec<String>,
}

/// Collect statistics from an HL7 message
pub(crate) fn collect_stats(message: &Message, distributions: bool) -> StatsReport {
    // Collect segment statistics
    let mut segment_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for segment in &message.segments {
        *segment_counts
            .entry(segment.id_str().to_string())
            .or_insert(0) += 1;
    }

    let segments: Vec<SegmentStats> = segment_counts
        .into_iter()
        .map(|(id, count)| SegmentStats {
            segment_id: id,
            count,
        })
        .collect();

    // Collect field distributions if requested
    let field_distributions = if distributions {
        let mut dists: Vec<FieldDistribution> = Vec::new();

        // Sample some common fields for distribution analysis
        for segment in &message.segments {
            let segment_id = segment.id_str();

            // Get field values (simplified - just first few fields)
            for (field_idx, field) in segment.fields.iter().enumerate().take(5) {
                if field_idx == 0 {
                    continue; // Skip segment ID field
                }

                let path = format!("{}.{}", segment_id, field_idx);
                // Get the first text value from the field
                let value = field.first_text().unwrap_or("").to_string();

                // Check if we already have this path
                if let Some(existing) = dists.iter_mut().find(|d| d.path == path) {
                    if !existing.sample_values.contains(&value) && existing.sample_values.len() < 10
                    {
                        existing.sample_values.push(value);
                    }
                    existing.unique_values = existing.sample_values.len();
                } else {
                    dists.push(FieldDistribution {
                        path,
                        unique_values: 1,
                        sample_values: vec![value],
                    });
                }
            }
        }

        Some(dists)
    } else {
        None
    };

    StatsReport {
        input_file: String::new(), // To be filled by caller
        file_size: 0,              // To be filled by caller
        segment_count: message.segments.len(),
        segments,
        field_distributions,
    }
}

/// Format statistics report based on requested format
pub(crate) fn format_stats_report(
    report: &StatsReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("Message Statistics:\n");
            output.push_str(&format!("  Input file: {}\n", report.input_file));
            output.push_str(&format!("  File size: {} bytes\n", report.file_size));
            output.push_str(&format!("  Total segments: {}\n", report.segment_count));
            output.push('\n');
            output.push_str("Segment breakdown:\n");
            for seg in &report.segments {
                output.push_str(&format!(
                    "  {}: {} occurrence(s)\n",
                    seg.segment_id, seg.count
                ));
            }

            if let Some(dists) = &report.field_distributions {
                output.push('\n');
                output.push_str("Field value distributions:\n");
                for dist in dists {
                    output.push_str(&format!("  {}:\n", dist.path));
                    output.push_str(&format!("    Unique values: {}\n", dist.unique_values));
                    if !dist.sample_values.is_empty() {
                        output.push_str(&format!(
                            "    Sample values: {:?}\n",
                            dist.sample_values.iter().take(5).collect::<Vec<_>>()
                        ));
                    }
                }
            }
            Ok(output)
        }
    }
}

pub(crate) fn stats_command(
    input: &PathBuf,
    mllp: bool,
    distributions: bool,
    format: &ReportFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the HL7 message file
    let contents = fs::read(input)?;
    let file_size = contents.len();

    let read_time = monitor.elapsed();
    monitor.record_metric("File read", read_time);

    // Parse the HL7 message
    let message = if mllp {
        parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

    // Collect statistics
    let mut stats_report = collect_stats(&message, distributions);
    stats_report.input_file = input.to_string_lossy().to_string();
    stats_report.file_size = file_size;

    // Format and output report
    let report_output = format_stats_report(&stats_report, format)?;
    println!("{}", report_output);

    let output_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("Output", output_time);

    Ok(())
}
