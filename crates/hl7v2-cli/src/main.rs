//! Command-line interface for HL7 v2 processing.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use std::process;
use hl7v2_core::{parse, to_json, write};
use hl7v2_prof::{load_profile, validate};
use hl7v2_gen::{ack, AckCode as GenAckCode, Template, generate};
mod monitor;
use monitor::{PerformanceMonitor, get_memory_info, get_cpu_info};

#[derive(Parser)]
#[command(name = "hl7v2", about = "HL7 v2 parser, validator, and generator")]
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
        
        /// Include envelope information in JSON output
        #[arg(long)]
        envelope: Option<PathBuf>,
        
        /// Input is MLLP framed
        #[arg(long)]
        mllp: bool,
        
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
        
        /// Show summary statistics
        #[arg(long)]
        summary: bool,
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
    
    /// Interactive mode
    Interactive,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum AckMode {
    Original,
    Enhanced,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum AckCode {
    AA,
    AE,
    AR,
    CA,
    CE,
    CR,
}

fn main() {
    let cli = Cli::parse();
    
    let result = match &cli.command {
        Commands::Parse { input, json, envelope, mllp, summary } => {
            parse_command(input, *json, envelope, *mllp, *summary)
        }
        Commands::Norm { input, canonical_delims, output, mllp_in, mllp_out, summary } => {
            norm_command(input, *canonical_delims, output, *mllp_in, *mllp_out, *summary)
        }
        Commands::Val { input, profile, mllp, detailed, summary } => {
            val_command(input, profile, *mllp, *detailed, *summary)
        }
        Commands::Ack { input, mode, code, mllp_in, mllp_out, summary } => {
            ack_command(input, mode, code, *mllp_in, *mllp_out, *summary)
        }
        Commands::Gen { profile, seed, count, out, stats } => {
            gen_command(profile, *seed, *count, out, *stats)
        }
        Commands::Interactive => {
            interactive_mode()
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Format bytes into human-readable string
#[allow(clippy::cast_precision_loss)]
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    }
}

/// Display performance statistics
fn display_performance_stats(monitor: &monitor::PerformanceMonitor) {
    println!();
    println!("Performance Statistics:");
    println!("  Total execution time: {:?}", monitor.elapsed());
    
    let mut metrics: Vec<_> = monitor.get_metrics().iter().collect();
    metrics.sort_by_key(|k| k.0);

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
    println!("    Total memory: {}", format_size(system_info.total_memory));
    println!("    Used memory: {}", format_size(system_info.used_memory));
    if let Some(rss) = system_info.memory.resident_set_size {
        println!("    Process memory (RSS): {}", format_size(rss));
    }
    if let Some(vms) = system_info.memory.virtual_memory_size {
        println!("    Process memory (VMS): {}", format_size(vms));
    }
}

fn parse_command(input: &PathBuf, json: bool, envelope: &Option<PathBuf>, mllp: bool, summary: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = monitor::PerformanceMonitor::new();
    
    // Read the input file
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
    
    // Count segments
    let segment_count = message.segments.len();
    
    // Convert to JSON
    let json_value = to_json(&message);
    
    let json_conversion_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("JSON conversion", json_conversion_time);
    
    // Output JSON
    if json {
        println!("{}", serde_json::to_string_pretty(&json_value)?);
    } else {
        println!("{}", serde_json::to_string(&json_value)?);
    }
    
    let output_time = monitor.elapsed() - read_time - parse_time - json_conversion_time;
    monitor.record_metric("Output", output_time);
    
    // Handle envelope if specified
    if let Some(envelope_path) = envelope {
        // For now, we'll just print a message
        println!("Envelope would be written to: {:?}", envelope_path);
    }
    
    // Show summary if requested
    if summary {
        println!();
        println!("Parse Summary:");
        println!("  Input file: {:?}", input);
        println!("  File size: {}", format_size(file_size as u64));
        println!("  Segments: {}", segment_count);
        println!("  Delimiters: |^~\\& (field={} comp={} rep={} esc={} sub={})", 
                 message.delims.field, message.delims.comp, message.delims.rep, 
                 message.delims.esc, message.delims.sub);
        display_performance_stats(&monitor);
    }
    
    Ok(())
}

fn norm_command(input: &PathBuf, canonical_delims: bool, output: &Option<PathBuf>, mllp_in: bool, mllp_out: bool, summary: bool) -> Result<(), Box<dyn std::error::Error>> {
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
            let write_time = monitor.elapsed() - read_time - parse_time - normalize_time - mllp_time;
            monitor.record_metric("File write", write_time);
            
            println!();
            println!("Normalize Summary:");
            println!("  Input file: {:?}", input);
            println!("  Output file: {:?}", output_path);
            println!("  Input size: {}", format_size(input_file_size as u64));
            println!("  Output size: {}", format_size(output_bytes.len() as u64));
            println!("  Segments: {}", segment_count);
            println!("  Canonical delimiters: {}", canonical_delims);
            println!("  MLLP output: {}", mllp_out);
            display_performance_stats(&monitor);
        }
    } else {
        std::io::stdout().write_all(&output_bytes)?;
        if summary {
            let write_time = monitor.elapsed() - read_time - parse_time - normalize_time - mllp_time;
            monitor.record_metric("Output write", write_time);
            
            println!();
            println!("Normalize Summary:");
            println!("  Input file: {:?}", input);
            println!("  Output: stdout");
            println!("  Input size: {}", format_size(input_file_size as u64));
            println!("  Output size: {}", format_size(output_bytes.len() as u64));
            println!("  Segments: {}", segment_count);
            println!("  Canonical delimiters: {}", canonical_delims);
            println!("  MLLP output: {}", mllp_out);
            display_performance_stats(&monitor);
        }
    }
    
    Ok(())
}

fn val_command(input: &PathBuf, profile: &PathBuf, mllp: bool, detailed: bool, summary: bool) -> Result<(), Box<dyn std::error::Error>> {
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
    let profile = load_profile(&profile_yaml)?;
    
    let load_profile_time = monitor.elapsed() - read_time - parse_time - read_profile_time;
    monitor.record_metric("Profile loading", load_profile_time);
    
    // Validate the message
    let results = validate(&message, &profile);
    
    let validation_time = monitor.elapsed() - read_time - parse_time - read_profile_time - load_profile_time;
    monitor.record_metric("Message validation", validation_time);
    
    // Print validation results
    if results.is_empty() {
        println!("Validation passed: No issues found");
    } else {
        if detailed {
            println!("Validation issues found:");
            for result in &results {
                println!("  - {:?}", result); // Use Debug formatting since Display isn't implemented
            }
        } else {
            println!("Validation failed: {} issues found", results.len());
        }
        std::process::exit(1);
    }
    
    // Show summary if requested
    if summary {
        println!();
        println!("Validation Summary:");
        println!("  Input file: {:?}", input);
        println!("  Profile file: {:?}", profile);
        println!("  File size: {}", format_size(file_size as u64));
        println!("  Segments: {}", message.segments.len());
        println!("  Issues found: 0");
        display_performance_stats(&monitor);
    }
    
    Ok(())
}

fn ack_command(input: &PathBuf, mode: &AckMode, code: &AckCode, mllp_in: bool, mllp_out: bool, summary: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        let write_time = monitor.elapsed() - read_time - parse_time - ack_generation_time - mllp_processing_time;
        monitor.record_metric("Output write", write_time);
        
        println!();
        println!("ACK Generation Summary:");
        println!("  Input file: {:?}", input);
        println!("  Mode: {:?}", mode);
        println!("  Code: {:?}", code);
        println!("  Input size: {}", format_size(input_file_size as u64));
        println!("  Output size: {}", format_size(ack_bytes.len() as u64));
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
        println!("Usage: parse <file> [--json] [--mllp] [--summary]");
        return Ok(());
    }
    
    let file_path = PathBuf::from(parts[1]);
    let mut json = false;
    let mut mllp = false;
    let mut summary = false;
    
    for part in &parts[2..] {
        match *part {
            "--json" => json = true,
            "--mllp" => mllp = true,
            "--summary" => summary = true,
            _ => println!("Unknown option: {}", part),
        }
    }
    
    parse_command(&file_path, json, &None, mllp, summary)
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
    
    norm_command(&file_path, canonical_delims, &None, mllp_in, mllp_out, summary)
}

/// Handle val command in interactive mode
fn handle_val_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 3 {
        println!("Usage: val <file> <profile> [--mllp] [--detailed] [--summary]");
        return Ok(());
    }
    
    let file_path = PathBuf::from(parts[1]);
    let profile_path = PathBuf::from(parts[2]);
    let mut mllp = false;
    let mut detailed = false;
    let mut summary = false;
    
    for part in &parts[3..] {
        match *part {
            "--mllp" => mllp = true,
            "--detailed" => detailed = true,
            "--summary" => summary = true,
            _ => println!("Unknown option: {}", part),
        }
    }
    
    val_command(&file_path, &profile_path, mllp, detailed, summary)
}

/// Handle ack command in interactive mode
fn handle_ack_command(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        println!("Usage: ack <file> [--mode <original|enhanced>] [--code <AA|AE|AR|CA|CE|CR>] [--mllp-in] [--mllp-out] [--summary]");
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
        println!("Usage: gen <profile> [--seed <number>] [--count <number>] [--out <directory>] [--stats]");
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

fn gen_command(profile: &PathBuf, seed: u64, count: usize, out: &PathBuf, stats: bool) -> Result<(), Box<dyn std::error::Error>> {
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
    
    let create_dir_time = monitor.elapsed() - read_template_time - parse_template_time - generation_time;
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
    
    let write_time = monitor.elapsed() - read_template_time - parse_template_time - generation_time - create_dir_time;
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
