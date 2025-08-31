//! Command-line interface for HL7 v2 processing.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use hl7v2_core::{parse, to_json, normalize, normalize_batch, normalize_file_batch};
use hl7v2_prof::{load_profile, validate};

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
    },
    
    /// Validate HL7 v2 message against profile
    Val {
        /// Input HL7 file
        input: PathBuf,
        
        /// Profile YAML file
        #[arg(long)]
        profile: PathBuf,
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
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
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
    
    match &cli.command {
        Commands::Parse { input, json, envelope } => {
            match parse_command(input, *json, envelope) {
                Ok(_) => {},
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Norm { input, canonical_delims, output } => {
            match norm_command(input, *canonical_delims, output) {
                Ok(_) => {},
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Val { input, profile } => {
            match val_command(input, profile) {
                Ok(_) => {},
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Ack { input, mode, code } => {
            println!("Generating ACK for {:?} (mode: {:?}, code: {:?})", input, mode, code);
            // Implementation will be added later
        }
        Commands::Gen { profile: _, seed, count, out } => {
            println!("Generating {} messages with seed {} into {:?}", count, seed, out);
            // Implementation will be added later
        }
    }
}

fn parse_command(input: &PathBuf, json: bool, envelope: &Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Read the input file
    let contents = fs::read(input)?;
    
    // Parse the HL7 message
    let message = parse(&contents)?;
    
    // Convert to JSON
    let json_value = to_json(&message);
    
    // Output JSON
    if json {
        println!("{}", serde_json::to_string_pretty(&json_value)?);
    } else {
        println!("{}", serde_json::to_string(&json_value)?);
    }
    
    // Handle envelope if specified
    if let Some(envelope_path) = envelope {
        // For now, we'll just print a message
        println!("Envelope would be written to: {:?}", envelope_path);
    }
    
    Ok(())
}

fn norm_command(input: &PathBuf, canonical_delims: bool, output: &Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Read the input file
    let contents = fs::read(input)?;
    
    // Try to determine the message type by looking at the first few bytes
    let normalized = if contents.starts_with(b"FHS") {
        // File batch message
        normalize_file_batch(&contents, canonical_delims)?
    } else if contents.starts_with(b"BHS") {
        // Batch message
        normalize_batch(&contents, canonical_delims)?
    } else {
        // Regular message
        normalize(&contents, canonical_delims)?
    };
    
    // Write to output file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, normalized)?;
    } else {
        std::io::stdout().write_all(&normalized)?;
    }
    
    Ok(())
}

fn val_command(input: &PathBuf, profile: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Read the HL7 message file
    let contents = fs::read(input)?;
    
    // Parse the HL7 message
    let message = parse(&contents)?;
    
    // Read the profile YAML file
    let profile_yaml = fs::read_to_string(profile)?;
    
    // Load the profile
    let profile = load_profile(&profile_yaml)?;
    
    // Validate the message against the profile
    let issues = validate(&message, &profile);
    
    // Output validation results
    if issues.is_empty() {
        println!("Validation passed: no issues found");
    } else {
        println!("Validation failed: {} issues found", issues.len());
        for issue in issues {
            let severity = match issue.severity {
                hl7v2_prof::Severity::Error => "ERROR",
                hl7v2_prof::Severity::Warning => "WARNING",
            };
            if let Some(path) = issue.path {
                println!("  [{}] {} at {}: {}", severity, issue.code, path, issue.detail);
            } else {
                println!("  [{}] {}: {}", severity, issue.code, issue.detail);
            }
        }
    }
    
    Ok(())
}