//! Command-line interface for HL7 v2 processing.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use std::process;
use hl7v2_core::{parse, to_json, write};
use hl7v2_prof::{load_profile, validate};
use hl7v2_gen::{ack, AckCode as GenAckCode, Template, generate};

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
        Commands::Parse { input, json, envelope, mllp } => {
            parse_command(input, *json, envelope, *mllp)
        }
        Commands::Norm { input, canonical_delims, output, mllp_in, mllp_out } => {
            norm_command(input, *canonical_delims, output, *mllp_in, *mllp_out)
        }
        Commands::Val { input, profile, mllp } => {
            val_command(input, profile, *mllp)
        }
        Commands::Ack { input, mode, code, mllp_in, mllp_out } => {
            ack_command(input, mode, code, *mllp_in, *mllp_out)
        }
        Commands::Gen { profile, seed, count, out } => {
            gen_command(profile, *seed, *count, out)
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn parse_command(input: &PathBuf, json: bool, envelope: &Option<PathBuf>, mllp: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Read the input file
    let contents = fs::read(input)?;
    
    // Parse the HL7 message
    let message = if mllp {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };
    
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

fn norm_command(input: &PathBuf, canonical_delims: bool, output: &Option<PathBuf>, mllp_in: bool, mllp_out: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Read the input file
    let contents = fs::read(input)?;
    
    // Parse the HL7 message
    let message = if mllp_in {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };
    
    // Normalize the message
    let normalized_bytes = if canonical_delims {
        // We need to implement normalization with canonical delimiters
        // For now, we'll just use the regular write function
        write(&message)
    } else {
        write(&message)
    };
    
    // Add MLLP framing if requested
    let output_bytes = if mllp_out {
        hl7v2_core::wrap_mllp(&normalized_bytes)
    } else {
        normalized_bytes
    };
    
    // Write to output file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, output_bytes)?;
    } else {
        std::io::stdout().write_all(&output_bytes)?;
    }
    
    Ok(())
}

fn val_command(input: &PathBuf, profile: &PathBuf, mllp: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Read the HL7 message file
    let contents = fs::read(input)?;
    
    // Parse the HL7 message
    let message = if mllp {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };
    
    // Read the profile YAML file
    let profile_yaml = fs::read_to_string(profile)?;
    
    // Load the profile
    let profile = load_profile(&profile_yaml)?;
    
    // Validate the message
    let results = validate(&message, &profile);
    
    // Print validation results
    if results.is_empty() {
        println!("Validation passed: No issues found");
    } else {
        println!("Validation issues found:");
        for result in results {
            println!("  - {:?}", result); // Use Debug formatting since Display isn't implemented
        }
        process::exit(1);
    }
    
    Ok(())
}

fn ack_command(input: &PathBuf, _mode: &AckMode, code: &AckCode, mllp_in: bool, mllp_out: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Read the HL7 message file
    let contents = fs::read(input)?;
    
    // Parse the HL7 message
    let message = if mllp_in {
        hl7v2_core::parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };
    
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
    
    // Write ACK message
    let ack_bytes = if mllp_out {
        hl7v2_core::write_mllp(&ack_message)
    } else {
        write(&ack_message)
    };
    
    std::io::stdout().write_all(&ack_bytes)?;
    
    Ok(())
}

fn gen_command(profile: &PathBuf, seed: u64, count: usize, out: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Read the template YAML file
    let template_yaml = fs::read_to_string(profile)?;
    
    // Parse the template from YAML
    let template: Template = serde_yaml::from_str(&template_yaml)?;
    
    // Generate messages
    let messages = generate(&template, seed, count)?;
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(out)?;
    
    // Write each message to a separate file
    for (i, message) in messages.iter().enumerate() {
        let filename = out.join(format!("message_{:03}.hl7", i + 1));
        let message_bytes = write(message);
        fs::write(&filename, &message_bytes)?;
        println!("Generated message written to: {:?}", filename);
    }
    
    println!("Successfully generated {} messages", messages.len());
    Ok(())
}