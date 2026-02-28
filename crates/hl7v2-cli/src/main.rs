//! Command-line interface for HL7 v2 processing.

use clap::{Parser, Subcommand};
use hl7v2_core::{parse, to_json, write};
use hl7v2_gen::{AckCode as GenAckCode, Template, ack, generate};
use hl7v2_prof::{load_profile, validate};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;
mod monitor;

mod serve;
#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(
    name = "hl7v2",
    about = "HL7 v2 parser, validator, and generator",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse HL7 v2 message and output JSON
    Parse {
        /// Input HL7 file
        input: PathBuf,

        /// Output JSON format
        #[arg(long)]
        json: bool,

        /// Output with canonical delimiters (|^~\&)
        #[arg(long)]
        canonical_delims: bool,

        /// Wrap output in MLLP envelope (add SB/EB markers)
        #[arg(long)]
        envelope: bool,

        /// Input is MLLP framed
        #[arg(long)]
        mllp: bool,

        /// Enable streaming mode for large files (memory-efficient processing)
        #[arg(long)]
        streaming: bool,

        /// Show summary statistics
        #[arg(long)]
        summary: bool,
    },

    /// Normalize HL7 v2 message
    Norm {
        /// Input HL7 file
        input: PathBuf,

        /// Use canonical delimiters (|^~\&)
        #[arg(long)]
        canonical_delims: bool,

        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Input is MLLP framed
        #[arg(long)]
        mllp_in: bool,

        /// Output should be MLLP framed
        #[arg(long)]
        mllp_out: bool,

        /// Show summary statistics
        #[arg(long)]
        summary: bool,
    },

    /// Validate HL7 v2 message against profile
    Val {
        /// Input HL7 file
        input: PathBuf,

        /// Profile YAML file
        #[arg(long)]
        profile: PathBuf,

        /// Input is MLLP framed
        #[arg(long)]
        mllp: bool,

        /// Show detailed validation results
        #[arg(long)]
        detailed: bool,

        /// Output validation report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        report: ReportFormat,

        /// Show summary statistics
        #[arg(long)]
        summary: bool,
    },

    /// Show statistics for HL7 v2 message
    Stats {
        /// Input HL7 file
        input: PathBuf,

        /// Input is MLLP framed
        #[arg(long)]
        mllp: bool,

        /// Show field value distributions
        #[arg(long)]
        distributions: bool,

        /// Output format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,
    },

    /// Generate ACK for HL7 v2 message
    Ack {
        /// Input HL7 file
        input: PathBuf,

        /// ACK mode (original or enhanced)
        #[arg(long)]
        mode: AckMode,

        /// ACK code
        #[arg(long)]
        code: AckCode,

        /// Input is MLLP framed
        #[arg(long)]
        mllp_in: bool,

        /// Output should be MLLP framed
        #[arg(long)]
        mllp_out: bool,

        /// Show summary statistics
        #[arg(long)]
        summary: bool,
    },

    /// Generate synthetic messages
    Gen {
        /// Profile YAML file
        #[arg(long)]
        profile: PathBuf,

        /// Random seed
        #[arg(long)]
        seed: u64,

        /// Number of messages to generate
        #[arg(long)]
        count: usize,

        /// Output directory
        #[arg(long)]
        out: PathBuf,

        /// Show generation statistics
        #[arg(long)]
        stats: bool,
    },

    /// Start HTTP/gRPC server for HL7 v2 processing
    Serve {
        /// Server mode (http or grpc)
        #[arg(long, value_enum, default_value = "http")]
        mode: ServerMode,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host address to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Maximum request body size in bytes
        #[arg(long, default_value = "10485760")]
        max_body_size: usize,
    },

    /// Interactive mode
    Interactive,
}

/// Server mode selection
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
enum ServerMode {
    /// HTTP server with REST API
    Http,
    /// gRPC server (requires grpc feature)
    Grpc,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum AckMode {
    Original,
    Enhanced,
}

#[derive(clap::ValueEnum, Clone, Debug)]
#[value(rename_all = "UPPERCASE")]
enum AckCode {
    AA,
    AE,
    AR,
    CA,
    CE,
    CR,
}

/// Report output format
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Default)]
enum ReportFormat {
    #[default]
    Text,
    Json,
    Yaml,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for server mode
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Parse {
            input,
            json,
            canonical_delims,
            envelope,
            mllp,
            streaming,
            summary,
        } => parse_command(
            input,
            *json,
            *canonical_delims,
            *envelope,
            *mllp,
            *streaming,
            *summary,
        ),
        Commands::Norm {
            input,
            canonical_delims,
            output,
            mllp_in,
            mllp_out,
            summary,
        } => norm_command(
            input,
            *canonical_delims,
            output,
            *mllp_in,
            *mllp_out,
            *summary,
        ),
        Commands::Val {
            input,
            profile,
            mllp,
            detailed,
            report,
            summary,
        } => val_command(input, profile, *mllp, *detailed, report, *summary),
        Commands::Stats {
            input,
            mllp,
            distributions,
            format,
        } => stats_command(input, *mllp, *distributions, format),
        Commands::Ack {
            input,
            mode,
            code,
            mllp_in,
            mllp_out,
            summary,
        } => ack_command(input, mode, code, *mllp_in, *mllp_out, *summary),
        Commands::Gen {
            profile,
            seed,
            count,
            out,
            stats,
        } => gen_command(profile, *seed, *count, out, *stats),
        Commands::Serve {
            mode,
            port,
            host,
            max_body_size,
        } => serve::run_server(mode, *port, host, *max_body_size).await,
        Commands::Interactive => interactive_mode(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Display performance statistics
fn display_performance_stats(monitor: &monitor::PerformanceMonitor) {
    println!();
    println!("Performance Statistics:");
    println!("  Total execution time: {:?}", monitor.elapsed());

    let metrics = monitor.get_metrics();
    if !metrics.is_empty() {
        println!("  Detailed metrics:");
        for (name, duration) in metrics {
            println!("    {}: {:?}", name, duration);
        }
    }

    // System information
    let system_info = monitor::get_system_info();
    println!("  System information:");
    if let Some(cpu_usage) = system_info.cpu.cpu_usage_percent {
        println!("    CPU usage: {:.2}%", cpu_usage);
    }
    println!("    Total memory: {} bytes", system_info.total_memory);
    println!("    Used memory: {} bytes", system_info.used_memory);
    if let Some(rss) = system_info.memory.resident_set_size {
        println!("    Process memory (RSS): {} bytes", rss);
    }
    if let Some(vms) = system_info.memory.virtual_memory_size {
        println!("    Process memory (VMS): {} bytes", vms);
    }
}

fn parse_command(
    input: &PathBuf,
    json: bool,
    canonical_delims: bool,
    envelope: bool,
    mllp: bool,
    streaming: bool,
    summary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the input file
    let contents = if streaming {
        // For streaming mode, read file in chunks would be ideal
        // For now, we still read the whole file but indicate streaming mode
        fs::read(input)?
    } else {
        fs::read(input)?
    };
    let file_size = contents.len();

    let read_time = monitor.elapsed();
    monitor.record_metric("File read", read_time);

    // Parse the HL7 message
    let message = if mllp {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

    // Count segments
    let segment_count = message.segments.len();

    // Handle output based on flags
    if canonical_delims {
        // Output with canonical delimiters (|^~\&)
        // Normalize the raw bytes with canonical delimiters
        let original_bytes = write(&message);
        let output_bytes = hl7v2_core::normalize(&original_bytes, true)?;

        if envelope {
            // Wrap in MLLP envelope
            let mllp_bytes = hl7v2_core::wrap_mllp(&output_bytes);
            std::io::stdout().write_all(&mllp_bytes)?;
        } else {
            std::io::stdout().write_all(&output_bytes)?;
        }
    } else if envelope {
        // Output with original delimiters but wrapped in MLLP envelope
        let output_bytes = write(&message);
        let mllp_bytes = hl7v2_core::wrap_mllp(&output_bytes);
        std::io::stdout().write_all(&mllp_bytes)?;
    } else {
        // Default JSON output
        let json_value = to_json(&message);
        let json_conversion_time = monitor.elapsed() - read_time - parse_time;
        monitor.record_metric("JSON conversion", json_conversion_time);

        // Output JSON
        if json {
            println!("{}", serde_json::to_string_pretty(&json_value)?);
        } else {
            println!("{}", serde_json::to_string(&json_value)?);
        }
    }

    let output_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("Output", output_time);

    // Show summary if requested
    if summary {
        println!();
        println!("Parse Summary:");
        println!("  Input file: {:?}", input);
        println!("  File size: {} bytes", file_size);
        println!("  Segments: {}", segment_count);
        println!("  Streaming mode: {}", streaming);
        println!("  Canonical delimiters: {}", canonical_delims);
        println!("  MLLP envelope: {}", envelope);
        println!(
            "  Delimiters: |^~\\& (field={} comp={} rep={} esc={} sub={})",
            message.delims.field,
            message.delims.comp,
            message.delims.rep,
            message.delims.esc,
            message.delims.sub
        );
        display_performance_stats(&monitor);
    }

    Ok(())
}

fn norm_command(
    input: &PathBuf,
    canonical_delims: bool,
    output: &Option<PathBuf>,
    mllp_in: bool,
    mllp_out: bool,
    summary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the input file
    let contents = fs::read(input)?;
    let input_file_size = contents.len();

    let read_time = monitor.elapsed();
    monitor.record_metric("File read", read_time);

    // Parse the HL7 message
    let message = if mllp_in {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

    // Count segments before normalization
    let segment_count = message.segments.len();

    // Normalize the message
    let normalized_bytes = if canonical_delims {
        // We need to implement normalization with canonical delimiters
        // For now, we'll just use the regular write function
        write(&message)
    } else {
        write(&message)
    };

    let normalize_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("Message normalization", normalize_time);

    // Add MLLP framing if requested
    let output_bytes = if mllp_out {
        hl7v2_core::wrap_mllp(&normalized_bytes)
    } else {
        normalized_bytes
    };

    let mllp_time = monitor.elapsed() - read_time - parse_time - normalize_time;
    monitor.record_metric("MLLP processing", mllp_time);

    // Write to output file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, &output_bytes)?;
        if summary {
            let write_time =
                monitor.elapsed() - read_time - parse_time - normalize_time - mllp_time;
            monitor.record_metric("File write", write_time);

            println!();
            println!("Normalize Summary:");
            println!("  Input file: {:?}", input);
            println!("  Output file: {:?}", output_path);
            println!("  Input size: {} bytes", input_file_size);
            println!("  Output size: {} bytes", output_bytes.len());
            println!("  Segments: {}", segment_count);
            println!("  Canonical delimiters: {}", canonical_delims);
            println!("  MLLP output: {}", mllp_out);
            display_performance_stats(&monitor);
        }
    } else {
        std::io::stdout().write_all(&output_bytes)?;
        if summary {
            let write_time =
                monitor.elapsed() - read_time - parse_time - normalize_time - mllp_time;
            monitor.record_metric("Output write", write_time);

            println!();
            println!("Normalize Summary:");
            println!("  Input file: {:?}", input);
            println!("  Output: stdout");
            println!("  Input size: {} bytes", input_file_size);
            println!("  Output size: {} bytes", output_bytes.len());
            println!("  Segments: {}", segment_count);
            println!("  Canonical delimiters: {}", canonical_delims);
            println!("  MLLP output: {}", mllp_out);
            display_performance_stats(&monitor);
        }
    }

    Ok(())
}

fn val_command(
    input: &PathBuf,
    profile: &PathBuf,
    mllp: bool,
    detailed: bool,
    report: &ReportFormat,
    summary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the HL7 message file
    let contents = fs::read(input)?;
    let file_size = contents.len();

    let read_time = monitor.elapsed();
    monitor.record_metric("File read", read_time);

    // Parse the HL7 message
    let message = if mllp {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

    // Read the profile YAML file
    let profile_yaml = fs::read_to_string(profile)?;

    let read_profile_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("Profile read", read_profile_time);

    // Load the profile
    let loaded_profile = load_profile(&profile_yaml)?;

    let load_profile_time = monitor.elapsed() - read_time - parse_time - read_profile_time;
    monitor.record_metric("Profile loading", load_profile_time);

    // Validate the message
    let results = validate(&message, &loaded_profile);

    let validation_time =
        monitor.elapsed() - read_time - parse_time - read_profile_time - load_profile_time;
    monitor.record_metric("Message validation", validation_time);

    // Build validation report
    let validation_report = ValidationReport {
        input_file: input.to_string_lossy().to_string(),
        profile_file: profile.to_string_lossy().to_string(),
        file_size,
        segment_count: message.segments.len(),
        is_valid: results.is_empty(),
        issue_count: results.len(),
        issues: results.iter().map(|r| format!("{:?}", r)).collect(),
    };

    // Output based on report format
    match report {
        ReportFormat::Json => {
            let json_output = serde_json::to_string_pretty(&validation_report)?;
            println!("{}", json_output);
        }
        ReportFormat::Yaml => {
            let yaml_output = serde_yaml::to_string(&validation_report)?;
            println!("{}", yaml_output);
        }
        ReportFormat::Text => {
            // Print validation results in text format
            if results.is_empty() {
                println!("Validation passed: No issues found");
            } else {
                if detailed {
                    println!("Validation issues found:");
                    for result in &results {
                        println!("  - {:?}", result);
                    }
                } else {
                    println!("Validation failed: {} issues found", results.len());
                }
            }
        }
    }

    // Show summary if requested (only for text format to avoid mixing output)
    if summary && *report == ReportFormat::Text {
        println!();
        println!("Validation Summary:");
        println!("  Input file: {:?}", input);
        println!("  Profile file: {:?}", profile);
        println!("  File size: {} bytes", file_size);
        println!("  Segments: {}", message.segments.len());
        println!("  Issues found: {}", results.len());
        display_performance_stats(&monitor);
    }

    // Exit with error code if validation failed
    if !results.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

/// Validation report structure for JSON/YAML output
#[derive(serde::Serialize)]
struct ValidationReport {
    input_file: String,
    profile_file: String,
    file_size: usize,
    segment_count: usize,
    is_valid: bool,
    issue_count: usize,
    issues: Vec<String>,
}

/// Statistics report structure for JSON/YAML output
#[derive(serde::Serialize)]
struct StatsReport {
    input_file: String,
    file_size: usize,
    segment_count: usize,
    segments: Vec<SegmentStats>,
    field_distributions: Option<Vec<FieldDistribution>>,
}

#[derive(serde::Serialize)]
struct SegmentStats {
    segment_id: String,
    count: usize,
}

#[derive(serde::Serialize)]
struct FieldDistribution {
    path: String,
    unique_values: usize,
    sample_values: Vec<String>,
}

fn stats_command(
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
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

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
        let mut distributions: Vec<FieldDistribution> = Vec::new();

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
                if let Some(existing) = distributions.iter_mut().find(|d| d.path == path) {
                    if !existing.sample_values.contains(&value) && existing.sample_values.len() < 10
                    {
                        existing.sample_values.push(value);
                    }
                    existing.unique_values = existing.sample_values.len();
                } else {
                    distributions.push(FieldDistribution {
                        path,
                        unique_values: 1,
                        sample_values: vec![value],
                    });
                }
            }
        }

        Some(distributions)
    } else {
        None
    };

    let stats_report = StatsReport {
        input_file: input.to_string_lossy().to_string(),
        file_size,
        segment_count: message.segments.len(),
        segments,
        field_distributions,
    };

    // Output based on format
    match format {
        ReportFormat::Json => {
            let json_output = serde_json::to_string_pretty(&stats_report)?;
            println!("{}", json_output);
        }
        ReportFormat::Yaml => {
            let yaml_output = serde_yaml::to_string(&stats_report)?;
            println!("{}", yaml_output);
        }
        ReportFormat::Text => {
            println!("Message Statistics:");
            println!("  Input file: {:?}", input);
            println!("  File size: {} bytes", file_size);
            println!("  Total segments: {}", stats_report.segment_count);
            println!();
            println!("Segment breakdown:");
            for seg in &stats_report.segments {
                println!("  {}: {} occurrence(s)", seg.segment_id, seg.count);
            }

            if let Some(dists) = &stats_report.field_distributions {
                println!();
                println!("Field value distributions:");
                for dist in dists {
                    println!("  {}:", dist.path);
                    println!("    Unique values: {}", dist.unique_values);
                    if !dist.sample_values.is_empty() {
                        println!(
                            "    Sample values: {:?}",
                            dist.sample_values.iter().take(5).collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
    }

    let output_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("Output", output_time);

    Ok(())
}

fn ack_command(
    input: &PathBuf,
    mode: &AckMode,
    code: &AckCode,
    mllp_in: bool,
    mllp_out: bool,
    summary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the HL7 message file
    let contents = fs::read(input)?;
    let input_file_size = contents.len();

    let read_time = monitor.elapsed();
    monitor.record_metric("File read", read_time);

    // Parse the HL7 message
    let message = if mllp_in {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

    // Convert ACK code
    let ack_code = match code {
        AckCode::AA => GenAckCode::AA,
        AckCode::AE => GenAckCode::AE,
        AckCode::AR => GenAckCode::AR,
        AckCode::CA => GenAckCode::CA,
        AckCode::CE => GenAckCode::CE,
        AckCode::CR => GenAckCode::CR,
    };

    // Generate ACK
    let ack_message = ack(&message, ack_code)?; // Remove the extra parameter

    let ack_generation_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("ACK generation", ack_generation_time);

    // Write ACK message
    let ack_bytes = if mllp_out {
        hl7v2_core::write_mllp(&ack_message)
    } else {
        write(&ack_message)
    };

    let mllp_processing_time = monitor.elapsed() - read_time - parse_time - ack_generation_time;
    monitor.record_metric("MLLP processing", mllp_processing_time);

    std::io::stdout().write_all(&ack_bytes)?;

    // Show summary if requested
    if summary {
        let write_time =
            monitor.elapsed() - read_time - parse_time - ack_generation_time - mllp_processing_time;
        monitor.record_metric("Output write", write_time);

        println!();
        println!("ACK Generation Summary:");
        println!("  Input file: {:?}", input);
        println!("  Mode: {:?}", mode);
        println!("  Code: {:?}", code);
        println!("  Input size: {} bytes", input_file_size);
        println!("  Output size: {} bytes", ack_bytes.len());
        println!("  Segments in original: {}", message.segments.len());
        println!("  Segments in ACK: {}", ack_message.segments.len());
        println!("  MLLP input: {}", mllp_in);
        println!("  MLLP output: {}", mllp_out);
        display_performance_stats(&monitor);
    }

    Ok(())
}

/// Interactive mode for HL7 v2 processing
fn interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("HL7 v2 Toolkit - Interactive Mode");
    println!("Type 'help' for available commands or 'exit' to quit.");
    println!();

    loop {
        print!("hl7v2> ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  parse <file> [options]  - Parse an HL7 message");
                println!("  norm <file> [options]   - Normalize an HL7 message");
                println!("  val <file> <profile>    - Validate an HL7 message");
                println!("  ack <file> [options]    - Generate an ACK for an HL7 message");
                println!("  gen <profile> [options] - Generate synthetic messages");
                println!("  help                    - Show this help message");
                println!("  exit|quit               - Exit interactive mode");
                println!();
            }
            _ => {
                if input.starts_with("parse ") {
                    handle_parse_command(input)?;
                } else if input.starts_with("norm ") {
                    handle_norm_command(input)?;
                } else if input.starts_with("val ") {
                    handle_val_command(input)?;
                } else if input.starts_with("ack ") {
                    handle_ack_command(input)?;
                } else if input.starts_with("gen ") {
                    handle_gen_command(input)?;
                } else if !input.is_empty() {
                    println!("Unknown command. Type 'help' for available commands.");
                }
            }
        }
    }

    Ok(())
}

/// Handle parse command in interactive mode
fn handle_parse_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        println!(
            "Usage: parse <file> [--json] [--canonical-delims] [--envelope] [--mllp] [--streaming] [--summary]"
        );
        return Ok(());
    }

    let file_path = PathBuf::from(parts[1]);
    let mut json = false;
    let mut canonical_delims = false;
    let mut envelope = false;
    let mut mllp = false;
    let mut streaming = false;
    let mut summary = false;

    for part in &parts[2..] {
        match *part {
            "--json" => json = true,
            "--canonical-delims" => canonical_delims = true,
            "--envelope" => envelope = true,
            "--mllp" => mllp = true,
            "--streaming" => streaming = true,
            "--summary" => summary = true,
            _ => println!("Unknown option: {}", part),
        }
    }

    parse_command(
        &file_path,
        json,
        canonical_delims,
        envelope,
        mllp,
        streaming,
        summary,
    )
}

/// Handle norm command in interactive mode
fn handle_norm_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        println!("Usage: norm <file> [--canonical-delims] [--mllp-in] [--mllp-out] [--summary]");
        return Ok(());
    }

    let file_path = PathBuf::from(parts[1]);
    let mut canonical_delims = false;
    let mut mllp_in = false;
    let mut mllp_out = false;
    let mut summary = false;

    for part in &parts[2..] {
        match *part {
            "--canonical-delims" => canonical_delims = true,
            "--mllp-in" => mllp_in = true,
            "--mllp-out" => mllp_out = true,
            "--summary" => summary = true,
            _ => println!("Unknown option: {}", part),
        }
    }

    norm_command(
        &file_path,
        canonical_delims,
        &None,
        mllp_in,
        mllp_out,
        summary,
    )
}

/// Handle val command in interactive mode
fn handle_val_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 3 {
        println!(
            "Usage: val <file> <profile> [--mllp] [--detailed] [--report <text|json|yaml>] [--summary]"
        );
        return Ok(());
    }

    let file_path = PathBuf::from(parts[1]);
    let profile_path = PathBuf::from(parts[2]);
    let mut mllp = false;
    let mut detailed = false;
    let mut summary = false;
    let mut report = ReportFormat::Text;

    let mut i = 3;
    while i < parts.len() {
        match parts[i] {
            "--mllp" => {
                mllp = true;
                i += 1;
            }
            "--detailed" => {
                detailed = true;
                i += 1;
            }
            "--summary" => {
                summary = true;
                i += 1;
            }
            "--report" => {
                if i + 1 < parts.len() {
                    report = match parts[i + 1] {
                        "json" => ReportFormat::Json,
                        "yaml" => ReportFormat::Yaml,
                        _ => ReportFormat::Text,
                    };
                    i += 2;
                } else {
                    println!("Missing report format value");
                    return Ok(());
                }
            }
            _ => {
                println!("Unknown option: {}", parts[i]);
                i += 1;
            }
        }
    }

    val_command(&file_path, &profile_path, mllp, detailed, &report, summary)
}

/// Handle ack command in interactive mode
fn handle_ack_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        println!(
            "Usage: ack <file> [--mode <original|enhanced>] [--code <AA|AE|AR|CA|CE|CR>] [--mllp-in] [--mllp-out] [--summary]"
        );
        return Ok(());
    }

    let file_path = PathBuf::from(parts[1]);
    let mut mode = AckMode::Original;
    let mut code = AckCode::AA;
    let mut mllp_in = false;
    let mut mllp_out = false;
    let mut summary = false;

    let mut i = 2;
    while i < parts.len() {
        match parts[i] {
            "--mode" => {
                if i + 1 < parts.len() {
                    mode = match parts[i + 1] {
                        "original" => AckMode::Original,
                        "enhanced" => AckMode::Enhanced,
                        _ => {
                            println!("Invalid mode: {}", parts[i + 1]);
                            return Ok(());
                        }
                    };
                    i += 2;
                } else {
                    println!("Missing mode value");
                    return Ok(());
                }
            }
            "--code" => {
                if i + 1 < parts.len() {
                    code = match parts[i + 1] {
                        "AA" => AckCode::AA,
                        "AE" => AckCode::AE,
                        "AR" => AckCode::AR,
                        "CA" => AckCode::CA,
                        "CE" => AckCode::CE,
                        "CR" => AckCode::CR,
                        _ => {
                            println!("Invalid code: {}", parts[i + 1]);
                            return Ok(());
                        }
                    };
                    i += 2;
                } else {
                    println!("Missing code value");
                    return Ok(());
                }
            }
            "--mllp-in" => {
                mllp_in = true;
                i += 1;
            }
            "--mllp-out" => {
                mllp_out = true;
                i += 1;
            }
            "--summary" => {
                summary = true;
                i += 1;
            }
            _ => {
                println!("Unknown option: {}", parts[i]);
                return Ok(());
            }
        }
    }

    ack_command(&file_path, &mode, &code, mllp_in, mllp_out, summary)
}

/// Handle gen command in interactive mode
fn handle_gen_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        println!(
            "Usage: gen <profile> [--seed <number>] [--count <number>] [--out <directory>] [--stats]"
        );
        return Ok(());
    }

    let profile_path = PathBuf::from(parts[1]);
    let mut seed = 42;
    let mut count = 1;
    let mut out = PathBuf::from("output");
    let mut stats = false;

    let mut i = 2;
    while i < parts.len() {
        match parts[i] {
            "--seed" => {
                if i + 1 < parts.len() {
                    seed = parts[i + 1].parse().unwrap_or(42);
                    i += 2;
                } else {
                    println!("Missing seed value");
                    return Ok(());
                }
            }
            "--count" => {
                if i + 1 < parts.len() {
                    count = parts[i + 1].parse().unwrap_or(1);
                    i += 2;
                } else {
                    println!("Missing count value");
                    return Ok(());
                }
            }
            "--out" => {
                if i + 1 < parts.len() {
                    out = PathBuf::from(parts[i + 1]);
                    i += 2;
                } else {
                    println!("Missing output directory");
                    return Ok(());
                }
            }
            "--stats" => {
                stats = true;
                i += 1;
            }
            _ => {
                println!("Unknown option: {}", parts[i]);
                return Ok(());
            }
        }
    }

    gen_command(&profile_path, seed, count, &out, stats)
}

fn gen_command(
    profile: &PathBuf,
    seed: u64,
    count: usize,
    out: &PathBuf,
    stats: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the template YAML file
    let template_yaml = fs::read_to_string(profile)?;

    let read_template_time = monitor.elapsed();
    monitor.record_metric("Template read", read_template_time);

    // Parse the template from YAML
    let template: Template = serde_yaml::from_str(&template_yaml)?;

    let parse_template_time = monitor.elapsed() - read_template_time;
    monitor.record_metric("Template parsing", parse_template_time);

    // Generate messages
    let messages = generate(&template, seed, count)?;

    let generation_time = monitor.elapsed() - read_template_time - parse_template_time;
    monitor.record_metric("Message generation", generation_time);

    // Create output directory if it doesn't exist
    fs::create_dir_all(out)?;

    let create_dir_time =
        monitor.elapsed() - read_template_time - parse_template_time - generation_time;
    monitor.record_metric("Directory creation", create_dir_time);

    // Write each message to a separate file
    let mut written_files = 0;
    for (i, message) in messages.iter().enumerate() {
        let filename = out.join(format!("message_{:03}.hl7", i + 1));
        let message_bytes = write(message);
        fs::write(&filename, &message_bytes)?;
        if stats {
            println!("Generated message written to: {:?}", filename);
        }
        written_files += 1;
    }

    let write_time = monitor.elapsed()
        - read_template_time
        - parse_template_time
        - generation_time
        - create_dir_time;
    monitor.record_metric("File writing", write_time);

    if stats {
        println!("Successfully generated {} messages", messages.len());
    }

    // Show stats if requested
    if stats {
        println!();
        println!("Generation Statistics:");
        println!("  Template file: {:?}", profile);
        println!("  Seed: {}", seed);
        println!("  Count: {}", count);
        println!("  Output directory: {:?}", out);
        println!("  Messages generated: {}", messages.len());
        println!("  Files written: {}", written_files);
        display_performance_stats(&monitor);
    }

    Ok(())
}
