//! Clap command model and small command option helpers.

use clap::{Parser, Subcommand};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "hl7v2",
    about = "HL7 v2 parser, validator, and generator",
    version
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
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

        /// Evidence schema version for machine-readable validation reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Show summary statistics
        #[arg(long)]
        summary: bool,

        /// Write the validation report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
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

    /// Run first-use diagnostics for the CLI and local HL7 inputs
    Doctor {
        /// Optional HL7 sample file to parse instead of the built-in ADT^A01 sample
        #[arg(long)]
        sample: Option<PathBuf>,

        /// Optional profile YAML file to check for readability and load errors
        #[arg(long)]
        profile: Option<PathBuf>,

        /// Optional HTTP server URL to check, for example http://127.0.0.1:8080/health
        #[arg(long)]
        server_url: Option<String>,

        /// Output report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

        /// Evidence schema version for machine-readable doctor reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the doctor report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Print a built-in synthetic HL7 sample message
    Sample {
        /// Built-in sample message type
        #[arg(long = "type", value_enum)]
        sample_type: SampleType,

        /// Write the sample HL7 message to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Validate a built-in synthetic sample against a profile
    ValidateSample {
        /// Built-in sample message type
        #[arg(long = "type", value_enum)]
        sample_type: SampleType,

        /// Profile YAML file
        #[arg(long)]
        profile: PathBuf,

        /// Output validation report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        report: ReportFormat,

        /// Evidence schema version for machine-readable validation reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the validation report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Inspect and lint validation profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },

    /// Inspect message corpora
    Corpus {
        #[command(subcommand)]
        command: CorpusCommands,
    },

    /// Redact an HL7 v2 message using a safe-analysis policy
    Redact {
        /// Input HL7 file
        input: PathBuf,

        /// Safe-analysis policy TOML file
        #[arg(long)]
        policy: PathBuf,

        /// Output format (json or hl7)
        #[arg(long, value_enum, default_value = "json")]
        format: RedactFormat,

        /// Evidence schema version for redaction JSON output
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the redaction output to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Create a redacted support/debug evidence bundle
    Bundle {
        /// Input HL7 file
        input: PathBuf,

        /// Profile YAML file
        #[arg(long)]
        profile: PathBuf,

        /// Safe-analysis redaction policy TOML file
        #[arg(long)]
        redact_policy: PathBuf,

        /// Output bundle directory, which must not already exist
        #[arg(long)]
        out: PathBuf,

        /// Evidence schema version for the bundle summary JSON
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the bundle summary JSON to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Replay a redacted evidence bundle and verify it reproduces
    Replay {
        /// Evidence bundle directory
        bundle: PathBuf,

        /// Output replay report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

        /// Evidence schema version for machine-readable replay reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the replay report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
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

#[derive(Subcommand, Debug)]
pub(crate) enum ProfileCommands {
    /// Lint a profile YAML file
    Lint {
        /// Profile YAML file
        profile: PathBuf,

        /// Output lint report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        report: ReportFormat,

        /// Evidence schema version for machine-readable lint reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the lint report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Explain the loaded profile contract
    Explain {
        /// Profile YAML file
        profile: PathBuf,

        /// Output explain report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

        /// Evidence schema version for machine-readable explain reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the explain report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,

        /// Suppress non-error diagnostics
        #[arg(long)]
        quiet: bool,

        /// Disable colored diagnostics
        #[arg(long)]
        no_color: bool,
    },

    /// Test a profile against valid/invalid HL7 fixture directories
    Test {
        /// Profile YAML file
        profile: PathBuf,

        /// Fixture root containing valid/, invalid/, and optional expected/
        fixtures: PathBuf,

        /// Output test report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        report: ReportFormat,

        /// Evidence schema version for machine-readable test reports
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        schema_version: u8,

        /// Write the test report to a file instead of stdout
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

#[derive(Subcommand, Debug)]
pub(crate) enum CorpusCommands {
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

/// Server mode selection
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
pub(crate) enum ServerMode {
    /// HTTP server with REST API
    Http,
    /// gRPC server
    Grpc,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub(crate) enum AckMode {
    Original,
    Enhanced,
}

#[derive(clap::ValueEnum, Clone, Debug)]
#[value(rename_all = "UPPERCASE")]
pub(crate) enum AckCode {
    AA,
    AE,
    AR,
    CA,
    CE,
    CR,
}

/// Report output format
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Default)]
pub(crate) enum ReportFormat {
    #[default]
    Text,
    Json,
    Yaml,
}

/// Redacted message output format.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Default)]
pub(crate) enum RedactFormat {
    #[default]
    Json,
    Hl7,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(crate) enum SampleType {
    #[value(name = "ADT_A01", alias = "adt_a01", alias = "adt-a01")]
    AdtA01,
    #[value(name = "ORU_R01", alias = "oru_r01", alias = "oru-r01")]
    OruR01,
}

impl SampleType {
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::AdtA01 => SAMPLE_ADT_A01,
            Self::OruR01 => SAMPLE_ORU_R01,
        }
    }
}

pub(crate) struct OutputOptions<'a> {
    pub(crate) output: Option<&'a PathBuf>,
    pub(crate) quiet: bool,
    pub(crate) no_color: bool,
}

impl<'a> OutputOptions<'a> {
    pub(crate) const fn new(output: Option<&'a PathBuf>, quiet: bool, no_color: bool) -> Self {
        Self {
            output,
            quiet,
            no_color,
        }
    }

    pub(crate) fn emit(&self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _colors_enabled = !self.no_color;
        if let Some(path) = self.output {
            fs::write(path, output)?;
        } else {
            println!("{}", output);
        }
        Ok(())
    }

    pub(crate) fn emit_raw(&self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _colors_enabled = !self.no_color;
        if let Some(path) = self.output {
            fs::write(path, output)?;
        } else {
            print!("{output}");
        }
        Ok(())
    }

    pub(crate) fn diagnostic(&self, message: impl fmt::Display) {
        let _colors_enabled = !self.no_color;
        if !self.quiet {
            eprintln!("{message}");
        }
    }
}

pub(crate) struct ValCommandOptions<'a> {
    pub(crate) mllp: bool,
    pub(crate) detailed: bool,
    pub(crate) report: &'a ReportFormat,
    pub(crate) schema_version: u8,
    pub(crate) summary: bool,
}

pub(crate) const SAMPLE_ADT_A01: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01^ADT_A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";
pub(crate) const SAMPLE_ORU_R01: &str = "MSH|^~\\&|LAB|LAB|EHR|HOSP|202605030101||ORU^R01^ORU_R01|CTRL456|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\rOBR|1|ORD1|FILL1|CBC^Complete Blood Count\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";
pub(crate) const DOCTOR_BUILTIN_SAMPLE: &[u8] = SAMPLE_ADT_A01.as_bytes();
