//! Command-line interface for HL7 v2 processing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::indexing_slicing,
    clippy::unchecked_time_subtraction,
    clippy::uninlined_format_args,
    clippy::unnecessary_debug_formatting,
    reason = "pre-existing CLI reporting and table-rendering debt is tracked in policy/clippy-debt.toml"
)]
#![cfg_attr(
    test,
    expect(
        clippy::expect_used,
        reason = "pre-existing CLI config tests use static fixture expects; cleanup is tracked in policy/clippy-debt.toml"
    )
)]

use clap::Parser;
use hl7v2::synthetic::corpus::compute_sha256;
use hl7v2::synthetic::generate::{Template, generate};
use hl7v2::{
    AckCode as GenAckCode, Event, Message, Profile, ProfileLintIssue, ProfileLintReport,
    StreamParser, ValidationReport, ValidationReportProfileIdentity, ValidationReportV2, ack, get,
    is_mllp_framed, lint_profile_yaml, load_profile, load_profile_checked, normalize, parse,
    parse_mllp, to_json, unwrap_mllp, validate, wrap_mllp, write, write_mllp,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;
mod cli;
mod config;
mod corpus;
mod errors;
mod evidence_bundle;
#[cfg(test)]
use corpus::{
    diff_command as corpus_diff_command, fingerprint_command as corpus_fingerprint_command,
    format_corpus_diff, format_corpus_fingerprint, format_corpus_summary,
    summarize_command as corpus_summarize_command,
};
mod monitor;
mod output;

mod serve;

use cli::{
    AckCode, AckMode, Cli, Commands, CorpusCommands, DOCTOR_BUILTIN_SAMPLE, ProfileCommands,
    RedactFormat, ReportFormat, SampleType, ServerMode,
};
use errors::{CliFailure, classify_cli_error};
pub(crate) use output::OutputOptions;
#[cfg(test)]
mod tests;

struct ValCommandOptions<'a> {
    mllp: bool,
    detailed: bool,
    report: &'a ReportFormat,
    schema_version: u8,
    summary: bool,
}

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

    fn to_v2(&self) -> DoctorReportV2<'_> {
        DoctorReportV2 {
            schema_version: "2",
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            report: self,
        }
    }
}

#[derive(serde::Serialize)]
struct DoctorReportV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    #[serde(flatten)]
    report: &'a DoctorReport,
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
struct ProfileExplainReportV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    #[serde(flatten)]
    report: &'a ProfileExplainReport,
}

impl ProfileExplainReport {
    fn to_v2(&self) -> ProfileExplainReportV2<'_> {
        ProfileExplainReportV2 {
            schema_version: "2",
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            report: self,
        }
    }
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
    required: bool,
    repetition: bool,
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
struct ProfileTestReportV2<'a> {
    schema_version: &'static str,
    tool_name: &'static str,
    tool_version: &'static str,
    #[serde(flatten)]
    report: &'a ProfileTestReport,
}

impl ProfileTestReport {
    fn to_v2(&self) -> ProfileTestReportV2<'_> {
        ProfileTestReportV2 {
            schema_version: "2",
            tool_name: "hl7v2-cli",
            tool_version: env!("CARGO_PKG_VERSION"),
            report: self,
        }
    }
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

#[tokio::main]
async fn main() {
    // Initialize tracing for server mode
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse_from(normalize_cli_args(std::env::args().collect()));

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
            schema_version,
            summary,
            output,
            quiet,
            no_color,
        } => val_command(
            input,
            profile,
            &ValCommandOptions {
                mllp: *mllp,
                detailed: *detailed,
                report,
                schema_version: *schema_version,
                summary: *summary,
            },
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
            schema_version,
            output,
            quiet,
            no_color,
        } => doctor_command(
            sample.as_ref(),
            profile.as_ref(),
            server_url.as_deref(),
            format,
            *schema_version,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Sample {
            sample_type,
            output,
            quiet,
            no_color,
        } => sample_command(
            *sample_type,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::ValidateSample {
            sample_type,
            profile,
            report,
            schema_version,
            output,
            quiet,
            no_color,
        } => validate_sample_command(
            *sample_type,
            profile,
            report,
            *schema_version,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Profile { command } => match command {
            ProfileCommands::Lint {
                profile,
                report,
                schema_version,
                output,
                quiet,
                no_color,
            } => profile_lint_command(
                profile,
                report,
                *schema_version,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            ProfileCommands::Explain {
                profile,
                format,
                schema_version,
                output,
                quiet,
                no_color,
            } => profile_explain_command(
                profile,
                format,
                *schema_version,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            ProfileCommands::Test {
                profile,
                fixtures,
                report,
                schema_version,
                output,
                quiet,
                no_color,
            } => profile_test_command(
                profile,
                fixtures,
                report,
                *schema_version,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
        },
        Commands::Corpus { command } => match command {
            CorpusCommands::Summarize {
                path,
                format,
                schema_version,
                output,
                quiet,
                no_color,
            } => corpus::summarize_command(
                path,
                format,
                *schema_version,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            CorpusCommands::Fingerprint {
                path,
                profile,
                format,
                schema_version,
                output,
                quiet,
                no_color,
            } => corpus::fingerprint_command(
                path,
                profile.as_ref(),
                format,
                *schema_version,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
            CorpusCommands::Diff {
                before,
                after,
                profile,
                format,
                schema_version,
                output,
                quiet,
                no_color,
            } => corpus::diff_command(
                before,
                after,
                profile.as_ref(),
                format,
                *schema_version,
                &OutputOptions::new(output.as_ref(), *quiet, *no_color),
            ),
        },
        Commands::Redact {
            input,
            policy,
            format,
            schema_version,
            output,
            quiet,
            no_color,
        } => evidence_bundle::redact_command(
            input,
            policy,
            format,
            *schema_version,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Bundle {
            input,
            profile,
            redact_policy,
            out,
            schema_version,
            output,
            quiet,
            no_color,
        } => evidence_bundle::bundle_command(
            input,
            profile,
            redact_policy,
            out,
            *schema_version,
            &OutputOptions::new(output.as_ref(), *quiet, *no_color),
        ),
        Commands::Replay {
            bundle,
            format,
            schema_version,
            output,
            quiet,
            no_color,
        } => evidence_bundle::replay_command(
            bundle,
            format,
            *schema_version,
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

fn normalize_cli_args(mut args: Vec<String>) -> Vec<String> {
    if args.get(1).is_some_and(|arg| arg == "diff") {
        args.insert(1, "corpus".to_string());
    }
    args
}

#[cfg(test)]
mod normalize_cli_args_tests {
    use super::normalize_cli_args;

    #[test]
    fn test_diff_alias_expands_to_corpus_diff() {
        let args = normalize_cli_args(vec![
            "hl7v2-cli".to_string(),
            "diff".to_string(),
            "before".to_string(),
            "after".to_string(),
        ]);

        assert_eq!(
            args,
            vec![
                "hl7v2-cli".to_string(),
                "corpus".to_string(),
                "diff".to_string(),
                "before".to_string(),
                "after".to_string(),
            ]
        );
    }

    #[test]
    fn test_non_diff_command_passthrough() {
        let args = normalize_cli_args(vec![
            "hl7v2-cli".to_string(),
            "parse".to_string(),
            "file.hl7".to_string(),
        ]);

        assert_eq!(
            args,
            vec![
                "hl7v2-cli".to_string(),
                "parse".to_string(),
                "file.hl7".to_string(),
            ]
        );
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
    schema_version: u8,
    output_options: &OutputOptions<'_>,
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

    let output = format_doctor_report(&report, format, schema_version)?;
    output_options.emit(&output)?;

    if report.has_errors() {
        return Err(CliFailure::check_failed("doctor reported failed checks"));
    }

    Ok(())
}

fn sample_command(
    sample_type: SampleType,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    output_options.emit(sample_type.message())
}

fn validate_sample_command(
    sample_type: SampleType,
    profile: &PathBuf,
    report: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *report == ReportFormat::Text {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "validation report schema v2 is available only with --report json or --report yaml",
        )));
    }

    let message = parse(sample_type.message().as_bytes())?;
    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile(&profile_yaml)?;
    let issues = validate(&message, &loaded_profile);
    let validation_report = ValidationReport::from_issues(
        &message,
        Some(profile.to_string_lossy().to_string()),
        issues,
    );

    let output = match report {
        ReportFormat::Json if schema_version == 2 => {
            let report_v2 = validation_report_v2_for_cli(
                &validation_report,
                profile,
                &profile_yaml,
                &loaded_profile,
            );
            serde_json::to_string_pretty(&report_v2)?
        }
        ReportFormat::Yaml if schema_version == 2 => {
            let report_v2 = validation_report_v2_for_cli(
                &validation_report,
                profile,
                &profile_yaml,
                &loaded_profile,
            );
            serde_yaml::to_string(&report_v2)?
        }
        ReportFormat::Json => serde_json::to_string_pretty(&validation_report)?,
        ReportFormat::Yaml => serde_yaml::to_string(&validation_report)?,
        ReportFormat::Text => {
            if validation_report.valid {
                "Validation passed: No issues found".to_string()
            } else {
                format!(
                    "Validation failed: {} issues found",
                    validation_report.issue_count
                )
            }
        }
    };
    output_options.emit(&output)?;

    if !validation_report.valid {
        return Err(CliFailure::check_failed("sample validation failed"));
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
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => {
            Ok(serde_json::to_string_pretty(&report.to_v2())?)
        }
        ReportFormat::Yaml if schema_version == 2 => Ok(serde_yaml::to_string(&report.to_v2())?),
        ReportFormat::Text if schema_version == 2 => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "doctor report schema v2 is available only with --format json or --format yaml",
        )
        .into()),
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

    // Normalize the message. Without canonical delimiter rewriting, preserve the
    // validated wire payload instead of parse/write normalizing escape bytes.
    let original_bytes = if mllp_in {
        unwrap_mllp(&contents)?.to_vec()
    } else {
        contents.clone()
    };
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
    options: &ValCommandOptions<'_>,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if options.schema_version == 2 && *options.report == ReportFormat::Text {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "validation report schema v2 is available only with --report json or --report yaml",
        )));
    }

    let mut monitor = monitor::PerformanceMonitor::new();

    // Read the HL7 message file
    let contents = fs::read(input)?;
    let file_size = contents.len();

    let read_time = monitor.elapsed();
    monitor.record_metric("File read", read_time);

    // Parse the HL7 message
    let message = if options.mllp {
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

    let output = match options.report {
        ReportFormat::Json if options.schema_version == 2 => {
            let report_v2 = validation_report_v2_for_cli(
                &validation_report,
                profile,
                &profile_yaml,
                &loaded_profile,
            );
            serde_json::to_string_pretty(&report_v2)?
        }
        ReportFormat::Yaml if options.schema_version == 2 => {
            let report_v2 = validation_report_v2_for_cli(
                &validation_report,
                profile,
                &profile_yaml,
                &loaded_profile,
            );
            serde_yaml::to_string(&report_v2)?
        }
        ReportFormat::Json => serde_json::to_string_pretty(&validation_report)?,
        ReportFormat::Yaml => serde_yaml::to_string(&validation_report)?,
        ReportFormat::Text => {
            // Print validation results in text format
            if validation_report.valid {
                "Validation passed: No issues found".to_string()
            } else if options.detailed {
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
    if options.summary && *options.report == ReportFormat::Text {
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

fn validation_report_v2_for_cli(
    report: &ValidationReport,
    profile_path: &Path,
    profile_yaml: &str,
    loaded_profile: &Profile,
) -> ValidationReportV2 {
    let profile_label = profile_display_label(profile_path);
    let mut report_v2 = report.to_v2(
        "hl7v2-cli",
        env!("CARGO_PKG_VERSION"),
        Some(ValidationReportProfileIdentity {
            label: profile_label.clone(),
            message_structure: Some(loaded_profile.message_structure.clone()),
            version: Some(loaded_profile.version.clone()),
            sha256: Some(compute_sha256(profile_yaml)),
        }),
    );
    report_v2.profile = Some(profile_label);
    report_v2
}

fn profile_display_label(profile_path: &Path) -> String {
    profile_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "profile".to_string())
}

fn profile_lint_command(
    profile: &Path,
    report: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *report == ReportFormat::Text {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "profile lint schema version is only available with --report json or --report yaml",
        )
        .into());
    }

    let profile_yaml = fs::read_to_string(profile)?;
    let lint_report = lint_profile_yaml(&profile_yaml);
    let output = format_profile_lint_report(profile, &lint_report, report, schema_version)?;
    output_options.emit(&output)?;

    if !lint_report.valid {
        return Err(CliFailure::check_failed("profile lint reported errors"));
    }

    Ok(())
}

fn profile_explain_command(
    profile: &Path,
    format: &ReportFormat,
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *format == ReportFormat::Text {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "profile explain schema version is only available with --format json or --format yaml",
        )
        .into());
    }

    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile_checked(&profile_yaml)?;
    let lint_report = lint_profile_yaml(&profile_yaml);
    let explain_report =
        build_profile_explain_report(profile, &profile_yaml, &loaded_profile, &lint_report);
    let output = format_profile_explain_report(&explain_report, format, schema_version)?;
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
                required: segment.required,
                repetition: segment.repetition,
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
                let table_id =
                    valueset_hl7_table_id(valueset.name.as_str(), valueset.hl7_table.as_deref());
                let table_code_count = table_code_counts.get(table_id).copied().unwrap_or(0);
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

fn valueset_hl7_table_id<'a>(name: &'a str, hl7_table: Option<&'a str>) -> &'a str {
    hl7_table
        .filter(|table_id| !table_id.trim().is_empty())
        .unwrap_or(name)
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
    schema_version: u8,
    output_options: &OutputOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if schema_version == 2 && *report == ReportFormat::Text {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "profile test schema version is only available with --report json or --report yaml",
        )
        .into());
    }

    let profile_yaml = fs::read_to_string(profile)?;
    let loaded_profile = load_profile_checked(&profile_yaml)?;
    let test_report = run_profile_fixture_tests(profile, fixtures, &loaded_profile)?;
    let output = format_profile_test_report(&test_report, report, schema_version)?;
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
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => {
            Ok(serde_json::to_string_pretty(&report.to_v2())?)
        }
        ReportFormat::Yaml if schema_version == 2 => Ok(serde_yaml::to_string(&report.to_v2())?),
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
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => {
            Ok(serde_json::to_string_pretty(&report.to_v2())?)
        }
        ReportFormat::Yaml if schema_version == 2 => Ok(serde_yaml::to_string(&report.to_v2())?),
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Yaml => Ok(serde_yaml::to_string(report)?),
        ReportFormat::Text => {
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
                format_profile_explain_segment_list(&report.segments)
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

fn format_profile_explain_segment_list(segments: &[ProfileExplainSegment]) -> String {
    if segments.is_empty() {
        return "<none>".to_string();
    }

    segments
        .iter()
        .map(|segment| {
            let mut flags = Vec::new();
            if segment.required {
                flags.push("required");
            }
            if segment.repetition {
                flags.push("repeating");
            }

            if flags.is_empty() {
                segment.id.clone()
            } else {
                format!("{} ({})", segment.id, flags.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_string_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(", ")
    }
}

fn format_profile_lint_report(
    profile: &Path,
    report: &ProfileLintReport,
    format: &ReportFormat,
    schema_version: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        ReportFormat::Json if schema_version == 2 => Ok(serde_json::to_string_pretty(
            &report.to_v2("hl7v2-cli", env!("CARGO_PKG_VERSION")),
        )?),
        ReportFormat::Yaml if schema_version == 2 => Ok(serde_yaml::to_string(
            &report.to_v2("hl7v2-cli", env!("CARGO_PKG_VERSION")),
        )?),
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
                let field_number = if segment_id == "MSH" {
                    field_idx + 2
                } else {
                    field_idx + 1
                };
                let path = format!("{}.{}", segment_id, field_number);
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
                println!();
                println!("Processing:");
                println!("  parse <file> [options]  - Parse an HL7 message");
                println!("  norm <file> [options]   - Normalize an HL7 message");
                println!("  ack <file> [options]    - Generate an ACK for an HL7 message");
                println!();
                println!("Validation:");
                println!("  val <file> <profile> [options] - Validate an HL7 message");
                println!();
                println!("Generation:");
                println!("  gen <profile> [options] - Generate synthetic messages");
                println!();
                println!("System:");
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
        &ValCommandOptions {
            mllp,
            detailed,
            report: &report,
            schema_version: 1,
            summary,
        },
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
