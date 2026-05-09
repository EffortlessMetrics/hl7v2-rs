//! Command-line interface for HL7 v2 processing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::cast_precision_loss,
    clippy::indexing_slicing,
    clippy::unchecked_time_subtraction,
    clippy::uninlined_format_args,
    clippy::unnecessary_debug_formatting,
    clippy::unwrap_used,
    reason = "pre-existing CLI reporting and table-rendering debt is tracked in policy/clippy-debt.toml"
)]
#![cfg_attr(
    test,
    expect(
        clippy::expect_used,
        reason = "pre-existing CLI config tests use static fixture expects; cleanup is tracked in policy/clippy-debt.toml"
    )
)]

use clap::{Parser, Subcommand};
use hl7v2::synthetic::corpus::{
    CorpusCount, CorpusCountDiff, CorpusDiffReport, CorpusFieldCardinalityDiff,
    CorpusFieldPresenceDiff, CorpusFingerprint, CorpusFingerprintProfile, CorpusSummary,
    CorpusValueShapeStatsDiff, compute_sha256, diff_corpus_fingerprints, diff_corpus_paths,
    fingerprint_corpus_path, summarize_corpus_path,
};
use hl7v2::synthetic::generate::{Template, generate};
use hl7v2::{
    AckCode as GenAckCode, Atom, Event, Field, Message, Profile, ProfileLintIssue,
    ProfileLintReport, StreamParser, ValidationReport, ack, get, is_mllp_framed, lint_profile_yaml,
    load_profile, load_profile_checked, normalize, parse, parse_mllp, to_json, validate, wrap_mllp,
    write, write_mllp,
};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::time::Duration;
mod config;
mod monitor;

mod serve;
#[cfg(test)]
mod tests;

const EXIT_CHECK_FAILED: i32 = 1;
const EXIT_INPUT_ERROR: i32 = 2;
const EXIT_RUNTIME_ERROR: i32 = 3;

#[derive(Debug)]
struct CliFailure {
    code: i32,
    message: String,
}

impl CliFailure {
    fn check_failed(message: impl Into<String>) -> Box<dyn std::error::Error> {
        Box::new(Self {
            code: EXIT_CHECK_FAILED,
            message: message.into(),
        })
    }
}

impl fmt::Display for CliFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliFailure {}

fn classify_cli_error(error: &(dyn std::error::Error + 'static)) -> i32 {
    if let Some(failure) = error.downcast_ref::<CliFailure>() {
        failure.code
    } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
        match error.kind() {
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::Other => EXIT_INPUT_ERROR,
            _ => EXIT_RUNTIME_ERROR,
        }
    } else {
        EXIT_INPUT_ERROR
    }
}

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
enum ProfileCommands {
    /// Lint a profile YAML file
    Lint {
        /// Profile YAML file
        profile: PathBuf,

        /// Output lint report format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        report: ReportFormat,

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
enum CorpusCommands {
    /// Summarize a directory or file corpus of HL7 messages
    Summarize {
        /// Corpus directory or single HL7 file
        path: PathBuf,

        /// Output summary format (json, yaml, text)
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,

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

/// Redacted message output format.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Default)]
enum RedactFormat {
    #[default]
    Json,
    Hl7,
}

struct OutputOptions<'a> {
    output: Option<&'a PathBuf>,
    quiet: bool,
    no_color: bool,
}

impl<'a> OutputOptions<'a> {
    const fn new(output: Option<&'a PathBuf>, quiet: bool, no_color: bool) -> Self {
        Self {
            output,
            quiet,
            no_color,
        }
    }

    fn emit(&self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _colors_enabled = !self.no_color;
        if let Some(path) = self.output {
            fs::write(path, output)?;
        } else {
            println!("{}", output);
        }
        Ok(())
    }

    fn emit_raw(&self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _colors_enabled = !self.no_color;
        if let Some(path) = self.output {
            fs::write(path, output)?;
        } else {
            print!("{output}");
        }
        Ok(())
    }

    fn diagnostic(&self, message: impl fmt::Display) {
        let _colors_enabled = !self.no_color;
        if !self.quiet {
            eprintln!("{message}");
        }
    }
}

const DOCTOR_BUILTIN_SAMPLE: &[u8] = b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||123456^^^HOSP^MR||Doe^John||19700101|M\r";

#[derive(serde::Serialize)]
struct DoctorReport {
    version: String,
    checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    fn has_errors(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Error)
    }
}

#[derive(serde::Serialize)]
struct DoctorCheck {
    name: String,
    status: DoctorStatus,
    message: String,
}

#[derive(Clone, Copy, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum DoctorStatus {
    Ok,
    Warn,
    Error,
}

#[derive(serde::Serialize)]
struct ProfileExplainReport {
    profile: String,
    profile_sha256: String,
    message_structure: String,
    version: String,
    message_type: Option<String>,
    parent: Option<String>,
    summary: ProfileExplainSummary,
    segments: Vec<ProfileExplainSegment>,
    required_fields: Vec<ProfileExplainRequiredField>,
    field_constraints: Vec<ProfileExplainConstraint>,
    length_rules: Vec<ProfileExplainLengthRule>,
    datatype_rules: Vec<ProfileExplainDatatypeRule>,
    value_sets: Vec<ProfileExplainValueSet>,
    rules: ProfileExplainRules,
    hl7_tables: Vec<ProfileExplainTable>,
    table_precedence: Vec<String>,
    expression_guardrails: ProfileExplainExpressionGuardrails,
    lint: ProfileExplainLintSummary,
}

#[derive(serde::Serialize)]
struct ProfileExplainSummary {
    segment_count: usize,
    required_field_count: usize,
    field_constraint_count: usize,
    length_rule_count: usize,
    datatype_rule_count: usize,
    advanced_datatype_rule_count: usize,
    value_set_count: usize,
    cross_field_rule_count: usize,
    temporal_rule_count: usize,
    contextual_rule_count: usize,
    custom_rule_count: usize,
    hl7_table_count: usize,
}

#[derive(serde::Serialize)]
struct ProfileExplainSegment {
    id: String,
}

#[derive(serde::Serialize)]
struct ProfileExplainRequiredField {
    path: String,
    conditional: bool,
}

#[derive(serde::Serialize)]
struct ProfileExplainConstraint {
    path: String,
    required: bool,
    conditional: bool,
    component_min: Option<usize>,
    component_max: Option<usize>,
    allowed_value_count: usize,
    allowed_values: Vec<String>,
    pattern: Option<String>,
}

#[derive(serde::Serialize)]
struct ProfileExplainLengthRule {
    path: String,
    max: Option<usize>,
    policy: Option<String>,
}

#[derive(serde::Serialize)]
struct ProfileExplainDatatypeRule {
    path: String,
    datatype: String,
    kind: &'static str,
    pattern: Option<String>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    format: Option<String>,
    checksum: Option<String>,
}

#[derive(serde::Serialize)]
struct ProfileExplainValueSet {
    name: String,
    path: String,
    source: &'static str,
    inline_code_count: usize,
    table_code_count: usize,
}

#[derive(serde::Serialize)]
struct ProfileExplainRules {
    cross_field: Vec<ProfileExplainRule>,
    temporal: Vec<ProfileExplainRule>,
    contextual: Vec<ProfileExplainRule>,
    custom: Vec<ProfileExplainRule>,
}

#[derive(serde::Serialize)]
struct ProfileExplainRule {
    id: String,
    description: String,
}

#[derive(serde::Serialize)]
struct ProfileExplainTable {
    id: String,
    name: String,
    version: String,
    code_count: usize,
}

#[derive(serde::Serialize)]
struct ProfileExplainExpressionGuardrails {
    max_depth: Option<usize>,
    max_length: Option<usize>,
    allow_custom_scripts: bool,
}

#[derive(serde::Serialize)]
struct ProfileExplainLintSummary {
    valid: bool,
    error_count: usize,
    warning_count: usize,
    issue_count: usize,
    ignored_or_unsupported: Vec<ProfileLintIssue>,
}

#[derive(serde::Serialize)]
struct ProfileTestReport {
    profile: String,
    fixtures: String,
    valid: bool,
    case_count: usize,
    passed_count: usize,
    failed_count: usize,
    cases: Vec<ProfileTestCaseReport>,
}

#[derive(serde::Serialize)]
struct ProfileTestCaseReport {
    name: String,
    path: String,
    expectation: ProfileFixtureExpectation,
    passed: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_report: Option<ValidationReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_report: Option<ExpectedReportComparison>,
}

#[derive(Clone, Copy, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum ProfileFixtureExpectation {
    Valid,
    Invalid,
}

impl ProfileFixtureExpectation {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(serde::Serialize)]
struct ExpectedReportComparison {
    path: String,
    matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

enum ExpectedReportCandidate {
    File(PathBuf),
    Ambiguous(PathBuf),
}

#[derive(serde::Deserialize)]
struct SafeAnalysisPolicy {
    rules: Vec<SafeAnalysisPolicyRule>,
}

#[derive(serde::Deserialize)]
struct SafeAnalysisPolicyRule {
    path: String,
    action: RedactionAction,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    optional: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum RedactionAction {
    Hash,
    Drop,
    Retain,
}

#[derive(serde::Serialize)]
struct RedactionOutput {
    input_sha256: String,
    policy_sha256: String,
    message_type: String,
    redacted_hl7: String,
    receipt: RedactionReceipt,
}

#[derive(serde::Serialize)]
struct RedactionReceipt {
    phi_removed: bool,
    hash_algorithm: &'static str,
    actions: Vec<RedactionActionReceipt>,
}

#[derive(serde::Serialize)]
struct RedactionActionReceipt {
    path: String,
    action: RedactionAction,
    reason: String,
    matched_count: usize,
    optional: bool,
    status: RedactionActionStatus,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum RedactionActionStatus {
    Applied,
    Retained,
    NotFound,
}

#[derive(serde::Serialize)]
struct EvidenceBundleSummary {
    bundle_version: &'static str,
    output_dir: String,
    message_type: String,
    validation_valid: bool,
    validation_issue_count: usize,
    redaction_phi_removed: bool,
    artifacts: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct EvidenceBundleManifest {
    bundle_version: String,
    tool_name: String,
    tool_version: String,
    artifacts: Vec<EvidenceBundleManifestArtifact>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct EvidenceBundleManifestArtifact {
    path: String,
    role: String,
    sha256: String,
}

#[derive(serde::Serialize)]
struct EvidenceBundleEnvironment {
    bundle_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    message_type: String,
    input_sha256: String,
    profile_sha256: String,
    redaction_policy_sha256: String,
    validation_valid: bool,
    validation_issue_count: usize,
    replay_command: &'static str,
}

#[derive(serde::Serialize)]
struct EvidenceReplayReport {
    replay_version: &'static str,
    bundle_version: Option<String>,
    tool_name: &'static str,
    tool_version: &'static str,
    message_type: Option<String>,
    reproduced: bool,
    validation_valid: Option<bool>,
    validation_issue_count: Option<usize>,
    checks: Vec<EvidenceReplayCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_report: Option<ValidationReport>,
}

#[derive(serde::Serialize)]
struct EvidenceReplayCheck {
    name: &'static str,
    status: EvidenceReplayCheckStatus,
    message: String,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceReplayCheckStatus {
    Pass,
    Fail,
}

#[derive(serde::Serialize)]
struct FieldPathTraceReport {
    message_type: String,
    field_count: usize,
    fields: Vec<FieldPathTrace>,
}

#[derive(serde::Serialize)]
struct FieldPathTrace {
    path: String,
    canonical_path: String,
    segment_index: usize,
    field_index: usize,
    present: bool,
    value_shape: FieldValueShape,
    #[serde(skip_serializing_if = "Option::is_none")]
    redaction_action: Option<RedactionAction>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum FieldValueShape {
    Empty,
    Present,
    HashedSha256,
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
            output,
            quiet,
            no_color,
        } => val_command(
            input,
            profile,
            *mllp,
            *detailed,
            report,
            *summary,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Stats {
            input,
            mllp,
            distributions,
            format,
        } => stats_command(input, *mllp, *distributions, format),
        Commands::Doctor {
            sample,
            profile,
            server_url,
            format,
        } => doctor_command(
            sample.as_ref(),
            profile.as_ref(),
            server_url.as_deref(),
            format,
        ),
        Commands::Profile { command } => match command {
            ProfileCommands::Lint {
                profile,
                report,
                output,
                quiet,
                no_color,
            } => profile_lint_command(
                profile,
                report,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            ProfileCommands::Explain {
                profile,
                format,
                output,
                quiet,
                no_color,
            } => profile_explain_command(
                profile,
                format,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            ProfileCommands::Test {
                profile,
                fixtures,
                report,
                output,
                quiet,
                no_color,
            } => profile_test_command(
                profile,
                fixtures,
                report,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
        },
        Commands::Corpus { command } => match command {
            CorpusCommands::Summarize {
                path,
                format,
                output,
                quiet,
                no_color,
            } => corpus_summarize_command(
                path,
                format,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            CorpusCommands::Fingerprint {
                path,
                profile,
                format,
                output,
                quiet,
                no_color,
            } => corpus_fingerprint_command(
                path,
                profile.as_ref(),
                format,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            CorpusCommands::Diff {
                before,
                after,
                profile,
                format,
                output,
                quiet,
                no_color,
            } => corpus_diff_command(
                before,
                after,
                profile.as_ref(),
                format,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
        },
        Commands::Redact {
            input,
            policy,
            format,
            output,
            quiet,
            no_color,
        } => redact_command(
            input,
            policy,
            format,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Bundle {
            input,
            profile,
            redact_policy,
            out,
            output,
            quiet,
            no_color,
        } => bundle_command(
            input,
            profile,
            redact_policy,
            out,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Replay {
            bundle,
            format,
            output,
            quiet,
            no_color,
        } => replay_command(
            bundle,
            format,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
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
        process::exit(classify_cli_error(e.as_ref()));
    }
}

/// Display performance statistics
fn display_performance_stats(monitor: &monitor::PerformanceMonitor) {
    print!("{}", format_performance_stats(monitor));
}

fn format_performance_stats(monitor: &monitor::PerformanceMonitor) -> String {
    let mut output = String::new();
    output.push('\n');
    output.push_str("Performance Statistics:\n");
    output.push_str(&format!(
        "  Total execution time: {:?}\n",
        monitor.elapsed()
    ));

    let metrics = monitor.get_metrics();
    if !metrics.is_empty() {
        output.push_str("  Detailed metrics:\n");
        for (name, duration) in metrics {
            output.push_str(&format!("    {name}: {duration:?}\n"));
        }
    }

    // System information
    let system_info = monitor::get_system_info();
    output.push_str("  System information:\n");
    if let Some(cpu_usage) = system_info.cpu.cpu_usage_percent {
        output.push_str(&format!("    CPU usage: {cpu_usage:.2}%\n"));
    }
    output.push_str(&format!(
        "    Total memory: {} bytes\n",
        system_info.total_memory
    ));
    output.push_str(&format!(
        "    Used memory: {} bytes\n",
        system_info.used_memory
    ));
    if let Some(rss) = system_info.memory.resident_set_size {
        output.push_str(&format!("    Process memory (RSS): {rss} bytes\n"));
    }
    if let Some(vms) = system_info.memory.virtual_memory_size {
        output.push_str(&format!("    Process memory (VMS): {vms} bytes\n"));
    }

    output
}

fn doctor_command(
    sample: Option<&PathBuf>,
    profile: Option<&PathBuf>,
    server_url: Option<&str>,
    format: &ReportFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut report = DoctorReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks: Vec::new(),
    };

    report.checks.push(DoctorCheck {
        name: "cli-version".to_string(),
        status: DoctorStatus::Ok,
        message: format!("hl7v2-cli {}", env!("CARGO_PKG_VERSION")),
    });

    add_sample_checks(&mut report, sample);
    add_profile_check(&mut report, profile);
    add_server_check(&mut report, server_url);
    add_python_check(&mut report);

    let output = format_doctor_report(&report, format)?;
    println!("{}", output);

    if report.has_errors() {
        return Err(CliFailure::check_failed("doctor reported failed checks"));
    }

    Ok(())
}

fn add_sample_checks(report: &mut DoctorReport, sample: Option<&PathBuf>) {
    let (source, bytes) = match sample {
        Some(path) => match fs::read(path) {
            Ok(contents) => (path.to_string_lossy().to_string(), contents),
            Err(err) => {
                report.checks.push(DoctorCheck {
                    name: "sample-read".to_string(),
                    status: DoctorStatus::Error,
                    message: format!("failed to read sample file {}: {}", path.display(), err),
                });
                return;
            }
        },
        None => (
            "built-in ADT_A01 sample".to_string(),
            DOCTOR_BUILTIN_SAMPLE.to_vec(),
        ),
    };

    add_sample_byte_diagnostics(report, &source, &bytes);

    let parse_result = if is_mllp_framed(&bytes) {
        parse_mllp(&bytes)
    } else {
        parse(&bytes)
    };

    match parse_result {
        Ok(message) => {
            let message_type = get(&message, "MSH.9").unwrap_or("UNKNOWN");
            report.checks.push(DoctorCheck {
                name: "sample-parse".to_string(),
                status: DoctorStatus::Ok,
                message: format!(
                    "{} parsed as {} with {} segment(s)",
                    source,
                    message_type,
                    message.segments.len()
                ),
            });
        }
        Err(err) => report.checks.push(DoctorCheck {
            name: "sample-parse".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} failed to parse: {}", source, err),
        }),
    }

    let framed = wrap_mllp(DOCTOR_BUILTIN_SAMPLE);
    match parse_mllp(&framed) {
        Ok(message) => report.checks.push(DoctorCheck {
            name: "mllp-roundtrip".to_string(),
            status: DoctorStatus::Ok,
            message: format!(
                "built-in MLLP framing parsed with {} segment(s)",
                message.segments.len()
            ),
        }),
        Err(err) => report.checks.push(DoctorCheck {
            name: "mllp-roundtrip".to_string(),
            status: DoctorStatus::Error,
            message: format!("built-in MLLP framing failed: {}", err),
        }),
    }
}

fn add_sample_byte_diagnostics(report: &mut DoctorReport, source: &str, bytes: &[u8]) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        report.checks.push(DoctorCheck {
            name: "sample-encoding".to_string(),
            status: DoctorStatus::Warn,
            message: format!(
                "{} starts with a UTF-8 BOM; remove it before parsing feeds",
                source
            ),
        });
    }

    if bytes.contains(&b'\n') && !bytes.contains(&b'\r') {
        report.checks.push(DoctorCheck {
            name: "sample-newlines".to_string(),
            status: DoctorStatus::Warn,
            message: format!(
                "{} uses LF without CR; HL7 segment separators are normally CR",
                source
            ),
        });
    }

    if bytes.first() == Some(&0x0B) && !is_mllp_framed(bytes) {
        report.checks.push(DoctorCheck {
            name: "sample-mllp-framing".to_string(),
            status: DoctorStatus::Error,
            message: format!(
                "{} starts with an MLLP start byte but is missing a complete end frame",
                source
            ),
        });
    } else if is_mllp_framed(bytes) {
        report.checks.push(DoctorCheck {
            name: "sample-mllp-framing".to_string(),
            status: DoctorStatus::Ok,
            message: format!("{} is complete MLLP-framed input", source),
        });
    }
}

fn add_profile_check(report: &mut DoctorReport, profile: Option<&PathBuf>) {
    let Some(path) = profile else {
        report.checks.push(DoctorCheck {
            name: "profile".to_string(),
            status: DoctorStatus::Warn,
            message: "no --profile provided; skipping profile load diagnostics".to_string(),
        });
        return;
    };

    match fs::read_to_string(path) {
        Ok(yaml) => match load_profile_checked(&yaml) {
            Ok(profile) => report.checks.push(DoctorCheck {
                name: "profile".to_string(),
                status: DoctorStatus::Ok,
                message: format!(
                    "{} loaded as {} {} with {} segment spec(s)",
                    path.display(),
                    profile.message_structure,
                    profile.version,
                    profile.segments.len()
                ),
            }),
            Err(err) => report.checks.push(DoctorCheck {
                name: "profile".to_string(),
                status: DoctorStatus::Error,
                message: format!("{} failed to load as a profile: {}", path.display(), err),
            }),
        },
        Err(err) => report.checks.push(DoctorCheck {
            name: "profile".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} is not readable: {}", path.display(), err),
        }),
    }
}

fn add_server_check(report: &mut DoctorReport, server_url: Option<&str>) {
    let Some(url) = server_url else {
        report.checks.push(DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Warn,
            message: "no --server-url provided; skipping HTTP health reachability".to_string(),
        });
        return;
    };

    report.checks.push(check_http_health(url));
}

fn check_http_health(url: &str) -> DoctorCheck {
    let Some(endpoint) = parse_http_endpoint(url) else {
        return DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!(
                "{} is not a supported HTTP URL; use http://host:port[/health]",
                url
            ),
        };
    };

    let mut addrs = match (endpoint.host.as_str(), endpoint.port).to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(err) => {
            return DoctorCheck {
                name: "server".to_string(),
                status: DoctorStatus::Error,
                message: format!("{} could not resolve: {}", url, err),
            };
        }
    };

    let Some(addr) = addrs.next() else {
        return DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} did not resolve to a socket address", url),
        };
    };

    let timeout = Duration::from_secs(2);
    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(stream) => stream,
        Err(err) => {
            return DoctorCheck {
                name: "server".to_string(),
                status: DoctorStatus::Error,
                message: format!("{} is not reachable: {}", url, err),
            };
        }
    };

    if let Err(err) = stream.set_read_timeout(Some(timeout)) {
        return DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} connected but read timeout setup failed: {}", url, err),
        };
    }
    if let Err(err) = stream.set_write_timeout(Some(timeout)) {
        return DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} connected but write timeout setup failed: {}", url, err),
        };
    }

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        endpoint.path, endpoint.host
    );
    if let Err(err) = stream.write_all(request.as_bytes()) {
        return DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} accepted TCP but HTTP request failed: {}", url, err),
        };
    }

    let mut response = String::new();
    if let Err(err) = stream.read_to_string(&mut response) {
        return DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} did not return a readable HTTP response: {}", url, err),
        };
    }

    if response.starts_with("HTTP/1.1 2") || response.starts_with("HTTP/1.0 2") {
        DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Ok,
            message: format!("{} returned a 2xx health response", url),
        }
    } else {
        let status_line = response.lines().next().unwrap_or("empty response");
        DoctorCheck {
            name: "server".to_string(),
            status: DoctorStatus::Error,
            message: format!("{} returned {}", url, status_line),
        }
    }
}

struct HttpEndpoint {
    host: String,
    port: u16,
    path: String,
}

fn parse_http_endpoint(url: &str) -> Option<HttpEndpoint> {
    let rest = url.strip_prefix("http://")?;
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{}", path)),
        None => (rest, "/health".to_string()),
    };

    if authority.is_empty() {
        return None;
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let parsed_port = port.parse::<u16>().ok()?;
            (host.to_string(), parsed_port)
        }
        Some(_) => return None,
        None => (authority.to_string(), 80),
    };

    Some(HttpEndpoint { host, port, path })
}

fn add_python_check(report: &mut DoctorReport) {
    let output = std::process::Command::new("python")
        .args(["-c", "import hl7v2; print(hl7v2.__version__)"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let message = if version.is_empty() {
                "Python module hl7v2 imports successfully".to_string()
            } else {
                format!("Python module hl7v2 imports successfully as {}", version)
            };
            report.checks.push(DoctorCheck {
                name: "python-binding".to_string(),
                status: DoctorStatus::Ok,
                message,
            });
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                "Python module hl7v2 is not importable via python".to_string()
            } else {
                let summary = stderr
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or(stderr.as_str());
                format!(
                    "Python module hl7v2 is not importable via python: {}",
                    summary.trim()
                )
            };
            report.checks.push(DoctorCheck {
                name: "python-binding".to_string(),
                status: DoctorStatus::Warn,
                message,
            });
        }
        Err(err) => report.checks.push(DoctorCheck {
            name: "python-binding".to_string(),
            status: DoctorStatus::Warn,
            message: format!(
                "python executable was not available for binding check: {}",
                err
            ),
        }),
    }
}

fn format_doctor_report(
    report: &DoctorReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("HL7v2 Doctor\n");
            output.push_str(&format!("  Version: {}\n\n", report.version));
            for check in &report.checks {
                output.push_str(&format!(
                    "[{}] {}: {}\n",
                    doctor_status_label(check.status),
                    check.name,
                    check.message
                ));
            }
            Ok(output)
        }
    }
}

fn doctor_status_label(status: DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Ok => "ok",
        DoctorStatus::Warn => "warn",
        DoctorStatus::Error => "error",
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

    if streaming {
        let file = fs::File::open(input)?;
        let reader = std::io::BufReader::new(file);
        let mut parser = StreamParser::new(reader);
        let mut message_count = 0;
        let mut event_count = 0;

        while let Ok(Some(event)) = parser.next_event() {
            event_count += 1;
            if matches!(event, Event::StartMessage { .. }) {
                message_count += 1;
            }

            if json {
                let event_json = match &event {
                    Event::StartMessage { delims } => serde_json::json!({
                        "event": "start_message",
                        "delims": {
                            "field": delims.field.to_string(),
                            "comp": delims.comp.to_string(),
                            "rep": delims.rep.to_string(),
                            "esc": delims.esc.to_string(),
                            "sub": delims.sub.to_string(),
                        }
                    }),
                    Event::Segment { id } => serde_json::json!({
                        "event": "segment",
                        "id": String::from_utf8_lossy(id)
                    }),
                    Event::Field { num, raw } => serde_json::json!({
                        "event": "field",
                        "num": num,
                        "raw": String::from_utf8_lossy(raw)
                    }),
                    Event::EndMessage => serde_json::json!({
                        "event": "end_message"
                    }),
                };
                println!("{}", serde_json::to_string(&event_json)?);
            } else {
                match event {
                    Event::StartMessage { delims } => println!(
                        "--- Message {} Start (delims: {:?}) ---",
                        message_count, delims
                    ),
                    Event::Segment { id } => println!("Segment: {}", String::from_utf8_lossy(&id)),
                    Event::Field { num, raw } => {
                        println!("  Field {}: {}", num, String::from_utf8_lossy(&raw));
                    }
                    Event::EndMessage => println!("--- Message End ---"),
                }
            }
        }

        if summary {
            println!("\nStreaming Parse Summary:");
            println!("  Input file: {:?}", input);
            println!("  Messages: {}", message_count);
            println!("  Total events: {}", event_count);
            display_performance_stats(&monitor);
        }
        return Ok(());
    }

    // Read the input file
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

    // Count segments
    let segment_count = message.segments.len();

    // Handle output based on flags
    if canonical_delims {
        // Output with canonical delimiters (|^~\&)
        // Normalize the raw bytes with canonical delimiters
        let original_bytes = write(&message);
        let output_bytes = normalize(&original_bytes, true)?;

        if envelope {
            // Wrap in MLLP envelope
            let mllp_bytes = wrap_mllp(&output_bytes);
            std::io::stdout().write_all(&mllp_bytes)?;
        } else {
            std::io::stdout().write_all(&output_bytes)?;
        }
    } else if envelope {
        // Output with original delimiters but wrapped in MLLP envelope
        let output_bytes = write(&message);
        let mllp_bytes = wrap_mllp(&output_bytes);
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
        parse_mllp(&contents)?
    } else {
        parse(&contents)?
    };

    let parse_time = monitor.elapsed() - read_time;
    monitor.record_metric("Message parsing", parse_time);

    // Count segments before normalization
    let segment_count = message.segments.len();

    // Normalize the message
    let original_bytes = write(&message);
    let normalized_bytes = if canonical_delims {
        // Use core normalization for canonical delimiters
        normalize(&original_bytes, true)?
    } else {
        original_bytes
    };

    let normalize_time = monitor.elapsed() - read_time - parse_time;
    monitor.record_metric("Message normalization", normalize_time);

    // Add MLLP framing if requested
    let output_bytes = if mllp_out {
        wrap_mllp(&normalized_bytes)
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
    output_options: &OutputOptions<'_>,
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
    let validation_report = ValidationReport::from_issues(
        &message,
        Some(profile.to_string_lossy().to_string()),
        results,
    );

    let output = match report {
        ReportFormat::Json => serde_json::to_string_pretty(&validation_report)?,
        ReportFormat::Yaml => serde_yaml::to_string(&validation_report)?,
        ReportFormat::Text => {
            // Print validation results in text format
            if validation_report.valid {
                "Validation passed: No issues found".to_string()
            } else if detailed {
                let mut output = String::from("Validation issues found:");
                for issue in &validation_report.issues {
                    let path = issue.path.as_deref().unwrap_or("message");
                    output.push_str(&format!(
                        "\n  - {} {} {}: {}",
                        issue.severity.as_str(),
                        issue.code,
                        path,
                        issue.message
                    ));
                }
                output
            } else {
                format!(
                    "Validation failed: {} issues found",
                    validation_report.issue_count
                )
            }
        }
    };
    output_options.emit(&output)?;

    // Show summary if requested (only for text format to avoid mixing output)
    if summary && *report == ReportFormat::Text {
        let mut summary_output = String::new();
        summary_output.push('\n');
        summary_output.push_str("Validation Summary:\n");
        summary_output.push_str(&format!("  Input file: {:?}\n", input));
        summary_output.push_str(&format!("  Profile file: {:?}\n", profile));
        summary_output.push_str(&format!("  File size: {file_size} bytes\n"));
        summary_output.push_str(&format!(
            "  Segments: {}\n",
            validation_report.segment_count
        ));
        summary_output.push_str(&format!(
            "  Issues found: {}\n",
            validation_report.issue_count
        ));
        summary_output.push_str(&format_performance_stats(&monitor));

        if output_options.output.is_some() || output_options.quiet {
            output_options.diagnostic(summary_output.trim_end());
        } else {
            print!("{summary_output}");
        }
    }

    // Exit with error code if validation failed
    if !validation_report.valid {
        return Err(CliFailure::check_failed("validation failed"));
    }

    Ok(())
}

fn redact_command(
    input: &Path,
    policy: &Path,
    format: &RedactFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read(input)?;
    let mut message = parse(&contents)?;
    let policy_text = fs::read_to_string(policy)?;
    let redaction_policy = load_safe_analysis_policy(&policy_text)?;
    let receipt = apply_safe_analysis_policy(&mut message, &redaction_policy)?;
    let redacted_hl7 = String::from_utf8(write(&message))?;

    match format {
        RedactFormat::Json => {
            let output = RedactionOutput {
                input_sha256: compute_sha256(&String::from_utf8_lossy(&contents)),
                policy_sha256: compute_sha256(&policy_text),
                message_type: message_field_text(&message, "MSH", 9)
                    .unwrap_or_else(|| "unknown".to_string()),
                redacted_hl7,
                receipt,
            };
            output_options.emit(&serde_json::to_string_pretty(&output)?)?;
        }
        RedactFormat::Hl7 => {
            output_options.diagnostic(format!(
                "Redaction receipt: {} action(s), PHI removed: {}",
                receipt.actions.len(),
                receipt.phi_removed
            ));
            output_options.emit_raw(&redacted_hl7)?;
        }
    }

    Ok(())
}

fn bundle_command(
    input: &Path,
    profile: &Path,
    redact_policy: &Path,
    out: &Path,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if out.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("bundle output directory already exists: {}", out.display()),
        )
        .into());
    }

    let contents = fs::read(input)?;
    let message = parse(&contents)?;
    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile_checked(&profile_yaml)?;
    let policy_text = fs::read_to_string(redact_policy)?;
    let redaction_policy = load_safe_analysis_policy(&policy_text)?;

    let mut redacted_message = message.clone();
    let redaction_receipt = apply_safe_analysis_policy(&mut redacted_message, &redaction_policy)?;
    let redacted_hl7 = String::from_utf8(write(&redacted_message))?;
    let field_trace = build_field_path_trace(&redacted_message, &redaction_receipt);
    let validation_report = ValidationReport::from_issues(
        &redacted_message,
        Some("profile.yaml".to_string()),
        validate(&redacted_message, &loaded_profile),
    );
    let message_type = message_field_text(&message, "MSH", 9).unwrap_or_else(|| "unknown".into());
    let environment = EvidenceBundleEnvironment {
        bundle_version: "1",
        tool_name: "hl7v2-cli",
        tool_version: env!("CARGO_PKG_VERSION"),
        message_type: message_type.clone(),
        input_sha256: compute_sha256(&String::from_utf8_lossy(&contents)),
        profile_sha256: compute_sha256(&profile_yaml),
        redaction_policy_sha256: compute_sha256(&policy_text),
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        replay_command: "hl7v2 val message.redacted.hl7 --profile profile.yaml --report json",
    };

    fs::create_dir(out)?;
    fs::write(out.join("message.redacted.hl7"), redacted_hl7)?;
    fs::write(out.join("profile.yaml"), profile_yaml)?;
    write_json_file(&out.join("validation-report.json"), &validation_report)?;
    write_json_file(&out.join("redaction-receipt.json"), &redaction_receipt)?;
    write_json_file(&out.join("field-paths.json"), &field_trace)?;
    write_json_file(&out.join("environment.json"), &environment)?;
    fs::write(out.join("replay.sh"), replay_shell_script())?;
    fs::write(out.join("replay.ps1"), replay_powershell_script())?;

    let artifact_specs = bundle_artifact_specs();
    let manifest = EvidenceBundleManifest {
        bundle_version: "1".to_string(),
        tool_name: "hl7v2-cli".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        artifacts: artifact_specs
            .iter()
            .map(|(path, role)| bundle_manifest_artifact(out, path, role))
            .collect::<Result<_, _>>()?,
    };
    write_json_file(&out.join("manifest.json"), &manifest)?;

    let mut artifacts = artifact_specs
        .iter()
        .map(|(path, _)| (*path).to_string())
        .collect::<Vec<_>>();
    artifacts.push("manifest.json".to_string());

    let summary = EvidenceBundleSummary {
        bundle_version: "1",
        output_dir: ".".to_string(),
        message_type,
        validation_valid: validation_report.valid,
        validation_issue_count: validation_report.issue_count,
        redaction_phi_removed: redaction_receipt.phi_removed,
        artifacts,
    };
    output_options.emit(&serde_json::to_string_pretty(&summary)?)?;

    Ok(())
}

fn replay_command(
    bundle: &Path,
    format: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = build_replay_report(bundle);
    output_options.emit(&render_replay_report(&report, format)?)?;

    if report.reproduced {
        Ok(())
    } else {
        Err(CliFailure::check_failed(
            "bundle replay did not reproduce stored evidence",
        ))
    }
}

fn build_replay_report(bundle: &Path) -> EvidenceReplayReport {
    let mut checks = Vec::new();
    let required_artifacts = [
        "manifest.json",
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "profile.yaml",
        "redaction-receipt.json",
        "environment.json",
        "replay.sh",
        "replay.ps1",
    ];

    let missing_artifacts: Vec<&str> = required_artifacts
        .iter()
        .copied()
        .filter(|artifact| !bundle.join(artifact).is_file())
        .collect();
    if missing_artifacts.is_empty() {
        checks.push(replay_check(
            "bundle-layout",
            EvidenceReplayCheckStatus::Pass,
            "all expected bundle artifacts are present",
        ));
    } else {
        checks.push(replay_check(
            "bundle-layout",
            EvidenceReplayCheckStatus::Fail,
            format!(
                "missing expected bundle artifact(s): {}",
                missing_artifacts.join(", ")
            ),
        ));
    }

    let manifest = match read_bundle_manifest(bundle) {
        Ok(manifest) => {
            checks.push(replay_check(
                "manifest",
                EvidenceReplayCheckStatus::Pass,
                "manifest.json parsed",
            ));
            Some(manifest)
        }
        Err(error) => {
            checks.push(replay_check(
                "manifest",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };
    let manifest_bundle_version = manifest
        .as_ref()
        .map(|manifest| manifest.bundle_version.clone());
    let manifest_catalog_ok = manifest
        .as_ref()
        .is_some_and(|manifest| verify_bundle_manifest_catalog(manifest, &mut checks));
    let manifest_hashes_ok = manifest_catalog_ok
        && manifest
            .as_ref()
            .is_some_and(|manifest| verify_bundle_manifest_hashes(bundle, manifest, &mut checks));

    if !manifest_hashes_ok {
        return EvidenceReplayReport {
            replay_version: "1",
            bundle_version: manifest_bundle_version,
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            message_type: None,
            reproduced: false,
            validation_valid: None,
            validation_issue_count: None,
            checks,
            validation_report: None,
        };
    }

    let environment = match read_bundle_json_value(bundle, "environment.json") {
        Ok(environment) => {
            checks.push(replay_check(
                "environment",
                EvidenceReplayCheckStatus::Pass,
                "environment.json parsed",
            ));
            Some(environment)
        }
        Err(error) => {
            checks.push(replay_check(
                "environment",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let stored_report = match read_bundle_validation_report(bundle, "validation-report.json") {
        Ok(report) => {
            checks.push(replay_check(
                "stored-validation-report",
                EvidenceReplayCheckStatus::Pass,
                "validation-report.json parsed",
            ));
            Some(report)
        }
        Err(error) => {
            checks.push(replay_check(
                "stored-validation-report",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let redacted_message = match read_bundle_artifact(bundle, "message.redacted.hl7") {
        Ok(contents) => match parse(&contents) {
            Ok(message) => {
                checks.push(replay_check(
                    "parse-redacted-message",
                    EvidenceReplayCheckStatus::Pass,
                    "message.redacted.hl7 parsed",
                ));
                Some(message)
            }
            Err(error) => {
                checks.push(replay_check(
                    "parse-redacted-message",
                    EvidenceReplayCheckStatus::Fail,
                    format!("message.redacted.hl7 did not parse: {error}"),
                ));
                None
            }
        },
        Err(error) => {
            checks.push(replay_check(
                "parse-redacted-message",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let loaded_profile = match read_bundle_string(bundle, "profile.yaml") {
        Ok(profile_yaml) => match load_profile_checked(&profile_yaml) {
            Ok(profile) => {
                checks.push(replay_check(
                    "load-profile",
                    EvidenceReplayCheckStatus::Pass,
                    "profile.yaml loaded",
                ));
                Some(profile)
            }
            Err(error) => {
                checks.push(replay_check(
                    "load-profile",
                    EvidenceReplayCheckStatus::Fail,
                    format!("profile.yaml did not load: {error}"),
                ));
                None
            }
        },
        Err(error) => {
            checks.push(replay_check(
                "load-profile",
                EvidenceReplayCheckStatus::Fail,
                error,
            ));
            None
        }
    };

    let actual_report = match (redacted_message.as_ref(), loaded_profile.as_ref()) {
        (Some(message), Some(profile)) => {
            let report = ValidationReport::from_issues(
                message,
                Some("profile.yaml".to_string()),
                validate(message, profile),
            );
            checks.push(replay_check(
                "generate-validation-report",
                EvidenceReplayCheckStatus::Pass,
                "validation report regenerated from bundled message and profile",
            ));
            Some(report)
        }
        _ => {
            checks.push(replay_check(
                "generate-validation-report",
                EvidenceReplayCheckStatus::Fail,
                "validation report could not be regenerated",
            ));
            None
        }
    };

    match (actual_report.as_ref(), stored_report.as_ref()) {
        (Some(actual), Some(stored)) if actual == stored => checks.push(replay_check(
            "report-match",
            EvidenceReplayCheckStatus::Pass,
            "regenerated validation report matches validation-report.json",
        )),
        (Some(_), Some(_)) => checks.push(replay_check(
            "report-match",
            EvidenceReplayCheckStatus::Fail,
            "regenerated validation report differs from validation-report.json",
        )),
        _ => checks.push(replay_check(
            "report-match",
            EvidenceReplayCheckStatus::Fail,
            "validation report comparison could not be completed",
        )),
    }

    if let (Some(environment), Some(actual)) = (environment.as_ref(), actual_report.as_ref()) {
        let mut mismatches = Vec::new();
        if json_string(environment, "message_type").as_deref() != Some(actual.message_type.as_str())
        {
            mismatches.push("message_type");
        }
        if json_bool(environment, "validation_valid") != Some(actual.valid) {
            mismatches.push("validation_valid");
        }
        if json_usize(environment, "validation_issue_count") != Some(actual.issue_count) {
            mismatches.push("validation_issue_count");
        }

        if mismatches.is_empty() {
            checks.push(replay_check(
                "environment-match",
                EvidenceReplayCheckStatus::Pass,
                "environment metadata matches regenerated validation report",
            ));
        } else {
            checks.push(replay_check(
                "environment-match",
                EvidenceReplayCheckStatus::Fail,
                format!("environment metadata mismatch: {}", mismatches.join(", ")),
            ));
        }
    } else {
        checks.push(replay_check(
            "environment-match",
            EvidenceReplayCheckStatus::Fail,
            "environment metadata comparison could not be completed",
        ));
    }

    let reproduced = checks
        .iter()
        .all(|check| check.status == EvidenceReplayCheckStatus::Pass);
    let bundle_version = environment
        .as_ref()
        .and_then(|value| json_string(value, "bundle_version"))
        .or(manifest_bundle_version);
    let message_type = actual_report
        .as_ref()
        .map(|report| report.message_type.clone())
        .or_else(|| {
            stored_report
                .as_ref()
                .map(|report| report.message_type.clone())
        })
        .or_else(|| {
            environment
                .as_ref()
                .and_then(|value| json_string(value, "message_type"))
        });
    let validation_valid = actual_report.as_ref().map(|report| report.valid);
    let validation_issue_count = actual_report.as_ref().map(|report| report.issue_count);

    EvidenceReplayReport {
        replay_version: "1",
        bundle_version,
        tool_name: "hl7v2-cli",
        tool_version: env!("CARGO_PKG_VERSION"),
        message_type,
        reproduced,
        validation_valid,
        validation_issue_count,
        checks,
        validation_report: actual_report,
    }
}

fn replay_check(
    name: &'static str,
    status: EvidenceReplayCheckStatus,
    message: impl Into<String>,
) -> EvidenceReplayCheck {
    EvidenceReplayCheck {
        name,
        status,
        message: message.into(),
    }
}

fn bundle_artifact_specs() -> [(&'static str, &'static str); 8] {
    [
        ("message.redacted.hl7", "redacted_message"),
        ("validation-report.json", "validation_report"),
        ("field-paths.json", "field_path_trace"),
        ("profile.yaml", "profile"),
        ("redaction-receipt.json", "redaction_receipt"),
        ("environment.json", "environment"),
        ("replay.sh", "replay_shell_script"),
        ("replay.ps1", "replay_powershell_script"),
    ]
}

fn read_bundle_manifest(bundle: &Path) -> Result<EvidenceBundleManifest, String> {
    let contents = read_bundle_string(bundle, "manifest.json")?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("manifest.json is invalid JSON: {error}"))
}

fn verify_bundle_manifest_catalog(
    manifest: &EvidenceBundleManifest,
    checks: &mut Vec<EvidenceReplayCheck>,
) -> bool {
    let expected = bundle_artifact_specs();
    let mut errors = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for artifact in &manifest.artifacts {
        if !seen_paths.insert(artifact.path.clone()) {
            errors.push("duplicate artifact path".to_string());
        }
        if safe_bundle_relative_path(&artifact.path).is_err() {
            errors.push("unsafe artifact path".to_string());
            continue;
        }
        if !is_lower_sha256_hex(&artifact.sha256) {
            errors.push(format!("{} has invalid sha256", artifact.path));
        }
        if !expected
            .iter()
            .any(|(path, role)| *path == artifact.path.as_str() && *role == artifact.role.as_str())
        {
            errors.push(format!(
                "{} has unexpected role {}",
                artifact.path, artifact.role
            ));
        }
    }

    for (expected_path, expected_role) in expected {
        if !manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.path == expected_path && artifact.role == expected_role)
        {
            errors.push(format!("missing manifest entry for {expected_path}"));
        }
    }

    if errors.is_empty() {
        checks.push(replay_check(
            "manifest-artifacts",
            EvidenceReplayCheckStatus::Pass,
            "manifest lists expected bundle artifacts",
        ));
        true
    } else {
        checks.push(replay_check(
            "manifest-artifacts",
            EvidenceReplayCheckStatus::Fail,
            format!("manifest artifact catalog invalid: {}", errors.join(", ")),
        ));
        false
    }
}

fn verify_bundle_manifest_hashes(
    bundle: &Path,
    manifest: &EvidenceBundleManifest,
    checks: &mut Vec<EvidenceReplayCheck>,
) -> bool {
    let mut errors = Vec::new();

    for artifact in &manifest.artifacts {
        let relative_path = match safe_bundle_relative_path(&artifact.path) {
            Ok(relative_path) => relative_path,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        match fs::read(bundle.join(relative_path)) {
            Ok(bytes) => {
                let actual = compute_sha256_bytes(&bytes);
                if actual != artifact.sha256 {
                    errors.push(format!("{} hash mismatch", artifact.path));
                }
            }
            Err(error) => {
                errors.push(format!("could not read {}: {error}", artifact.path));
            }
        }
    }

    if errors.is_empty() {
        checks.push(replay_check(
            "manifest-hashes",
            EvidenceReplayCheckStatus::Pass,
            "manifest artifact hashes match bundle contents",
        ));
        true
    } else {
        checks.push(replay_check(
            "manifest-hashes",
            EvidenceReplayCheckStatus::Fail,
            format!("manifest hash verification failed: {}", errors.join(", ")),
        ));
        false
    }
}

fn safe_bundle_relative_path(path: &str) -> Result<PathBuf, String> {
    if path.is_empty() || path.contains('\\') {
        return Err("manifest artifact path must be bundle-relative".to_string());
    }

    let relative_path = Path::new(path);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return Err("manifest artifact path must be bundle-relative".to_string());
    }

    Ok(relative_path.to_path_buf())
}

fn is_lower_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn read_bundle_artifact(bundle: &Path, artifact: &str) -> Result<Vec<u8>, String> {
    fs::read(bundle.join(artifact)).map_err(|error| format!("could not read {artifact}: {error}"))
}

fn read_bundle_string(bundle: &Path, artifact: &str) -> Result<String, String> {
    fs::read_to_string(bundle.join(artifact))
        .map_err(|error| format!("could not read {artifact}: {error}"))
}

fn read_bundle_json_value(bundle: &Path, artifact: &str) -> Result<serde_json::Value, String> {
    let contents = read_bundle_string(bundle, artifact)?;
    serde_json::from_str(&contents).map_err(|error| format!("{artifact} is invalid JSON: {error}"))
}

fn read_bundle_validation_report(
    bundle: &Path,
    artifact: &str,
) -> Result<ValidationReport, String> {
    let contents = read_bundle_string(bundle, artifact)?;
    serde_json::from_str(&contents).map_err(|error| format!("{artifact} is invalid JSON: {error}"))
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToOwned::to_owned)
}

fn json_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

fn json_usize(value: &serde_json::Value, key: &str) -> Option<usize> {
    value
        .get(key)?
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
}

fn render_replay_report(
    report: &EvidenceReplayReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let mut output = String::new();
            output.push_str("Evidence Replay\n");
            output.push_str(&format!("  Reproduced: {}\n", report.reproduced));
            if let Some(message_type) = &report.message_type {
                output.push_str(&format!("  Message type: {message_type}\n"));
            }
            if let Some(valid) = report.validation_valid {
                output.push_str(&format!("  Validation valid: {valid}\n"));
            }
            if let Some(issue_count) = report.validation_issue_count {
                output.push_str(&format!("  Validation issues: {issue_count}\n"));
            }
            output.push_str("Checks:\n");
            for check in &report.checks {
                let status = match check.status {
                    EvidenceReplayCheckStatus::Pass => "PASS",
                    EvidenceReplayCheckStatus::Fail => "FAIL",
                };
                output.push_str(&format!("  {status} {} - {}\n", check.name, check.message));
            }
            Ok(output)
        }
    }
}

fn write_json_file<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn bundle_manifest_artifact(
    bundle_dir: &Path,
    path: &'static str,
    role: &'static str,
) -> Result<EvidenceBundleManifestArtifact, Box<dyn std::error::Error>> {
    let bytes = fs::read(bundle_dir.join(path))?;
    Ok(EvidenceBundleManifestArtifact {
        path: path.to_string(),
        role: role.to_string(),
        sha256: compute_sha256_bytes(&bytes),
    })
}

fn compute_sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn replay_shell_script() -> &'static str {
    "#!/usr/bin/env sh\nset -eu\ncd \"$(dirname \"$0\")\"\nhl7v2 val message.redacted.hl7 --profile profile.yaml --report json > validation-report.replayed.json\n"
}

fn replay_powershell_script() -> &'static str {
    "$ErrorActionPreference = 'Stop'\nSet-Location $PSScriptRoot\nhl7v2 val .\\message.redacted.hl7 --profile .\\profile.yaml --report json > .\\validation-report.replayed.json\n"
}

fn load_safe_analysis_policy(
    policy_text: &str,
) -> Result<SafeAnalysisPolicy, Box<dyn std::error::Error>> {
    let policy: SafeAnalysisPolicy = toml::from_str(policy_text)?;
    if policy.rules.is_empty() {
        return Err(
            std::io::Error::other("redaction policy must contain at least one rule").into(),
        );
    }

    let mut seen_paths = BTreeSet::new();
    for rule in &policy.rules {
        parse_redaction_path(&rule.path).map_err(std::io::Error::other)?;
        if !seen_paths.insert(rule.path.clone()) {
            return Err(std::io::Error::other(format!(
                "redaction policy contains duplicate rule for {}",
                rule.path
            ))
            .into());
        }
        if rule.reason.as_deref().unwrap_or("").trim().is_empty() {
            return Err(std::io::Error::other(format!(
                "redaction rule {} must include a reason",
                rule.path
            ))
            .into());
        }
        if safe_analysis_sensitive_paths().contains(rule.path.as_str())
            && rule.action == RedactionAction::Retain
        {
            return Err(std::io::Error::other(format!(
                "redaction rule {} cannot retain a built-in sensitive field",
                rule.path
            ))
            .into());
        }
    }

    Ok(policy)
}

fn apply_safe_analysis_policy(
    message: &mut Message,
    policy: &SafeAnalysisPolicy,
) -> Result<RedactionReceipt, Box<dyn std::error::Error>> {
    validate_safe_analysis_policy_covers_sensitive_fields(message, policy)?;

    let mut actions = Vec::new();
    let mut phi_removed = false;
    let mut errors = Vec::new();

    for rule in &policy.rules {
        let parsed_path = parse_redaction_path(&rule.path).map_err(std::io::Error::other)?;
        let mut matched_count = 0_usize;

        for segment in &mut message.segments {
            if segment.id_str() != parsed_path.segment_id {
                continue;
            }

            let Some(field_index) =
                modeled_field_index(&parsed_path.segment_id, parsed_path.field_index)
            else {
                continue;
            };
            let Some(field) = segment.fields.get_mut(field_index) else {
                continue;
            };

            matched_count = matched_count.saturating_add(1);
            match rule.action {
                RedactionAction::Hash => {
                    let value = field_to_text(field, &message.delims);
                    *field = Field::from_text(format!("hash:sha256:{}", compute_sha256(&value)));
                    phi_removed = true;
                }
                RedactionAction::Drop => {
                    *field = Field::new();
                    phi_removed = true;
                }
                RedactionAction::Retain => {}
            }
        }

        let status = match (matched_count, rule.action) {
            (0, _) => RedactionActionStatus::NotFound,
            (_, RedactionAction::Retain) => RedactionActionStatus::Retained,
            _ => RedactionActionStatus::Applied,
        };

        if matched_count == 0 && !rule.optional && rule.action != RedactionAction::Retain {
            errors.push(format!(
                "redaction rule {} matched no fields; mark optional=true if absence is expected",
                rule.path
            ));
        }

        actions.push(RedactionActionReceipt {
            path: rule.path.clone(),
            action: rule.action,
            reason: rule.reason.clone().unwrap_or_default(),
            matched_count,
            optional: rule.optional,
            status,
        });
    }

    if !errors.is_empty() {
        return Err(std::io::Error::other(errors.join("; ")).into());
    }

    Ok(RedactionReceipt {
        phi_removed,
        hash_algorithm: "sha256",
        actions,
    })
}

fn validate_safe_analysis_policy_covers_sensitive_fields(
    message: &Message,
    policy: &SafeAnalysisPolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    let protected_paths: BTreeSet<&str> = policy
        .rules
        .iter()
        .filter(|rule| rule.action != RedactionAction::Retain)
        .map(|rule| rule.path.as_str())
        .collect();
    let present_sensitive_paths = present_sensitive_paths(message);
    let missing_paths: Vec<&str> = present_sensitive_paths
        .iter()
        .copied()
        .filter(|path| !protected_paths.contains(path))
        .collect();

    if missing_paths.is_empty() {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "redaction policy does not protect present sensitive field(s): {}",
        missing_paths.join(", ")
    ))
    .into())
}

fn present_sensitive_paths(message: &Message) -> BTreeSet<&'static str> {
    safe_analysis_sensitive_paths()
        .iter()
        .copied()
        .filter(|path| {
            parse_redaction_path(path).ok().is_some_and(|parsed| {
                message_has_nonempty_field(message, &parsed.segment_id, parsed.field_index)
            })
        })
        .collect()
}

fn safe_analysis_sensitive_paths() -> BTreeSet<&'static str> {
    [
        "PID.3", "PID.5", "PID.7", "PID.11", "PID.13", "PID.14", "PID.19", "NK1.2", "NK1.4",
        "NK1.5",
    ]
    .into_iter()
    .collect()
}

struct ParsedRedactionPath {
    segment_id: String,
    field_index: usize,
}

fn parse_redaction_path(path: &str) -> Result<ParsedRedactionPath, String> {
    let (segment_id, field_part) = path
        .split_once('.')
        .ok_or_else(|| format!("redaction path '{path}' must use SEG.field syntax"))?;
    if segment_id.len() != 3
        || !segment_id
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return Err(format!(
            "redaction path '{path}' must start with a three-character uppercase segment id"
        ));
    }
    if field_part.contains('.') {
        return Err(format!(
            "redaction path '{path}' must target a field, not a component"
        ));
    }

    let field_index = field_part.parse::<usize>().map_err(|_err| {
        format!("redaction path '{path}' must use a positive numeric field index")
    })?;
    if field_index == 0 {
        return Err(format!(
            "redaction path '{path}' must use a one-based field index"
        ));
    }
    if segment_id == "MSH" && field_index < 3 {
        return Err(format!(
            "redaction path '{path}' targets MSH.1/MSH.2, which are delimiter metadata and not redacted by this command"
        ));
    }

    Ok(ParsedRedactionPath {
        segment_id: segment_id.to_string(),
        field_index,
    })
}

fn message_field_text(message: &Message, segment_id: &str, field_index: usize) -> Option<String> {
    let field_index = modeled_field_index(segment_id, field_index)?;
    let field = message
        .segments
        .iter()
        .find(|segment| segment.id_str() == segment_id)?
        .fields
        .get(field_index)?;
    Some(field_to_text(field, &message.delims))
}

fn message_has_nonempty_field(message: &Message, segment_id: &str, field_index: usize) -> bool {
    let Some(field_index) = modeled_field_index(segment_id, field_index) else {
        return false;
    };

    message
        .segments
        .iter()
        .filter(|segment| segment.id_str() == segment_id)
        .filter_map(|segment| segment.fields.get(field_index))
        .any(|field| !field_to_text(field, &message.delims).is_empty())
}

fn build_field_path_trace(message: &Message, receipt: &RedactionReceipt) -> FieldPathTraceReport {
    let redaction_actions: BTreeMap<&str, RedactionAction> = receipt
        .actions
        .iter()
        .map(|action| (action.path.as_str(), action.action))
        .collect();
    let mut fields = Vec::new();

    for (segment_position, segment) in message.segments.iter().enumerate() {
        let segment_index = segment_position.saturating_add(1);
        for (modeled_index, field) in segment.fields.iter().enumerate() {
            let field_index = hl7_field_index(segment.id_str(), modeled_index);
            let canonical_path = format!("{}.{}", segment.id_str(), field_index);
            let field_text = field_to_text(field, &message.delims);
            fields.push(FieldPathTrace {
                path: format!("{}[{}].{}", segment.id_str(), segment_index, field_index),
                canonical_path: canonical_path.clone(),
                segment_index,
                field_index,
                present: !field_text.is_empty(),
                value_shape: field_value_shape(&field_text),
                redaction_action: redaction_actions.get(canonical_path.as_str()).copied(),
            });
        }
    }

    FieldPathTraceReport {
        message_type: message_field_text(message, "MSH", 9).unwrap_or_else(|| "unknown".into()),
        field_count: fields.len(),
        fields,
    }
}

fn hl7_field_index(segment_id: &str, modeled_index: usize) -> usize {
    if segment_id == "MSH" {
        modeled_index.saturating_add(2)
    } else {
        modeled_index.saturating_add(1)
    }
}

fn field_value_shape(field_text: &str) -> FieldValueShape {
    if field_text.is_empty() {
        FieldValueShape::Empty
    } else if field_text.starts_with("hash:sha256:") {
        FieldValueShape::HashedSha256
    } else {
        FieldValueShape::Present
    }
}

fn modeled_field_index(segment_id: &str, field_index: usize) -> Option<usize> {
    if segment_id == "MSH" {
        field_index.checked_sub(2)
    } else {
        field_index.checked_sub(1)
    }
}

fn field_to_text(field: &Field, delims: &hl7v2::Delims) -> String {
    field
        .reps
        .iter()
        .map(|rep| {
            rep.comps
                .iter()
                .map(|comp| {
                    comp.subs
                        .iter()
                        .map(|atom| match atom {
                            Atom::Text(text) => text.as_str(),
                            Atom::Null => "\"\"",
                        })
                        .collect::<Vec<_>>()
                        .join(&delims.sub.to_string())
                })
                .collect::<Vec<_>>()
                .join(&delims.comp.to_string())
        })
        .collect::<Vec<_>>()
        .join(&delims.rep.to_string())
}

fn profile_lint_command(
    profile: &Path,
    report: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile_yaml = fs::read_to_string(profile)?;
    let lint_report = lint_profile_yaml(&profile_yaml);
    let output = format_profile_lint_report(profile, &lint_report, report)?;
    output_options.emit(&output)?;

    if !lint_report.valid {
        return Err(CliFailure::check_failed("profile lint reported errors"));
    }

    Ok(())
}

fn profile_explain_command(
    profile: &Path,
    format: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile_checked(&profile_yaml)?;
    let lint_report = lint_profile_yaml(&profile_yaml);
    let explain_report =
        build_profile_explain_report(profile, &profile_yaml, &loaded_profile, &lint_report);
    let output = format_profile_explain_report(&explain_report, format)?;
    output_options.emit(&output)?;
    Ok(())
}

fn build_profile_explain_report(
    profile_path: &Path,
    profile_yaml: &str,
    profile: &Profile,
    lint_report: &ProfileLintReport,
) -> ProfileExplainReport {
    let required_fields: Vec<ProfileExplainRequiredField> = profile
        .constraints
        .iter()
        .filter(|constraint| constraint.required)
        .map(|constraint| ProfileExplainRequiredField {
            path: constraint.path.clone(),
            conditional: constraint.when.is_some(),
        })
        .collect();

    let table_code_counts: BTreeMap<&str, usize> = profile
        .hl7_tables
        .iter()
        .map(|table| (table.id.as_str(), table.codes.len()))
        .collect();

    let datatype_rules = profile
        .datatypes
        .iter()
        .map(|datatype| ProfileExplainDatatypeRule {
            path: datatype.path.clone(),
            datatype: datatype.r#type.clone(),
            kind: "simple",
            pattern: None,
            min_length: None,
            max_length: None,
            format: None,
            checksum: None,
        })
        .chain(
            profile
                .advanced_datatypes
                .iter()
                .map(|datatype| ProfileExplainDatatypeRule {
                    path: datatype.path.clone(),
                    datatype: datatype.r#type.clone(),
                    kind: "advanced",
                    pattern: datatype.pattern.clone(),
                    min_length: datatype.min_length,
                    max_length: datatype.max_length,
                    format: datatype.format.clone(),
                    checksum: datatype.checksum.clone(),
                }),
        )
        .collect();

    ProfileExplainReport {
        profile: profile_path.to_string_lossy().to_string(),
        profile_sha256: compute_sha256(profile_yaml),
        message_structure: profile.message_structure.clone(),
        version: profile.version.clone(),
        message_type: profile.message_type.clone(),
        parent: profile.parent.clone(),
        summary: ProfileExplainSummary {
            segment_count: profile.segments.len(),
            required_field_count: required_fields.len(),
            field_constraint_count: profile.constraints.len(),
            length_rule_count: profile.lengths.len(),
            datatype_rule_count: profile.datatypes.len(),
            advanced_datatype_rule_count: profile.advanced_datatypes.len(),
            value_set_count: profile.valuesets.len(),
            cross_field_rule_count: profile.cross_field_rules.len(),
            temporal_rule_count: profile.temporal_rules.len(),
            contextual_rule_count: profile.contextual_rules.len(),
            custom_rule_count: profile.custom_rules.len(),
            hl7_table_count: profile.hl7_tables.len(),
        },
        segments: profile
            .segments
            .iter()
            .map(|segment| ProfileExplainSegment {
                id: segment.id.clone(),
            })
            .collect(),
        required_fields,
        field_constraints: profile
            .constraints
            .iter()
            .map(|constraint| {
                let (component_min, component_max) = constraint
                    .components
                    .as_ref()
                    .map(|components| (components.min, components.max))
                    .unwrap_or((None, None));
                let allowed_values = constraint.r#in.clone().unwrap_or_default();
                ProfileExplainConstraint {
                    path: constraint.path.clone(),
                    required: constraint.required,
                    conditional: constraint.when.is_some(),
                    component_min,
                    component_max,
                    allowed_value_count: allowed_values.len(),
                    allowed_values,
                    pattern: constraint.pattern.clone(),
                }
            })
            .collect(),
        length_rules: profile
            .lengths
            .iter()
            .map(|length| ProfileExplainLengthRule {
                path: length.path.clone(),
                max: length.max,
                policy: length.policy.clone(),
            })
            .collect(),
        datatype_rules,
        value_sets: profile
            .valuesets
            .iter()
            .map(|valueset| {
                let table_code_count = table_code_counts
                    .get(valueset.name.as_str())
                    .copied()
                    .unwrap_or(0);
                let source = if !valueset.codes.is_empty() {
                    "inline"
                } else if table_code_count > 0 {
                    "hl7_table"
                } else {
                    "empty"
                };
                ProfileExplainValueSet {
                    name: valueset.name.clone(),
                    path: valueset.path.clone(),
                    source,
                    inline_code_count: valueset.codes.len(),
                    table_code_count,
                }
            })
            .collect(),
        rules: ProfileExplainRules {
            cross_field: profile
                .cross_field_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
            temporal: profile
                .temporal_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
            contextual: profile
                .contextual_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
            custom: profile
                .custom_rules
                .iter()
                .map(|rule| ProfileExplainRule {
                    id: rule.id.clone(),
                    description: rule.description.clone(),
                })
                .collect(),
        },
        hl7_tables: profile
            .hl7_tables
            .iter()
            .map(|table| ProfileExplainTable {
                id: table.id.clone(),
                name: table.name.clone(),
                version: table.version.clone(),
                code_count: table.codes.len(),
            })
            .collect(),
        table_precedence: profile.table_precedence.clone(),
        expression_guardrails: ProfileExplainExpressionGuardrails {
            max_depth: profile.expression_guardrails.max_depth,
            max_length: profile.expression_guardrails.max_length,
            allow_custom_scripts: profile.expression_guardrails.allow_custom_scripts,
        },
        lint: ProfileExplainLintSummary {
            valid: lint_report.valid,
            error_count: lint_report.error_count,
            warning_count: lint_report.warning_count,
            issue_count: lint_report.issue_count,
            ignored_or_unsupported: lint_report
                .issues
                .iter()
                .filter(|issue| profile_lint_issue_is_ignored_or_unsupported(issue))
                .cloned()
                .collect(),
        },
    }
}

fn profile_lint_issue_is_ignored_or_unsupported(issue: &ProfileLintIssue) -> bool {
    issue.code.starts_with("unknown_")
        || issue.code.contains("unsupported")
        || issue.message.contains("ignored")
}

fn profile_test_command(
    profile: &Path,
    fixtures: &Path,
    report: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile_checked(&profile_yaml)?;
    let test_report = run_profile_fixture_tests(profile, fixtures, &loaded_profile)?;
    let output = format_profile_test_report(&test_report, report)?;
    output_options.emit(&output)?;

    if !test_report.valid {
        return Err(CliFailure::check_failed("profile test reported failures"));
    }

    Ok(())
}

fn run_profile_fixture_tests(
    profile_path: &Path,
    fixtures: &Path,
    profile: &Profile,
) -> Result<ProfileTestReport, Box<dyn std::error::Error>> {
    let valid_root = fixtures.join("valid");
    let invalid_root = fixtures.join("invalid");
    let expected_root = fixtures.join("expected");
    let valid_files = collect_hl7_fixture_files(&valid_root)?;
    let invalid_files = collect_hl7_fixture_files(&invalid_root)?;
    let expected_reports =
        build_expected_report_lookup(fixtures, &expected_root, [&valid_files, &invalid_files]);

    let mut cases = Vec::new();
    cases.extend(run_profile_fixture_group(
        profile_path,
        fixtures,
        &valid_files,
        &expected_reports,
        ProfileFixtureExpectation::Valid,
        profile,
    )?);
    cases.extend(run_profile_fixture_group(
        profile_path,
        fixtures,
        &invalid_files,
        &expected_reports,
        ProfileFixtureExpectation::Invalid,
        profile,
    )?);

    if cases.is_empty() {
        return Err(std::io::Error::other(format!(
            "no .hl7 fixtures found under {}",
            fixtures.display()
        ))
        .into());
    }

    let passed_count = cases.iter().filter(|case| case.passed).count();
    let case_count = cases.len();
    let failed_count = case_count.saturating_sub(passed_count);

    Ok(ProfileTestReport {
        profile: profile_path.to_string_lossy().to_string(),
        fixtures: fixtures.to_string_lossy().to_string(),
        valid: failed_count == 0,
        case_count,
        passed_count,
        failed_count,
        cases,
    })
}

fn run_profile_fixture_group(
    profile_path: &Path,
    fixture_root: &Path,
    files: &[PathBuf],
    expected_reports: &BTreeMap<PathBuf, ExpectedReportCandidate>,
    expectation: ProfileFixtureExpectation,
    profile: &Profile,
) -> Result<Vec<ProfileTestCaseReport>, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for path in files {
        cases.push(run_profile_fixture_case(
            profile_path,
            fixture_root,
            expected_reports,
            path,
            expectation,
            profile,
        ));
    }
    Ok(cases)
}

fn run_profile_fixture_case(
    profile_path: &Path,
    fixture_root: &Path,
    expected_reports: &BTreeMap<PathBuf, ExpectedReportCandidate>,
    path: &Path,
    expectation: ProfileFixtureExpectation,
    profile: &Profile,
) -> ProfileTestCaseReport {
    let name = relative_display_path(fixture_root, path);

    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(err) => {
            return ProfileTestCaseReport {
                name,
                path: path.to_string_lossy().to_string(),
                expectation,
                passed: false,
                message: format!("fixture could not be read: {err}"),
                validation_report: None,
                expected_report: None,
            };
        }
    };

    let message = match parse(&contents) {
        Ok(message) => message,
        Err(err) => {
            return ProfileTestCaseReport {
                name,
                path: path.to_string_lossy().to_string(),
                expectation,
                passed: false,
                message: format!("fixture did not parse as HL7: {err}"),
                validation_report: None,
                expected_report: None,
            };
        }
    };

    let issues = validate(&message, profile);
    let validation_report = ValidationReport::from_issues(
        &message,
        Some(profile_path.to_string_lossy().to_string()),
        issues,
    );
    let expected_valid = expectation == ProfileFixtureExpectation::Valid;
    let mut passed = validation_report.valid == expected_valid;
    let mut message = if passed {
        format!(
            "expected {} and report was {}",
            expectation.as_str(),
            if validation_report.valid {
                "valid"
            } else {
                "invalid"
            }
        )
    } else {
        format!(
            "expected {} but report was {}",
            expectation.as_str(),
            if validation_report.valid {
                "valid"
            } else {
                "invalid"
            }
        )
    };

    let expected_report = expected_reports
        .get(path)
        .map(|candidate| compare_expected_report_candidate(candidate, &validation_report));
    if let Some(comparison) = &expected_report {
        if comparison.matched {
            message.push_str("; expected report matched");
        } else {
            passed = false;
            let detail = comparison
                .message
                .as_deref()
                .unwrap_or("expected report did not match");
            message.push_str(&format!("; {detail}"));
        }
    }

    ProfileTestCaseReport {
        name,
        path: path.to_string_lossy().to_string(),
        expectation,
        passed,
        message,
        validation_report: Some(validation_report),
        expected_report,
    }
}

fn collect_hl7_fixture_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }

    collect_hl7_fixture_files_recursive(root, &mut files)?;
    files.sort_by(|left, right| compare_paths_case_stable(left, right));
    Ok(files)
}

fn compare_paths_case_stable(left: &Path, right: &Path) -> Ordering {
    let left_display = left.to_string_lossy();
    let right_display = right.to_string_lossy();
    left_display
        .to_lowercase()
        .cmp(&right_display.to_lowercase())
        .then_with(|| left_display.cmp(&right_display))
}

fn collect_hl7_fixture_files_recursive(
    root: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_hl7_fixture_files_recursive(&path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("hl7"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn build_expected_report_lookup<'a>(
    fixture_root: &Path,
    expected_root: &Path,
    fixture_groups: impl IntoIterator<Item = &'a Vec<PathBuf>>,
) -> BTreeMap<PathBuf, ExpectedReportCandidate> {
    let fixtures: Vec<&PathBuf> = fixture_groups
        .into_iter()
        .flat_map(|group| group.iter())
        .collect();
    let mut fallback_counts = BTreeMap::new();
    for fixture_path in &fixtures {
        let fallback = fallback_expected_report_path(expected_root, fixture_path);
        if fallback.exists() {
            let count = fallback_counts.entry(fallback).or_insert(0_usize);
            *count = count.saturating_add(1);
        }
    }

    let mut lookup = BTreeMap::new();
    for fixture_path in fixtures {
        let primary = primary_expected_report_path(expected_root, fixture_root, fixture_path);
        if primary.exists() {
            lookup.insert(fixture_path.clone(), ExpectedReportCandidate::File(primary));
            continue;
        }

        let fallback = fallback_expected_report_path(expected_root, fixture_path);
        match fallback_counts.get(&fallback).copied() {
            Some(1) => {
                lookup.insert(
                    fixture_path.clone(),
                    ExpectedReportCandidate::File(fallback),
                );
            }
            Some(_) => {
                lookup.insert(
                    fixture_path.clone(),
                    ExpectedReportCandidate::Ambiguous(fallback),
                );
            }
            None => {}
        }
    }
    lookup
}

fn primary_expected_report_path(
    expected_root: &Path,
    fixture_root: &Path,
    fixture_path: &Path,
) -> PathBuf {
    let relative = fixture_path
        .strip_prefix(fixture_root)
        .unwrap_or(fixture_path);
    let mut path = expected_root.join(relative);
    path.set_extension("report.json");
    path
}

fn fallback_expected_report_path(expected_root: &Path, fixture_path: &Path) -> PathBuf {
    let stem = fixture_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("fixture");
    expected_root.join(format!("{stem}.report.json"))
}

fn compare_expected_report_candidate(
    candidate: &ExpectedReportCandidate,
    actual_report: &ValidationReport,
) -> ExpectedReportComparison {
    match candidate {
        ExpectedReportCandidate::File(path) => {
            compare_expected_report(path, actual_report).unwrap_or_else(|| ExpectedReportComparison {
                path: path.to_string_lossy().to_string(),
                matched: false,
                message: Some("expected report path was registered but no longer exists".to_string()),
            })
        }
        ExpectedReportCandidate::Ambiguous(path) => ExpectedReportComparison {
            path: path.to_string_lossy().to_string(),
            matched: false,
            message: Some(
                "ambiguous basename expected report; use expected/valid/... or expected/invalid/..."
                    .to_string(),
            ),
        },
    }
}

fn compare_expected_report(
    expected_path: &Path,
    actual_report: &ValidationReport,
) -> Option<ExpectedReportComparison> {
    if !expected_path.exists() {
        return None;
    }

    let path = expected_path.to_string_lossy().to_string();
    let expected = match fs::read_to_string(expected_path)
        .map_err(|err| format!("expected report could not be read: {err}"))
        .and_then(|contents| {
            serde_json::from_str::<serde_json::Value>(&contents)
                .map_err(|err| format!("expected report is not valid JSON: {err}"))
        }) {
        Ok(expected) => expected,
        Err(message) => {
            return Some(ExpectedReportComparison {
                path,
                matched: false,
                message: Some(message),
            });
        }
    };

    let actual = match serde_json::to_value(actual_report) {
        Ok(actual) => actual,
        Err(err) => {
            return Some(ExpectedReportComparison {
                path,
                matched: false,
                message: Some(format!("actual report could not be serialized: {err}")),
            });
        }
    };

    match json_subset_matches(&expected, &actual, "$") {
        Ok(()) => Some(ExpectedReportComparison {
            path,
            matched: true,
            message: None,
        }),
        Err(message) => Some(ExpectedReportComparison {
            path,
            matched: false,
            message: Some(message),
        }),
    }
}

fn json_subset_matches(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    path: &str,
) -> Result<(), String> {
    match (expected, actual) {
        (serde_json::Value::Object(expected), serde_json::Value::Object(actual)) => {
            for (key, expected_value) in expected {
                let actual_value = actual
                    .get(key)
                    .ok_or_else(|| format!("{path}.{key} was missing from actual report"))?;
                json_subset_matches(expected_value, actual_value, &format!("{path}.{key}"))?;
            }
            Ok(())
        }
        (serde_json::Value::Array(expected), serde_json::Value::Array(actual)) => {
            for (index, expected_value) in expected.iter().enumerate() {
                let matched = actual.iter().any(|actual_value| {
                    json_subset_matches(expected_value, actual_value, &format!("{path}[{index}]"))
                        .is_ok()
                });
                if !matched {
                    return Err(format!(
                        "{path}[{index}] did not match any actual report item"
                    ));
                }
            }
            Ok(())
        }
        _ if expected == actual => Ok(()),
        _ => Err(format!(
            "{path} expected {} but actual report had {}",
            expected, actual
        )),
    }
}

fn relative_display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn format_profile_test_report(
    report: &ProfileTestReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let mut lines = Vec::new();
            if report.valid {
                lines.push(format!(
                    "Profile test passed: {} against {}",
                    report.profile, report.fixtures
                ));
            } else {
                lines.push(format!(
                    "Profile test failed: {} failure(s) across {} case(s)",
                    report.failed_count, report.case_count
                ));
            }
            lines.push(format!(
                "  Cases: {} passed, {} failed",
                report.passed_count, report.failed_count
            ));

            for case in &report.cases {
                let status = if case.passed { "PASS" } else { "FAIL" };
                lines.push(format!(
                    "  - {} {} expected {}: {}",
                    status,
                    case.name,
                    case.expectation.as_str(),
                    case.message
                ));
            }

            Ok(lines.join("\n"))
        }
    }
}

fn format_profile_explain_report(
    report: &ProfileExplainReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let segment_ids = report
                .segments
                .iter()
                .map(|segment| segment.id.clone())
                .collect::<Vec<_>>();
            let required_paths = report
                .required_fields
                .iter()
                .map(|field| field.path.clone())
                .collect::<Vec<_>>();
            let mut lines = Vec::new();

            lines.push(format!("Profile explain: {}", report.profile));
            lines.push(format!("  Message structure: {}", report.message_structure));
            lines.push(format!("  Version: {}", report.version));
            if let Some(message_type) = &report.message_type {
                lines.push(format!("  Message type: {message_type}"));
            }
            if let Some(parent) = &report.parent {
                lines.push(format!("  Parent: {parent} (loaded profile only)"));
            }
            lines.push(format!("  Profile SHA-256: {}", report.profile_sha256));
            lines.push(format!(
                "  Segments: {} ({})",
                report.summary.segment_count,
                format_string_list(&segment_ids)
            ));
            lines.push(format!(
                "  Required fields: {} ({})",
                report.summary.required_field_count,
                format_string_list(&required_paths)
            ));
            lines.push(format!(
                "  Constraints: {} field, {} length, {} datatype, {} advanced datatype",
                report.summary.field_constraint_count,
                report.summary.length_rule_count,
                report.summary.datatype_rule_count,
                report.summary.advanced_datatype_rule_count
            ));
            lines.push(format!(
                "  Value sets: {} set(s), {} inline code(s), {} table code(s)",
                report.summary.value_set_count,
                report
                    .value_sets
                    .iter()
                    .map(|valueset| valueset.inline_code_count)
                    .sum::<usize>(),
                report
                    .value_sets
                    .iter()
                    .map(|valueset| valueset.table_code_count)
                    .sum::<usize>()
            ));
            lines.push(format!(
                "  Rules: {} cross-field, {} temporal, {} contextual, {} custom",
                report.summary.cross_field_rule_count,
                report.summary.temporal_rule_count,
                report.summary.contextual_rule_count,
                report.summary.custom_rule_count
            ));
            lines.push(format!(
                "  HL7 tables: {} table(s)",
                report.summary.hl7_table_count
            ));
            lines.push(format!(
                "  Lint: {} ({} error(s), {} warning(s))",
                if report.lint.valid {
                    "valid"
                } else {
                    "invalid"
                },
                report.lint.error_count,
                report.lint.warning_count
            ));

            if !report.lint.ignored_or_unsupported.is_empty() {
                lines.push("  Ignored or unsupported profile config:".to_string());
                for issue in &report.lint.ignored_or_unsupported {
                    let location = issue.path.as_deref().unwrap_or("profile");
                    lines.push(format!(
                        "    - {} {} {}: {}",
                        issue.severity.as_str(),
                        issue.code,
                        location,
                        issue.message
                    ));
                }
            }

            Ok(lines.join("\n"))
        }
    }
}

fn format_profile_lint_report(
    profile: &Path,
    report: &ProfileLintReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
            let mut lines = Vec::new();
            if report.valid {
                lines.push(format!("Profile lint passed: {}", profile.display()));
            } else {
                lines.push(format!(
                    "Profile lint failed: {} error(s), {} warning(s)",
                    report.error_count, report.warning_count
                ));
            }

            for issue in &report.issues {
                let location = issue.path.as_deref().unwrap_or("profile");
                lines.push(format!(
                    "  - {} {} {}: {}",
                    issue.severity.as_str(),
                    issue.code,
                    location,
                    issue.message
                ));
            }

            if report.issues.is_empty() {
                lines.push("  No profile lint issues found".to_string());
            } else if report.warning_count > 0 && report.error_count == 0 {
                lines.push(format!(
                    "  {} warning(s) found; profile can still load",
                    report.warning_count
                ));
            }

            Ok(lines.join("\n"))
        }
    }
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

/// Collect statistics from an HL7 message
fn collect_stats(message: &Message, distributions: bool) -> StatsReport {
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
fn format_stats_report(
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

fn corpus_summarize_command(
    path: &PathBuf,
    format: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = summarize_corpus_path(path)?;
    let output = format_corpus_summary(&summary, format)?;
    output_options.emit(&output)?;
    Ok(())
}

fn format_corpus_summary(
    summary: &CorpusSummary,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
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

fn corpus_diff_command(
    before: &PathBuf,
    after: &PathBuf,
    profile: Option<&PathBuf>,
    format: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let output = format_corpus_diff(&diff, format)?;
    output_options.emit(&output)?;
    Ok(())
}

fn corpus_fingerprint_command(
    path: &PathBuf,
    profile: Option<&PathBuf>,
    format: &ReportFormat,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut fingerprint = fingerprint_corpus_path(path)?;

    if let Some(profile_path) = profile {
        let (profile_metadata, issue_counts) =
            fingerprint_validation_issue_counts(path, profile_path)?;
        fingerprint.profile = Some(profile_metadata);
        fingerprint.validation_issue_code_counts = issue_counts;
    }

    let output = format_corpus_fingerprint(&fingerprint, format)?;
    output_options.emit(&output)?;
    Ok(())
}

fn format_corpus_diff(
    diff: &CorpusDiffReport,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
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

fn format_corpus_fingerprint(
    fingerprint: &CorpusFingerprint,
    format: &ReportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
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
        parse_mllp(&contents)?
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
        write_mllp(&ack_message)
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

    val_command(
        &file_path,
        &profile_path,
        mllp,
        detailed,
        &report,
        summary,
        &OutputOptions::new(None, false, false),
    )
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
