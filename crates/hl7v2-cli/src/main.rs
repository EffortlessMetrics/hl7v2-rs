//! Command-line interface for HL7 v2 processing.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
            println!("Parsing {:?} (json: {}, envelope: {:?})", input, json, envelope);
            // Implementation will be added later
        }
        Commands::Norm { input, canonical_delims, output } => {
            println!("Normalizing {:?} (canonical_delims: {:?})", input, canonical_delims);
            // Implementation will be added later
        }
        Commands::Val { input, profile } => {
            println!("Validating {:?} against {:?}", input, profile);
            // Implementation will be added later
        }
        Commands::Ack { input, mode, code } => {
            println!("Generating ACK for {:?} (mode: {:?}, code: {:?})", input, mode, code);
            // Implementation will be added later
        }
        Commands::Gen { profile, seed, count, out } => {
            println!("Generating {} messages with seed {} into {:?}", count, seed, out);
            // Implementation will be added later
        }
    }
}