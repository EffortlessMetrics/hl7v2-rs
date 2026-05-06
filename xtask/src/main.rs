//! Workspace task runner for repository automation and release checks.

use anyhow::{Result, anyhow};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};
use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development automation tasks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all checks (format, lint, test)
    Gate {
        /// Run in check mode (no mutation, strict CI parity)
        #[arg(long)]
        check: bool,
        /// Only check crates that have changed
        #[arg(long)]
        changed: bool,
        /// Run only specific check (fmt, clippy, test)
        #[arg(long)]
        only: Option<String>,
    },
    /// Fix formatting and common clippy issues
    LintFix,
    /// Setup development environment (git hooks, etc.)
    Setup,
    /// Audit dependencies for vulnerabilities and license compliance
    Audit,
    /// Check for outdated dependencies
    Outdated,
    /// Print the crates.io publish order for workspace crates
    PublishPlan {
        /// Resume from this crate name
        #[arg(long)]
        from: Option<String>,
    },
    /// Publish workspace crates to crates.io in dependency order
    Publish {
        /// Resume from this crate name
        #[arg(long)]
        from: Option<String>,
        /// Confirm that this should publish to crates.io
        #[arg(long)]
        yes: bool,
        /// Retry attempts for crates.io index propagation or transient failures
        #[arg(long, default_value_t = 10)]
        retry_attempts: u32,
        /// Delay between retries, and between successful crate publishes
        #[arg(long, default_value_t = 30)]
        retry_delay_secs: u64,
    },
    /// Scaffold a new microcrate
    Scaffold {
        /// Name of the crate (without hl7v2- prefix)
        name: String,
        /// Description of the crate
        #[arg(long)]
        description: Option<String>,
    },
    /// Generate and open documentation
    Docs {
        /// Don't open in browser
        #[arg(long)]
        no_open: bool,
    },
    /// Git pre-commit hook: lint-fix staged Rust/Cargo files
    HookPreCommit,
    /// Git pre-push hook: run full gate checks
    HookPrePush,
    /// Verify the workspace Clippy policy, lint inheritance, and debt ledger
    CheckLintPolicy,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gate {
            check,
            changed,
            only,
        } => gate(check, changed, only)?,
        Commands::LintFix => lint_fix()?,
        Commands::Setup => setup()?,
        Commands::Audit => audit()?,
        Commands::Outdated => outdated()?,
        Commands::PublishPlan { from } => publish_plan(from)?,
        Commands::Publish {
            from,
            yes,
            retry_attempts,
            retry_delay_secs,
        } => publish(from, yes, retry_attempts, retry_delay_secs)?,
        Commands::Scaffold { name, description } => scaffold(&name, description)?,
        Commands::Docs { no_open } => docs(no_open)?,
        Commands::HookPreCommit => hook_pre_commit()?,
        Commands::HookPrePush => hook_pre_push()?,
        Commands::CheckLintPolicy => check_lint_policy()?,
    }

    Ok(())
}

fn gate(check: bool, changed_only: bool, only: Option<String>) -> Result<()> {
    println!("🚀 Running gate checks...");

    let (changed_only, crates) = if changed_only {
        match get_changed_scope()? {
            ChangedScope::Crates(c) => (true, c),
            ChangedScope::Workspace => {
                println!("Non-crate files changed. Running full workspace gate.");
                (false, vec![])
            }
            ChangedScope::None => {
                println!("No files changed. Skipping checks.");
                return Ok(());
            }
        }
    } else {
        (false, vec![])
    };

    let run_fmt = only.as_deref().is_none_or(|s| s == "fmt");
    let run_clippy = only.as_deref().is_none_or(|s| s == "clippy");
    let run_test = only.as_deref().is_none_or(|s| s == "test");

    if only.is_none() {
        println!("Checking lint policy...");
        check_lint_policy()?;
    }

    if run_fmt {
        if check {
            println!("Checking formatting...");
            run_command("cargo", &["fmt", "--all", "--", "--check"])?;
        } else {
            println!("Formatting code...");
            run_command("cargo", &["fmt", "--all"])?;
        }
    }

    // Warm graph (huge speed win in big workspaces)
    if run_clippy || run_test {
        println!("Warming dependency graph...");
        let mut check_args = vec!["check", "--workspace", "--all-targets", "--all-features"];
        if changed_only {
            check_args.retain(|&a| a != "--workspace");
            for c in &crates {
                check_args.push("-p");
                check_args.push(c);
            }
        }
        run_command("cargo", &check_args)?;
    }

    if run_clippy {
        println!("Running clippy...");
        let mut args = vec!["clippy", "--all-targets", "--all-features"];
        if changed_only {
            for c in &crates {
                args.push("-p");
                args.push(c);
            }
        } else {
            args.push("--workspace");
        }
        args.extend_from_slice(&["--", "-D", "warnings"]);
        run_command("cargo", &args)?;
    }

    if run_test {
        println!("Compiling tests (no-run)...");
        let mut args = vec!["test", "--all-targets", "--all-features", "--no-run"];
        if changed_only {
            for c in &crates {
                args.push("-p");
                args.push(c);
            }
        } else {
            args.push("--workspace");
        }
        run_command("cargo", &args)?;
    }

    println!("✅ Gate checks passed!");
    Ok(())
}

fn lint_fix() -> Result<()> {
    println!("🛠️  Fixing lints and formatting...");

    println!("Formatting code...");
    run_command("cargo", &["fmt", "--all"])?;

    println!("Applying clippy fixes (best-effort)...");
    // Best-effort fix pass: do NOT use -D warnings here
    // Also: allow failure; we still do a strict verify after.
    let _ = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--fix",
            "--allow-dirty",
            "--allow-staged",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    println!("Verifying clippy (strict)...");
    run_command(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    println!("✅ Lint fixes applied!");
    Ok(())
}

fn setup() -> Result<()> {
    println!("⚙️  Setting up repository hooks...");

    run_command_git(&["config", "core.hooksPath", ".githooks"])?;

    #[cfg(unix)]
    {
        println!("Marking hooks as executable...");
        let root = env::current_dir()?;
        let hooks_dir = root.join(".githooks");
        if hooks_dir.exists() {
            for entry in fs::read_dir(hooks_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&path, perms)?;
                }
            }
        }
    }

    // Check for required tools
    let tools = ["cargo-deny", "cargo-audit", "cargo-nextest", "just"];
    for tool in tools {
        if !command_exists(tool) {
            println!(
                "Note: '{}' not found. Consider installing it for full DevEx.",
                tool
            );
        }
    }

    println!("✅ Setup complete!");
    Ok(())
}

fn audit() -> Result<()> {
    println!("🔍 Auditing dependencies...");

    if command_exists("cargo-audit") {
        println!("Running cargo-audit...");
        run_command("cargo", &["audit"])?;
    } else {
        println!("Warning: cargo-audit not found. Skipping vulnerability scan.");
    }

    if command_exists("cargo-deny") {
        println!("Running cargo-deny...");
        run_command("cargo", &["deny", "check"])?;
    } else {
        println!("Warning: cargo-deny not found. Skipping license/ban check.");
    }

    Ok(())
}

fn outdated() -> Result<()> {
    println!("📦 Checking for outdated dependencies...");

    if command_exists("cargo-outdated") {
        run_command("cargo", &["outdated", "--workspace", "--depth", "1"])?;
    } else {
        println!("Error: cargo-outdated not found. Install with 'cargo install cargo-outdated'.");
    }

    Ok(())
}

fn publish_plan(from: Option<String>) -> Result<()> {
    let crates = publish_order(from.as_deref())?;

    println!("📋 crates.io publish order");
    for (index, crate_name) in crates.iter().enumerate() {
        println!("{:>2}. {}", index + 1, crate_name);
    }

    println!();
    println!("Execute with:");
    if let Some(start) = crates.first() {
        println!("  cargo run -p xtask -- publish --yes --from {}", start);
    } else {
        println!("  cargo run -p xtask -- publish --yes");
    }

    Ok(())
}

fn publish(
    from: Option<String>,
    yes: bool,
    retry_attempts: u32,
    retry_delay_secs: u64,
) -> Result<()> {
    if !yes {
        return Err(anyhow!(
            "Refusing to publish without --yes. Run `cargo run -p xtask -- publish-plan` first."
        ));
    }

    let crates = publish_order(from.as_deref())?;
    if env::var_os("CARGO_REGISTRY_TOKEN").is_none() {
        println!(
            "Warning: CARGO_REGISTRY_TOKEN is not set; cargo publish will use local cargo credentials if available."
        );
    }

    println!("🚢 Publishing {} crates to crates.io...", crates.len());
    for (index, crate_name) in crates.iter().enumerate() {
        publish_crate(crate_name, retry_attempts, retry_delay_secs)?;
        if index + 1 < crates.len() && retry_delay_secs > 0 {
            println!(
                "Waiting {}s for crates.io index propagation before continuing...",
                retry_delay_secs
            );
            sleep(Duration::from_secs(retry_delay_secs));
        }
    }

    println!("✅ Publish sequence complete!");
    Ok(())
}

fn publish_crate(crate_name: &str, retry_attempts: u32, retry_delay_secs: u64) -> Result<()> {
    let max_attempts = retry_attempts.max(1);
    for attempt in 1..=max_attempts {
        println!(
            "Publishing {} (attempt {}/{})...",
            crate_name, attempt, max_attempts
        );

        let output = Command::new("cargo")
            .args(["publish", "-p", crate_name, "--locked"])
            .output()?;

        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;

        if !stdout.is_empty() {
            print!("{stdout}");
        }
        if !stderr.is_empty() {
            eprint!("{stderr}");
        }

        if output.status.success() {
            return Ok(());
        }

        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("is already uploaded") || combined.contains("already exists") {
            println!(
                "Skipping {} because this version is already present on crates.io.",
                crate_name
            );
            return Ok(());
        }

        let retryable = combined.contains("no matching package named")
            || combined.contains("failed to get successful HTTP response")
            || combined.contains("network failure seems to have happened")
            || combined.contains("Timeout was reached")
            || combined.contains("429 Too Many Requests")
            || combined.contains("SSL connect error");

        if retryable && attempt < max_attempts {
            println!(
                "Retryable publish failure for {}. Waiting {}s before retry...",
                crate_name, retry_delay_secs
            );
            sleep(Duration::from_secs(retry_delay_secs));
            continue;
        }

        return Err(anyhow!(
            "Failed to publish {} after {} attempt(s).",
            crate_name,
            attempt
        ));
    }

    unreachable!("publish loop always returns or errors")
}

fn scaffold(name: &str, description: Option<String>) -> Result<()> {
    let crate_name = if name.starts_with("hl7v2-") {
        name.to_string()
    } else {
        format!("hl7v2-{}", name)
    };

    println!("🏗️  Scaffolding new microcrate: {}...", crate_name);

    let root = env::current_dir()?;
    let crate_path = root.join("crates").join(&crate_name);

    if crate_path.exists() {
        return Err(anyhow!("Crate {} already exists", crate_name));
    }

    fs::create_dir_all(crate_path.join("src"))?;
    fs::create_dir_all(crate_path.join("tests"))?;

    // Cargo.toml
    let description = description.unwrap_or_else(|| format!("HL7 v2 {} functionality", name));
    let cargo_toml = format!(
        r#"[package]
name = "{crate_name}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
description = "{description}"
license.workspace = true
repository.workspace = true
readme = "README.md"
keywords = ["hl7", "healthcare"]
categories = ["parser-implementations"]

[lints]
workspace = true

[dependencies]
hl7v2-model = {{ path = "../hl7v2-model" }}
thiserror = {{ workspace = true }}

[dev-dependencies]
hl7v2-test-utils = {{ path = "../hl7v2-test-utils" }}
"#
    );
    fs::write(crate_path.join("Cargo.toml"), cargo_toml)?;

    // README.md
    let readme = format!(
        r"# {crate_name}

{description}

## Usage

```rust
use {crate_name}::*;
```
"
    );
    fs::write(crate_path.join("README.md"), readme)?;

    // CLAUDE.md
    let claude = format!(
        r"# {crate_name} Development

## Build & Test

```bash
cargo build -p {crate_name}
cargo test -p {crate_name}
cargo clippy -p {crate_name} -- -D warnings
```
"
    );
    fs::write(crate_path.join("CLAUDE.md"), claude)?;

    // src/lib.rs
    fs::write(
        crate_path.join("src").join("lib.rs"),
        r#"//! Main library file
    
pub fn example() {
    println!("Hello from {}!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
"#,
    )?;

    println!("✅ Crate {} scaffolded successfully!", crate_name);
    println!("Don't forget to run 'cargo build' to update the workspace.");

    Ok(())
}

fn hook_pre_commit() -> Result<()> {
    let staged = git_output(&["diff", "--cached", "--name-only", "--diff-filter=ACMR"])?;
    let staged_files: Vec<&str> = staged
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let has_relevant = staged_files
        .iter()
        .any(|f| f.ends_with(".rs") || f.ends_with("Cargo.toml") || f.ends_with("Cargo.lock"));

    if !has_relevant {
        return Ok(());
    }

    println!("pre-commit: lint-fix");
    lint_fix()?;

    // Restage the files that were originally staged (in chunks to avoid command-line length limits)
    for chunk in staged_files.chunks(50) {
        let mut args: Vec<&str> = vec!["add"];
        args.extend_from_slice(chunk);
        run_command_git(&args)?;
    }

    Ok(())
}

fn hook_pre_push() -> Result<()> {
    println!("pre-push: gate --check");
    gate(true, false, None)
}

fn check_lint_policy() -> Result<()> {
    println!("🔎 Checking Clippy lint policy...");

    let root_cargo = fs::read_to_string("Cargo.toml")?;
    let policy = fs::read_to_string("policy/clippy-lints.toml")?;
    let debt = fs::read_to_string("policy/clippy-debt.toml")?;
    let clippy = fs::read_to_string("clippy.toml")?;

    let workspace_msrv = required_value(&root_cargo, "rust-version")?;
    let policy_msrv = required_value(&policy, "msrv")?;
    if workspace_msrv != policy_msrv {
        return Err(anyhow!(
            "workspace MSRV {workspace_msrv} does not match policy MSRV {policy_msrv}"
        ));
    }

    require_contains(&root_cargo, "[workspace.lints.rust]", "root Cargo.toml")?;
    require_contains(&root_cargo, "[workspace.lints.clippy]", "root Cargo.toml")?;
    require_contains(
        &policy,
        "panic_free_tests = true",
        "policy/clippy-lints.toml",
    )?;
    require_contains(
        &policy,
        "allow_test_carveouts = false",
        "policy/clippy-lints.toml",
    )?;
    require_contains(
        &policy,
        "suppression_style = \"expect-with-reason\"",
        "policy/clippy-lints.toml",
    )?;

    for carveout in [
        "allow-unwrap-in-tests",
        "allow-expect-in-tests",
        "allow-panic-in-tests",
        "allow-indexing-slicing-in-tests",
        "allow-dbg-in-tests",
    ] {
        if clippy.contains(carveout) && !comment_only_mentions(&clippy, carveout) {
            return Err(anyhow!(
                "clippy.toml must not configure test carveout `{carveout}`"
            ));
        }
    }

    for planned in [
        "clippy::same_length_and_capacity",
        "clippy::manual_ilog2",
        "clippy::decimal_bitwise_operands",
        "clippy::needless_type_cast",
        "clippy::disallowed_fields",
        "clippy::manual_checked_ops",
        "clippy::manual_take",
        "clippy::manual_pop_if",
        "clippy::duration_suboptimal_units",
        "clippy::unnecessary_trailing_comma",
    ] {
        require_contains(&policy, planned, "policy/clippy-lints.toml")?;
        let cargo_key = planned.trim_start_matches("clippy::");
        if root_cargo.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with(cargo_key) && trimmed.contains('=')
        }) {
            return Err(anyhow!(
                "planned lint `{planned}` is active in Cargo.toml before its recorded MSRV flip"
            ));
        }
    }

    for lint in active_policy_lints(&policy) {
        let cargo_key = lint.trim_start_matches("clippy::");
        if !root_cargo.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with(cargo_key) && trimmed.contains('=')
        }) {
            return Err(anyhow!(
                "active policy lint `{lint}` is missing from root Cargo.toml"
            ));
        }
    }

    for manifest in cargo_manifests()? {
        let manifest_text = fs::read_to_string(&manifest)?;
        if !manifest_text.contains("[lints]") || !manifest_text.contains("workspace = true") {
            return Err(anyhow!(
                "{} must inherit workspace lints with `[lints] workspace = true`",
                manifest.display()
            ));
        }
    }

    validate_debt(&debt)?;
    validate_suppressions(Path::new("."))?;

    println!("✅ Lint policy check passed");
    Ok(())
}

fn require_contains(haystack: &str, needle: &str, label: &str) -> Result<()> {
    if haystack.contains(needle) {
        Ok(())
    } else {
        Err(anyhow!("{label} must contain `{needle}`"))
    }
}

fn required_value(text: &str, key: &str) -> Result<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.trim_start().strip_prefix('='))
                .map(|value| value.trim().trim_matches('"').to_string())
        })
        .ok_or_else(|| anyhow!("missing required `{key}` value"))
}

fn active_policy_lints(policy: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_active = false;

    for line in policy.lines().map(str::trim) {
        if line == "[[lint]]" {
            if current_active {
                if let Some(name) = current_name.take() {
                    result.push(name);
                }
            }
            current_name = None;
            current_active = false;
        } else if let Some(value) = line.strip_prefix("name = ") {
            current_name = Some(value.trim_matches('"').to_string());
        } else if line == "status = \"active\"" {
            current_active = true;
        }
    }

    if current_active {
        if let Some(name) = current_name {
            result.push(name);
        }
    }

    result
}

fn cargo_manifests() -> Result<Vec<PathBuf>> {
    let mut manifests = vec![PathBuf::from("Cargo.toml")];
    for entry in fs::read_dir("crates")? {
        let entry = entry?;
        let path = entry.path().join("Cargo.toml");
        if path.exists() {
            manifests.push(path);
        }
    }
    manifests.push(PathBuf::from("xtask/Cargo.toml"));
    manifests.sort();
    Ok(manifests)
}

fn comment_only_mentions(text: &str, needle: &str) -> bool {
    text.lines()
        .filter(|line| line.contains(needle))
        .all(|line| line.trim_start().starts_with('#'))
}

fn validate_debt(debt: &str) -> Result<()> {
    let current_date = current_utc_date()?;
    let mut in_debt = false;
    let mut fields = BTreeSet::new();
    let mut expires: Option<String> = None;

    for line in debt.lines().map(str::trim) {
        if line == "[[debt]]" {
            if in_debt {
                require_debt_fields(&fields)?;
                require_unexpired_debt(expires.as_deref(), &current_date)?;
            }
            in_debt = true;
            fields.clear();
            expires = None;
        } else if in_debt {
            for field in ["lint", "path", "owner", "reason", "expires"] {
                if line.starts_with(field) && line.contains('=') {
                    fields.insert(field.to_string());
                    if field == "expires" {
                        expires = line
                            .split_once('=')
                            .map(|(_, value)| value.trim().trim_matches('"').to_string());
                    }
                }
            }
        }
    }

    if in_debt {
        require_debt_fields(&fields)?;
        require_unexpired_debt(expires.as_deref(), &current_date)?;
    }

    Ok(())
}

fn current_utc_date() -> Result<String> {
    let output = Command::new("date").args(["-u", "+%F"]).output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to determine current UTC date for debt expiry"
        ));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn require_unexpired_debt(expires: Option<&str>, current_date: &str) -> Result<()> {
    let expires =
        expires.ok_or_else(|| anyhow!("policy/clippy-debt.toml debt entry missing `expires`"))?;
    if expires <= current_date {
        return Err(anyhow!(
            "policy/clippy-debt.toml debt entry expired on {expires}"
        ));
    }
    Ok(())
}

fn require_debt_fields(fields: &BTreeSet<String>) -> Result<()> {
    for required in ["lint", "path", "owner", "reason", "expires"] {
        if !fields.contains(required) {
            return Err(anyhow!(
                "policy/clippy-debt.toml debt entry missing `{required}`"
            ));
        }
    }
    Ok(())
}

fn validate_suppressions(root: &Path) -> Result<()> {
    for rust_file in rust_files(root)? {
        let text = fs::read_to_string(&rust_file)?;
        let lines: Vec<_> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[allow") || trimmed.starts_with("#![allow") {
                return Err(anyhow!(
                    "{}:{} uses #[allow]; use #[expect(..., reason = \"...\")] or policy debt instead",
                    rust_file.display(),
                    index + 1
                ));
            }
            if trimmed.starts_with("#[expect") || trimmed.starts_with("#![expect") {
                let attr = collect_attribute(&lines[index..]);
                if !attr.contains("reason") {
                    return Err(anyhow!(
                        "{}:{} uses #[expect] without a reason",
                        rust_file.display(),
                        index + 1
                    ));
                }
            }
        }
    }
    Ok(())
}

fn collect_attribute(lines: &[&str]) -> String {
    let mut attr = String::new();
    for line in lines {
        attr.push_str(line);
        if line.contains(']') {
            break;
        }
    }
    attr
}

fn rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if path.is_dir() {
            if matches!(name, ".git" | "target") {
                continue;
            }
            collect_rust_files(&path, files)?;
        } else if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn docs(no_open: bool) -> Result<()> {
    println!("📚 Generating documentation...");
    let mut args = vec!["doc", "--workspace", "--no-deps"];
    if !no_open {
        args.push("--open");
    }
    run_command("cargo", &args)?;
    Ok(())
}

fn publish_order(from: Option<&str>) -> Result<Vec<String>> {
    let metadata = MetadataCommand::new().exec()?;
    let packages = publishable_workspace_packages(&metadata);
    let ordered = topological_publish_order(&packages)?;

    match from {
        Some(start) => {
            let index = ordered
                .iter()
                .position(|crate_name| crate_name == start)
                .ok_or_else(|| anyhow!("Unknown publishable crate '{}'", start))?;
            Ok(ordered[index..].to_vec())
        }
        None => Ok(ordered),
    }
}

fn publishable_workspace_packages(metadata: &Metadata) -> HashMap<String, Package> {
    let workspace_members: HashSet<_> = metadata.workspace_members.iter().cloned().collect();

    metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(&pkg.id))
        .filter(|pkg| {
            pkg.publish
                .as_ref()
                .is_none_or(|registries| !registries.is_empty())
        })
        .filter(|pkg| pkg.name != "xtask" && pkg.name != "hl7v2-examples")
        .cloned()
        .map(|pkg| (pkg.name.to_string(), pkg))
        .collect()
}

fn topological_publish_order(packages: &HashMap<String, Package>) -> Result<Vec<String>> {
    let mut indegree: BTreeMap<String, usize> = packages
        .keys()
        .cloned()
        .map(|name| (name, 0usize))
        .collect();
    let mut dependents: BTreeMap<String, BTreeSet<String>> = packages
        .keys()
        .cloned()
        .map(|name| (name, BTreeSet::new()))
        .collect();

    for package in packages.values() {
        for dependency in internal_publish_dependencies(package, packages) {
            dependents
                .entry(dependency)
                .or_default()
                .insert(package.name.to_string());
            *indegree
                .get_mut(package.name.as_str())
                .expect("publishable package should have indegree entry") += 1;
        }
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(name, _)| name.clone())
        .collect();
    let mut ordered = Vec::with_capacity(packages.len());

    while let Some(next) = ready.pop_first() {
        ordered.push(next.clone());
        if let Some(children) = dependents.get(&next) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("child package should have indegree entry");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }

    if ordered.len() != packages.len() {
        let remaining: Vec<_> = indegree
            .into_iter()
            .filter_map(|(name, degree)| (degree > 0).then_some(name))
            .collect();
        return Err(anyhow!(
            "Could not derive publish order due to internal dependency cycle(s): {}",
            remaining.join(", ")
        ));
    }

    Ok(ordered)
}

fn internal_publish_dependencies(
    package: &Package,
    packages: &HashMap<String, Package>,
) -> BTreeSet<String> {
    package
        .dependencies
        .iter()
        .filter(|dep| dep.kind != DependencyKind::Development)
        .filter_map(|dep| packages.contains_key(&dep.name).then_some(dep.name.clone()))
        .collect()
}

fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}",
            cmd,
            args.join(" "),
            status.code()
        ));
    }

    Ok(())
}

fn run_command_git(args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!(
            "Git command 'git {}' failed with exit code: {:?}",
            args.join(" "),
            status.code()
        ));
    }

    Ok(())
}

fn git_output(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Git command 'git {}' failed with exit code: {:?}",
            args.join(" "),
            output.status.code()
        ));
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn command_exists(cmd: &str) -> bool {
    if cfg!(windows) {
        Command::new("where")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        let safe = cmd.replace('\'', r"'\''");
        Command::new("sh")
            .args(["-lc", &format!("command -v '{safe}' >/dev/null 2>&1")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

enum ChangedScope {
    /// Only `crates/<name>/` files changed — scoped gate possible
    Crates(Vec<String>),
    /// Non-crate files changed — full workspace gate required
    Workspace,
    /// Nothing changed
    None,
}

fn get_changed_scope() -> Result<ChangedScope> {
    let files = git_output(&["diff", "--name-only", "HEAD"])?;
    let mut changed_crates = HashSet::new();
    let mut has_non_crate_files = false;

    for line in files.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("crates/") {
            let parts: Vec<&str> = line.split('/').collect();
            if parts.len() > 1 {
                changed_crates.insert(parts[1].to_string());
            }
        } else {
            has_non_crate_files = true;
        }
    }

    if changed_crates.is_empty() && !has_non_crate_files {
        return Ok(ChangedScope::None);
    }

    if has_non_crate_files {
        return Ok(ChangedScope::Workspace);
    }

    Ok(ChangedScope::Crates(changed_crates.into_iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_order_uses_workspace_dependency_order() {
        let ordered = publish_order(None).expect("workspace publish order should resolve");

        assert!(ordered.contains(&"hl7v2-core".to_string()));
        assert!(ordered.contains(&"hl7v2".to_string()));
        assert!(ordered.contains(&"hl7v2-template-values".to_string()));
        assert!(!ordered.contains(&"xtask".to_string()));

        assert_dependency_precedes(&ordered, "hl7v2-datatype", "hl7v2-core");
        assert_dependency_precedes(&ordered, "hl7v2-core", "hl7v2");
        assert_dependency_precedes(&ordered, "hl7v2-template-values", "hl7v2-template");
    }

    #[test]
    fn publish_order_can_resume_from_a_named_crate() {
        let ordered = publish_order(None).expect("workspace publish order should resolve");
        let resumed =
            publish_order(Some("hl7v2-core")).expect("resume point should exist in workspace");
        let start = ordered
            .iter()
            .position(|crate_name| crate_name == "hl7v2-core")
            .expect("hl7v2-core should be publishable");

        assert_eq!(resumed, ordered[start..].to_vec());
    }

    fn assert_dependency_precedes(ordered: &[String], dependency: &str, dependent: &str) {
        let dependency_index = ordered
            .iter()
            .position(|crate_name| crate_name == dependency)
            .unwrap_or_else(|| panic!("{dependency} should be present in publish order"));
        let dependent_index = ordered
            .iter()
            .position(|crate_name| crate_name == dependent)
            .unwrap_or_else(|| panic!("{dependent} should be present in publish order"));

        assert!(
            dependency_index < dependent_index,
            "{dependency} should appear before {dependent}"
        );
    }
}
