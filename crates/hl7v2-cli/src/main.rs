//! Command-line interface for HL7 v2 processing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::cast_precision_loss,
    clippy::exit,
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
use hl7v2::synthetic::corpus::{CorpusSummary, summarize_corpus_path};
use hl7v2::synthetic::generate::{Template, generate};
use hl7v2::{
    AckCode as GenAckCode, Event, Message, ProfileLintReport, StreamParser, ValidationReport, ack,
    get, is_mllp_framed, lint_profile_yaml, load_profile, load_profile_checked, normalize, parse,
    parse_mllp, to_json, validate, wrap_mllp, write, write_mllp,
};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Duration;
mod config;
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
            ProfileCommands::Lint { profile, report } => profile_lint_command(profile, report),
        },
        Commands::Corpus { command } => match command {
            CorpusCommands::Summarize { path, format } => corpus_summarize_command(path, format),
        },
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
        return Err(std::io::Error::other("doctor reported failed checks").into());
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
            if validation_report.valid {
                println!("Validation passed: No issues found");
            } else if detailed {
                println!("Validation issues found:");
                for issue in &validation_report.issues {
                    let path = issue.path.as_deref().unwrap_or("message");
                    println!(
                        "  - {} {} {}: {}",
                        issue.severity.as_str(),
                        issue.code,
                        path,
                        issue.message
                    );
                }
            } else {
                println!(
                    "Validation failed: {} issues found",
                    validation_report.issue_count
                );
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
        println!("  Segments: {}", validation_report.segment_count);
        println!("  Issues found: {}", validation_report.issue_count);
        display_performance_stats(&monitor);
    }

    // Exit with error code if validation failed
    if !validation_report.valid {
        std::process::exit(1);
    }

    Ok(())
}

fn profile_lint_command(
    profile: &Path,
    report: &ReportFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile_yaml = fs::read_to_string(profile)?;
    let lint_report = lint_profile_yaml(&profile_yaml);
    let output = format_profile_lint_report(profile, &lint_report, report)?;
    println!("{}", output);

    if !lint_report.valid {
        return Err(std::io::Error::other("profile lint reported errors").into());
    }

    Ok(())
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
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = summarize_corpus_path(path)?;
    let output = format_corpus_summary(&summary, format)?;
    println!("{}", output);
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
