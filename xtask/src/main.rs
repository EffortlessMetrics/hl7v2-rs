//! Workspace task runner for repository automation and release checks.

use anyhow::{Result, anyhow};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};
use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
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
    /// Dry-run publish workspace crates in dependency order
    PublishDryRun {
        /// Resume from this crate name
        #[arg(long)]
        from: Option<String>,
        /// Patch internal workspace crates to local paths during verification
        #[arg(long)]
        workspace_patches: bool,
        /// Include uncommitted working tree changes in the dry-run package
        #[arg(long)]
        allow_dirty: bool,
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
    /// Verify workspace lint policy, ledgers, and debt receipts
    CheckLintPolicy,
    /// Print the governed lint policy rollout and debt summary
    PolicyReport,
    /// Verify the panic-family allowlist against current source findings
    CheckNoPanicFamily {
        /// Treat staged crates as report-only (default).
        #[arg(long)]
        include_staged: bool,
    },
    /// Generate proposed no-panic allowlist entries from current findings
    NoPanic {
        #[command(subcommand)]
        action: NoPanicAction,
    },
    /// Verify the non-Rust file allowlist against tracked files
    CheckFilePolicy,
}

#[derive(Subcommand)]
enum NoPanicAction {
    /// Emit proposed allowlist entries for current findings
    Propose {
        /// Include staged (non-required) crates as well
        #[arg(long)]
        include_staged: bool,
    },
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
        Commands::PublishDryRun {
            from,
            workspace_patches,
            allow_dirty,
        } => publish_dry_run(from, workspace_patches, allow_dirty)?,
        Commands::Scaffold { name, description } => scaffold(&name, description)?,
        Commands::Docs { no_open } => docs(no_open)?,
        Commands::HookPreCommit => hook_pre_commit()?,
        Commands::HookPrePush => hook_pre_push()?,
        Commands::CheckLintPolicy => check_lint_policy()?,
        Commands::PolicyReport => policy_report()?,
        Commands::CheckNoPanicFamily { include_staged } => {
            check_no_panic_family(include_staged)?;
        }
        Commands::NoPanic { action } => match action {
            NoPanicAction::Propose { include_staged } => no_panic_propose(include_staged)?,
        },
        Commands::CheckFilePolicy => check_file_policy()?,
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

    if !changed_only {
        println!("Checking lint policy...");
        check_lint_policy()?;
        println!("Checking no-panic-family policy...");
        check_no_panic_family(false)?;
        println!("Checking non-Rust file policy...");
        check_file_policy()?;
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
    match Command::new("cargo")
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
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => println!("Best-effort clippy fix exited with status: {status}"),
        Err(error) => println!("Best-effort clippy fix could not run: {error}"),
    }

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
            println!("Note: '{tool}' not found. Consider installing it for full DevEx.");
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
        let display_index = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("publish-plan index overflow"))?;
        println!("{display_index:>2}. {crate_name}");
    }

    println!();
    println!("Execute with:");
    if let Some(start) = crates.first() {
        println!("  cargo run -p xtask -- publish --yes --from {start}");
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
        let has_next = index
            .checked_add(1)
            .is_some_and(|next_index| next_index < crates.len());
        if has_next && retry_delay_secs > 0 {
            println!(
                "Waiting {retry_delay_secs}s for crates.io index propagation before continuing..."
            );
            sleep(Duration::from_secs(retry_delay_secs));
        }
    }

    println!("✅ Publish sequence complete!");
    Ok(())
}

fn publish_dry_run(from: Option<String>, workspace_patches: bool, allow_dirty: bool) -> Result<()> {
    let metadata = MetadataCommand::new().exec()?;
    let packages = publishable_workspace_packages(&metadata);
    let ordered = topological_publish_order(&packages)?;
    let crates = resume_publish_order(&ordered, from.as_deref())?;

    println!("🧪 Dry-running crates.io publish verification");
    if workspace_patches {
        println!("Using local workspace patches for unpublished internal crates.");
    }

    for crate_name in crates {
        let config_path = if workspace_patches {
            workspace_patch_config(&crate_name, &packages)?
        } else {
            None
        };
        publish_dry_run_crate(&crate_name, config_path.as_deref(), allow_dirty)?;
    }

    println!("✅ Publish dry-run checks passed!");
    Ok(())
}

fn publish_dry_run_crate(
    crate_name: &str,
    config_path: Option<&Path>,
    allow_dirty: bool,
) -> Result<()> {
    println!("Dry-running {crate_name}...");

    let mut command = Command::new("cargo");
    command.args(["publish", "--dry-run", "-p", crate_name, "--locked"]);
    if allow_dirty {
        command.arg("--allow-dirty");
    }
    if let Some(config_path) = config_path {
        command.arg("--config").arg(config_path);
    }

    let status = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!(
            "Dry-run publish failed for {} with exit code: {:?}",
            crate_name,
            status.code()
        ));
    }

    Ok(())
}

fn workspace_patch_config(
    crate_name: &str,
    packages: &HashMap<String, Package>,
) -> Result<Option<PathBuf>> {
    let dependencies = internal_workspace_dependency_closure(crate_name, packages)?;
    if dependencies.is_empty() {
        return Ok(None);
    }

    let config_dir = env::current_dir()?
        .join("target")
        .join("hl7v2-publish-dry-run")
        .join("workspace-patches");
    fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join(format!("{crate_name}.toml"));
    let mut config = String::from("[patch.crates-io]\n");
    for dependency in dependencies {
        let package = packages
            .get(&dependency)
            .ok_or_else(|| anyhow!("dependency closure includes unknown package {dependency}"))?;
        let manifest_dir = package
            .manifest_path
            .parent()
            .ok_or_else(|| anyhow!("Package {dependency} has no manifest parent"))?;
        let path = manifest_dir.as_str().replace('\\', "/");
        config.push('"');
        config.push_str(&escape_toml_basic_string(&dependency));
        config.push_str("\" = { path = \"");
        config.push_str(&escape_toml_basic_string(&path));
        config.push_str("\" }\n");
    }

    fs::write(&config_path, config)?;
    Ok(Some(config_path))
}

fn publish_crate(crate_name: &str, retry_attempts: u32, retry_delay_secs: u64) -> Result<()> {
    let max_attempts = retry_attempts.max(1);
    for attempt in 1..=max_attempts {
        println!("Publishing {crate_name} (attempt {attempt}/{max_attempts})...");

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
            println!("Skipping {crate_name} because this version is already present on crates.io.");
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
                "Retryable publish failure for {crate_name}. Waiting {retry_delay_secs}s before retry..."
            );
            sleep(Duration::from_secs(retry_delay_secs));
            continue;
        }

        return Err(anyhow!(
            "Failed to publish {crate_name} after {attempt} attempt(s)."
        ));
    }

    Err(anyhow!(
        "publish loop ended without returning a status for {crate_name}"
    ))
}

fn scaffold(name: &str, description: Option<String>) -> Result<()> {
    let crate_name = if name.starts_with("hl7v2-") {
        name.to_string()
    } else {
        format!("hl7v2-{name}")
    };

    println!("🏗️  Scaffolding new microcrate: {crate_name}...");

    let root = env::current_dir()?;
    let crate_path = root.join("crates").join(&crate_name);

    if crate_path.exists() {
        return Err(anyhow!("Crate {crate_name} already exists"));
    }

    fs::create_dir_all(crate_path.join("src"))?;
    fs::create_dir_all(crate_path.join("tests"))?;

    // Cargo.toml
    let description = description.unwrap_or_else(|| format!("HL7 v2 {name} functionality"));
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

    println!("✅ Crate {crate_name} scaffolded successfully!");
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

    resume_publish_order(&ordered, from)
}

fn resume_publish_order(ordered: &[String], from: Option<&str>) -> Result<Vec<String>> {
    match from {
        Some(start) => {
            let index = ordered
                .iter()
                .position(|crate_name| crate_name == start)
                .ok_or_else(|| anyhow!("Unknown publishable crate '{start}'"))?;
            let resumed = ordered
                .get(index..)
                .ok_or_else(|| anyhow!("resume index for {start} is outside publish order"))?;
            Ok(resumed.to_vec())
        }
        None => Ok(ordered.to_vec()),
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
            let package_indegree = indegree
                .get_mut(package.name.as_str())
                .ok_or_else(|| anyhow!("publishable package should have indegree entry"))?;
            *package_indegree = package_indegree
                .checked_add(1)
                .ok_or_else(|| anyhow!("publish indegree overflow"))?;
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
                    .ok_or_else(|| anyhow!("child package should have indegree entry"))?;
                *degree = degree
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("publish indegree underflow"))?;
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
    internal_workspace_dependencies(package, packages, false)
}

fn internal_workspace_dependency_closure(
    crate_name: &str,
    packages: &HashMap<String, Package>,
) -> Result<BTreeSet<String>> {
    let package = packages
        .get(crate_name)
        .ok_or_else(|| anyhow!("Unknown publishable crate '{crate_name}'"))?;
    let mut dependencies = BTreeSet::new();
    let mut stack: Vec<_> = internal_workspace_dependencies(package, packages, true)
        .into_iter()
        .collect();

    while let Some(dependency) = stack.pop() {
        if dependency == crate_name || !dependencies.insert(dependency.clone()) {
            continue;
        }

        if let Some(package) = packages.get(&dependency) {
            stack.extend(internal_workspace_dependencies(package, packages, true));
        }
    }

    Ok(dependencies)
}

fn internal_workspace_dependencies(
    package: &Package,
    packages: &HashMap<String, Package>,
    include_dev: bool,
) -> BTreeSet<String> {
    package
        .dependencies
        .iter()
        .filter(|dep| include_dev || dep.kind != DependencyKind::Development)
        .filter_map(|dep| packages.contains_key(&dep.name).then_some(dep.name.clone()))
        .collect()
}

fn check_lint_policy() -> Result<()> {
    println!("🔎 Checking lint policy...");

    let root = env::current_dir()?;
    let cargo_toml = root.join("Cargo.toml");
    let policy_lints = root.join("policy/clippy-lints.toml");
    let policy_debt = root.join("policy/clippy-debt.toml");
    let clippy_toml = root.join("clippy.toml");

    let cargo_text = fs::read_to_string(&cargo_toml)?;
    let policy_text = fs::read_to_string(&policy_lints)?;

    let workspace_msrv = quoted_value_after(&cargo_text, "[workspace.package]", "rust-version")
        .ok_or_else(|| anyhow!("Cargo.toml is missing workspace.package.rust-version"))?;
    let policy_msrv = top_level_quoted_value(&policy_text, "msrv")
        .ok_or_else(|| anyhow!("policy/clippy-lints.toml is missing msrv"))?;
    if workspace_msrv != policy_msrv {
        return Err(anyhow!(
            "workspace.package.rust-version ({workspace_msrv}) must match policy/clippy-lints.toml msrv ({policy_msrv})"
        ));
    }

    let active_lints = parse_policy_lints(&policy_text, "active")?;
    let planned_lints = parse_policy_lints(&policy_text, "planned")?;
    let manifest_lints = parse_workspace_lints(&cargo_text);

    for (name, level) in &active_lints {
        match manifest_lints.get(name) {
            Some(actual) if actual == level => {}
            Some(actual) => {
                return Err(anyhow!(
                    "active lint {name} is {actual} in Cargo.toml but {level} in policy/clippy-lints.toml"
                ));
            }
            None => {
                return Err(anyhow!(
                    "active lint {name} is present in policy/clippy-lints.toml but missing from Cargo.toml"
                ));
            }
        }
    }

    for name in manifest_lints.keys() {
        if !active_lints.contains_key(name) {
            return Err(anyhow!(
                "workspace lint {name} is present in Cargo.toml but missing as an active lint in policy/clippy-lints.toml"
            ));
        }
    }

    for planned in planned_lints.keys() {
        if manifest_lints.contains_key(planned) {
            return Err(anyhow!(
                "planned lint {planned} must not be active in Cargo.toml before its activate_when_msrv gate"
            ));
        }
    }

    ensure_policy_flags(&policy_text)?;
    ensure_no_test_carveouts(&clippy_toml)?;
    ensure_workspace_lint_inheritance(&root, &policy_text)?;
    ensure_debt_receipts(&policy_debt)?;

    println!("✅ Lint policy checks passed!");
    Ok(())
}

fn policy_report() -> Result<()> {
    let root = env::current_dir()?;
    let cargo_text = fs::read_to_string(root.join("Cargo.toml"))?;
    let policy_text = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let debt_text = fs::read_to_string(root.join("policy/clippy-debt.toml"))?;

    let workspace_msrv = quoted_value_after(&cargo_text, "[workspace.package]", "rust-version")
        .ok_or_else(|| anyhow!("Cargo.toml is missing workspace.package.rust-version"))?;
    let policy_msrv = top_level_quoted_value(&policy_text, "msrv")
        .ok_or_else(|| anyhow!("policy/clippy-lints.toml is missing msrv"))?;
    let active_lints = parse_policy_lints(&policy_text, "active")?;
    let planned_lints = parse_policy_lints(&policy_text, "planned")?;
    let required_packages =
        string_array_after(&policy_text, "[rollout]", "required_inheriting_packages").ok_or_else(
            || anyhow!("policy/clippy-lints.toml is missing rollout.required_inheriting_packages"),
        )?;
    let staged_packages =
        string_array_after(&policy_text, "[rollout]", "staged_inheriting_packages").ok_or_else(
            || anyhow!("policy/clippy-lints.toml is missing rollout.staged_inheriting_packages"),
        )?;
    let debt_count = table_array_entries(&debt_text, "[[debt]]").len();

    let no_panic_text = fs::read_to_string(root.join("policy/no-panic-allowlist.toml"))?;
    let no_panic_entries = parse_no_panic_allowlist(&no_panic_text)?;
    let file_policy_text = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;
    let file_policy_entries = parse_file_policy_allowlist(&file_policy_text)?;

    let metadata = MetadataCommand::new().current_dir(&root).exec()?;
    let strict_units = collect_rust_files_for(&root, &metadata, &required_packages)?;
    let advisory_units = collect_rust_files_for(&root, &metadata, &staged_packages)?;
    let strict_findings = scan_panic_family(&root, &strict_units)?;
    let advisory_findings = scan_panic_family(&root, &advisory_units)?;

    println!("Lint policy report");
    println!("  Workspace MSRV: {workspace_msrv}");
    println!("  Policy MSRV: {policy_msrv}");
    println!("  Active lint entries: {}", active_lints.len());
    println!("  Planned lint entries: {}", planned_lints.len());
    println!(
        "  Required inherited packages: {}",
        required_packages.join(", ")
    );
    println!("  Staged packages: {}", staged_packages.join(", "));
    println!("  Debt receipts: {debt_count}");
    println!();
    println!("No-panic policy");
    println!("  Allowlist entries: {}", no_panic_entries.len());
    println!(
        "  Strict findings (required-inheriting crates): {}",
        strict_findings.len()
    );
    println!(
        "  Advisory findings (staged crates):           {}",
        advisory_findings.len()
    );
    println!();
    println!("File policy");
    println!(
        "  Non-Rust allowlist entries: {}",
        file_policy_entries.len()
    );
    Ok(())
}

fn ensure_policy_flags(policy_text: &str) -> Result<()> {
    for (key, expected) in [
        ("panic_free_tests", "true"),
        ("allow_test_carveouts", "false"),
        ("suppression_style", "expect-with-reason"),
        ("blanket_categories", "false"),
    ] {
        let actual = value_after(policy_text, "[policy]", key)
            .ok_or_else(|| anyhow!("policy/clippy-lints.toml is missing policy.{key}"))?;
        let actual = actual.trim().trim_matches('"');
        if actual != expected {
            return Err(anyhow!(
                "policy/clippy-lints.toml policy.{key} must be {expected}, found {actual}"
            ));
        }
    }
    Ok(())
}

fn ensure_no_test_carveouts(clippy_toml: &Path) -> Result<()> {
    let text = fs::read_to_string(clippy_toml)?;
    let banned = [
        "allow-unwrap-in-tests",
        "allow-expect-in-tests",
        "allow-panic-in-tests",
        "allow-indexing-slicing-in-tests",
        "allow-dbg-in-tests",
    ];
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        for key in banned {
            if line.starts_with(key) {
                return Err(anyhow!(
                    "clippy.toml must not configure test carveout `{key}`; tests inherit the workspace panic-free policy"
                ));
            }
        }
    }
    Ok(())
}

fn ensure_workspace_lint_inheritance(root: &Path, policy_text: &str) -> Result<()> {
    let metadata = MetadataCommand::new().current_dir(root).exec()?;
    let workspace_members: HashSet<_> = metadata.workspace_members.iter().cloned().collect();
    let mut inherited_count = 0usize;
    let mut inherited_packages = BTreeSet::new();

    for package in metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(&pkg.id))
    {
        let manifest_path = PathBuf::from(package.manifest_path.as_str());
        let text = fs::read_to_string(&manifest_path)?;
        if !text.lines().any(|line| line.trim() == "[lints]") {
            continue;
        }

        let inherits = value_after(&text, "[lints]", "workspace")
            .map(|value| value.trim() == "true")
            .unwrap_or(false);
        if !inherits {
            return Err(anyhow!(
                "{} has a [lints] table but does not inherit workspace lints with `workspace = true`",
                manifest_path.display()
            ));
        }
        inherited_count = inherited_count
            .checked_add(1)
            .ok_or_else(|| anyhow!("lint inheritance count overflow"))?;
        inherited_packages.insert(package.name.to_string());
    }

    let required_packages =
        string_array_after(policy_text, "[rollout]", "required_inheriting_packages").ok_or_else(
            || anyhow!("policy/clippy-lints.toml is missing rollout.required_inheriting_packages"),
        )?;
    if required_packages.is_empty() {
        return Err(anyhow!(
            "policy/clippy-lints.toml rollout.required_inheriting_packages must not be empty"
        ));
    }

    for required in &required_packages {
        if !inherited_packages.contains(required) {
            return Err(anyhow!(
                "{required} must inherit workspace lints with [lints] workspace = true"
            ));
        }
    }

    let staged_packages =
        string_array_after(policy_text, "[rollout]", "staged_inheriting_packages").ok_or_else(
            || anyhow!("policy/clippy-lints.toml is missing rollout.staged_inheriting_packages"),
        )?;

    for staged in &staged_packages {
        if required_packages.iter().any(|required| required == staged) {
            return Err(anyhow!(
                "{staged} cannot be both required and staged for workspace lint inheritance"
            ));
        }
    }

    println!(
        "lint policy: {inherited_count} workspace package(s) inherit the baseline; {} package(s) are staged",
        staged_packages.len()
    );
    Ok(())
}

fn ensure_debt_receipts(policy_debt: &Path) -> Result<()> {
    let text = fs::read_to_string(policy_debt)?;
    for (index, entry) in table_array_entries(&text, "[[debt]]").iter().enumerate() {
        let entry_number = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("debt entry index overflow"))?;
        for key in ["lint", "path", "owner", "reason", "expires"] {
            if top_level_quoted_value(entry, key).is_none() {
                return Err(anyhow!(
                    "policy/clippy-debt.toml debt entry {entry_number} is missing required field `{key}`"
                ));
            }
        }

        let expires = top_level_quoted_value(entry, "expires").ok_or_else(|| {
            anyhow!("policy/clippy-debt.toml debt entry {entry_number} is missing expires")
        })?;
        if expires.as_str() < "2026-05-06" {
            return Err(anyhow!(
                "policy/clippy-debt.toml debt entry {entry_number} expired on {expires}"
            ));
        }
    }
    Ok(())
}

fn parse_workspace_lints(cargo_text: &str) -> BTreeMap<String, String> {
    let mut lints = BTreeMap::new();
    for (section, prefix) in [
        ("[workspace.lints.rust]", ""),
        ("[workspace.lints.clippy]", "clippy::"),
    ] {
        for line in section_body(cargo_text, section) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((name, level)) = trimmed.split_once('=') {
                lints.insert(
                    format!("{prefix}{}", name.trim()),
                    level.trim().trim_matches('"').to_string(),
                );
            }
        }
    }
    lints
}

fn parse_policy_lints(policy_text: &str, status: &str) -> Result<BTreeMap<String, String>> {
    let mut lints = BTreeMap::new();
    for entry in table_array_entries(policy_text, "[[lint]]") {
        let entry_status = top_level_quoted_value(&entry, "status")
            .ok_or_else(|| anyhow!("policy lint entry is missing status"))?;
        if entry_status != status {
            continue;
        }
        let name = top_level_quoted_value(&entry, "name")
            .ok_or_else(|| anyhow!("policy lint entry is missing name"))?;
        let level = top_level_quoted_value(&entry, "level")
            .ok_or_else(|| anyhow!("policy lint entry {name} is missing level"))?;
        if status == "planned" && top_level_quoted_value(&entry, "activate_when_msrv").is_none() {
            return Err(anyhow!("planned lint {name} is missing activate_when_msrv"));
        }
        for required in ["class", "reason"] {
            if top_level_quoted_value(&entry, required).is_none() {
                return Err(anyhow!("policy lint entry {name} is missing {required}"));
            }
        }
        lints.insert(name, level);
    }
    Ok(lints)
}

fn quoted_value_after(text: &str, section: &str, key: &str) -> Option<String> {
    value_after(text, section, key).map(|value| value.trim().trim_matches('"').to_string())
}

fn value_after(text: &str, section: &str, key: &str) -> Option<String> {
    section_body(text, section).find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return None;
        }
        let (name, value) = trimmed.split_once('=')?;
        (name.trim() == key).then(|| value.trim().to_string())
    })
}

fn top_level_quoted_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return None;
        }
        let (name, value) = trimmed.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_string())
    })
}

fn string_array_after(text: &str, section: &str, key: &str) -> Option<Vec<String>> {
    let mut value = value_after(text, section, key)?;
    if value.trim_start().starts_with('[') && !value.trim_end().ends_with(']') {
        let mut found_key = false;
        for line in section_body(text, section) {
            let trimmed = line.trim();
            if found_key {
                value.push(' ');
                value.push_str(trimmed);
                if trimmed.ends_with(']') {
                    break;
                }
                continue;
            }
            if trimmed.starts_with('#') {
                continue;
            }
            let (name, _) = trimmed.split_once('=')?;
            if name.trim() == key {
                found_key = true;
            }
        }
    }
    let trimmed = value.trim();
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.trim_matches('"').to_string())
            .collect(),
    )
}

fn section_body<'a>(text: &'a str, section: &str) -> impl Iterator<Item = &'a str> {
    let mut in_section = false;
    text.lines().filter(move |line| {
        let trimmed = line.trim();
        if trimmed == section {
            in_section = true;
            return false;
        }
        if in_section && trimmed.starts_with('[') {
            in_section = false;
        }
        in_section
    })
}

fn table_array_entries(text: &str, marker: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = Vec::new();
    let mut in_entry = false;

    for line in text.lines() {
        if line.trim() == marker {
            if in_entry {
                entries.push(current.join("\n"));
                current.clear();
            }
            in_entry = true;
            continue;
        }
        if in_entry {
            current.push(line.to_string());
        }
    }

    if in_entry {
        entries.push(current.join("\n"));
    }

    entries
}

fn escape_toml_basic_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
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
            if let Some(crate_name) = parts.get(1) {
                changed_crates.insert((*crate_name).to_string());
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

// ---------------------------------------------------------------------------
// Semantic no-panic checker
// ---------------------------------------------------------------------------
//
// Scans Rust source under crates that inherit the workspace clippy panic
// baseline (plus xtask) and matches findings against
// `policy/no-panic-allowlist.toml`. Identity is `path + family + selector`
// (kind + callee, plus optional container). `last_seen.{line,column}` is
// advisory.
//
// The scanner is intentionally lexical and skips:
//   * line comments (`//`, `///`, `//!`)
//   * block comments (`/* ... */`, with simple state)
//   * string and byte-string literals
//   * raw string literals (`r"..."`, `r#"..."#`)
//   * findings inside files that have a file-level `#![expect(...)]`
//     covering the relevant clippy lint — those are governed by Clippy and
//     `policy/clippy-debt.toml`.
//
// Doc comments and `cfg(test)` attributes are not given special treatment.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PanicFamily {
    Unwrap,
    Expect,
    GetUnwrap,
    PanicMacro,
    Todo,
    Unimplemented,
    Unreachable,
}

impl PanicFamily {
    fn as_str(self) -> &'static str {
        match self {
            PanicFamily::Unwrap => "unwrap",
            PanicFamily::Expect => "expect",
            PanicFamily::GetUnwrap => "get_unwrap",
            PanicFamily::PanicMacro => "panic_macro",
            PanicFamily::Todo => "todo",
            PanicFamily::Unimplemented => "unimplemented",
            PanicFamily::Unreachable => "unreachable",
        }
    }

    fn callee(self) -> &'static str {
        match self {
            PanicFamily::Unwrap => "unwrap",
            PanicFamily::Expect => "expect",
            PanicFamily::GetUnwrap => "get_unwrap",
            PanicFamily::PanicMacro => "panic",
            PanicFamily::Todo => "todo",
            PanicFamily::Unimplemented => "unimplemented",
            PanicFamily::Unreachable => "unreachable",
        }
    }

    fn selector_kind(self) -> &'static str {
        match self {
            PanicFamily::Unwrap | PanicFamily::Expect | PanicFamily::GetUnwrap => "method_call",
            PanicFamily::PanicMacro
            | PanicFamily::Todo
            | PanicFamily::Unimplemented
            | PanicFamily::Unreachable => "macro",
        }
    }

    /// Clippy lint name (without the `clippy::` prefix) that, when wholesale
    /// suppressed at file or module level, masks findings of this family.
    fn clippy_lint(self) -> &'static str {
        match self {
            PanicFamily::Unwrap => "unwrap_used",
            PanicFamily::Expect => "expect_used",
            PanicFamily::GetUnwrap => "get_unwrap",
            PanicFamily::PanicMacro => "panic",
            PanicFamily::Todo => "todo",
            PanicFamily::Unimplemented => "unimplemented",
            PanicFamily::Unreachable => "unreachable",
        }
    }

    fn all() -> &'static [PanicFamily] {
        &[
            PanicFamily::Unwrap,
            PanicFamily::Expect,
            PanicFamily::GetUnwrap,
            PanicFamily::PanicMacro,
            PanicFamily::Todo,
            PanicFamily::Unimplemented,
            PanicFamily::Unreachable,
        ]
    }

    fn from_str(s: &str) -> Option<PanicFamily> {
        match s {
            "unwrap" => Some(PanicFamily::Unwrap),
            "expect" => Some(PanicFamily::Expect),
            "get_unwrap" => Some(PanicFamily::GetUnwrap),
            "panic_macro" => Some(PanicFamily::PanicMacro),
            "todo" => Some(PanicFamily::Todo),
            "unimplemented" => Some(PanicFamily::Unimplemented),
            "unreachable" => Some(PanicFamily::Unreachable),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct PanicFinding {
    path: String,
    family: PanicFamily,
    container: Option<String>,
    line: usize,
    column: usize,
}

#[derive(Clone, Debug)]
#[expect(
    dead_code,
    reason = "owner/classification/explanation are validated at parse time and surfaced in error messages; future report subcommands will read them"
)]
struct NoPanicAllowEntry {
    id: String,
    path: String,
    family: String,
    classification: String,
    owner: String,
    explanation: String,
    expires: String,
    selector_kind: String,
    selector_callee: String,
    selector_container: Option<String>,
}

const NO_PANIC_CLASSIFICATIONS: &[&str] = &[
    "production",
    "test_helper",
    "generated",
    "fixture",
    "external_api",
];

const NO_PANIC_SELECTOR_KINDS: &[&str] = &["method_call", "macro", "indexing"];

fn check_no_panic_family(include_staged_in_strict: bool) -> Result<()> {
    println!("🔎 Checking no-panic-family policy...");
    let root = env::current_dir()?;
    let policy_text = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let allowlist_text = fs::read_to_string(root.join("policy/no-panic-allowlist.toml"))?;

    let entries = parse_no_panic_allowlist(&allowlist_text)?;
    enforce_no_panic_expirations(&entries)?;

    let required = string_array_after(&policy_text, "[rollout]", "required_inheriting_packages")
        .ok_or_else(|| {
            anyhow!("policy/clippy-lints.toml is missing rollout.required_inheriting_packages")
        })?;
    let staged = string_array_after(&policy_text, "[rollout]", "staged_inheriting_packages")
        .ok_or_else(|| {
            anyhow!("policy/clippy-lints.toml is missing rollout.staged_inheriting_packages")
        })?;

    let metadata = MetadataCommand::new().current_dir(&root).exec()?;
    let strict_files = collect_rust_files_for(&root, &metadata, &required)?;
    let advisory_files = if include_staged_in_strict {
        Vec::new()
    } else {
        collect_rust_files_for(&root, &metadata, &staged)?
    };

    let strict_findings = scan_panic_family(&root, &strict_files)?;
    let advisory_findings = scan_panic_family(&root, &advisory_files)?;

    let unmatched = match_findings_against_allowlist(&strict_findings, &entries);
    if !unmatched.is_empty() {
        for f in unmatched.iter().take(20) {
            eprintln!(
                "no-panic: {}:{}:{}: unallowlisted {} ({} {})",
                f.path,
                f.line,
                f.column,
                f.family.as_str(),
                f.family.selector_kind(),
                f.family.callee(),
            );
        }
        if unmatched.len() > 20 {
            eprintln!(
                "no-panic: ... and {} more findings",
                unmatched.len().saturating_sub(20)
            );
        }
        return Err(anyhow!(
            "{} unallowlisted panic-family finding(s) in inheriting crates; \
             receipt them via policy/no-panic-allowlist.toml or remove the call",
            unmatched.len()
        ));
    }

    let stale = stale_no_panic_entries(&entries, &strict_findings);
    if !stale.is_empty() {
        for entry in &stale {
            eprintln!(
                "no-panic: stale entry id={} path={} family={} (no matching finding)",
                entry.id, entry.path, entry.family
            );
        }
        return Err(anyhow!(
            "{} stale no-panic-allowlist entr(ies); remove or update them",
            stale.len()
        ));
    }

    println!(
        "✅ no-panic policy: {} required-inheriting source file(s) scanned, \
         {} allowlist entr(ies), {} advisory finding(s) in staged crates",
        total_files(&strict_files),
        entries.len(),
        advisory_findings.len(),
    );
    Ok(())
}

fn no_panic_propose(include_staged: bool) -> Result<()> {
    let root = env::current_dir()?;
    let policy_text = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let metadata = MetadataCommand::new().current_dir(&root).exec()?;

    let mut packages =
        string_array_after(&policy_text, "[rollout]", "required_inheriting_packages")
            .ok_or_else(|| anyhow!("policy is missing required_inheriting_packages"))?;
    if include_staged {
        let staged = string_array_after(&policy_text, "[rollout]", "staged_inheriting_packages")
            .ok_or_else(|| anyhow!("policy is missing staged_inheriting_packages"))?;
        packages.extend(staged);
    }

    let files = collect_rust_files_for(&root, &metadata, &packages)?;
    let findings = scan_panic_family(&root, &files)?;

    let report_dir = root.join("target/policy");
    fs::create_dir_all(&report_dir)?;
    let report_path = report_dir.join("no-panic-proposed-allowlist.toml");

    let mut out = String::new();
    out.push_str("schema_version = \"0.3\"\n\n");
    out.push_str("# Proposed allowlist entries generated by `xtask no-panic propose`.\n");
    out.push_str("# Review each entry, set owner/classification/explanation/expires, then\n");
    out.push_str("# copy into policy/no-panic-allowlist.toml.\n\n");

    for (index, finding) in findings.iter().enumerate() {
        let proposal_index = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("proposal index overflow"))?;
        out.push_str("[[allow]]\n");
        out.push_str(&format!("id = \"panic-proposal-{proposal_index:04}\"\n"));
        out.push_str(&format!(
            "path = \"{}\"\n",
            escape_toml_basic_string(&finding.path),
        ));
        out.push_str(&format!("family = \"{}\"\n", finding.family.as_str()));
        out.push_str("classification = \"FILL_ME_IN\"\n");
        out.push_str("owner = \"FILL_ME_IN\"\n");
        out.push_str("explanation = \"FILL_ME_IN\"\n");
        out.push_str("expires = \"FILL_ME_IN\"\n");
        out.push_str("\n[allow.selector]\n");
        out.push_str(&format!("kind = \"{}\"\n", finding.family.selector_kind(),));
        out.push_str(&format!("callee = \"{}\"\n", finding.family.callee(),));
        if let Some(container) = &finding.container {
            out.push_str(&format!(
                "container = \"{}\"\n",
                escape_toml_basic_string(container),
            ));
        }
        out.push_str("\n[allow.last_seen]\n");
        out.push_str(&format!("line = {}\n", finding.line));
        out.push_str(&format!("column = {}\n\n", finding.column));
    }

    fs::write(&report_path, out)?;
    println!(
        "wrote {} proposed entr(ies) to {}",
        findings.len(),
        report_path.display()
    );
    Ok(())
}

/// A scanning unit: one crate, with the set of files to scan and the union of
/// crate-root clippy suppressions that apply to all of those files.
struct ScanUnit {
    files: Vec<PathBuf>,
    root_suppressions: HashSet<String>,
}

fn collect_rust_files_for(
    root: &Path,
    metadata: &Metadata,
    package_names: &[String],
) -> Result<Vec<ScanUnit>> {
    let mut units = Vec::new();
    let workspace_members: HashSet<_> = metadata.workspace_members.iter().cloned().collect();
    let by_name: HashMap<&str, &Package> = metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(&pkg.id))
        .map(|pkg| (pkg.name.as_str(), pkg))
        .collect();

    for name in package_names {
        let Some(pkg) = by_name.get(name.as_str()) else {
            continue;
        };
        let manifest = PathBuf::from(pkg.manifest_path.as_str());
        let crate_root = manifest
            .parent()
            .ok_or_else(|| anyhow!("crate {name} manifest has no parent directory"))?
            .to_path_buf();

        let mut files = Vec::new();
        for sub in ["src", "tests", "benches", "examples"] {
            let dir = crate_root.join(sub);
            if dir.exists() {
                walk_rust_sources(&dir, &mut files)?;
            }
        }
        let build_script = crate_root.join("build.rs");
        if build_script.exists() {
            files.push(build_script);
        }
        files.sort();
        files.dedup();

        // Union of crate-root suppressions: any `#![...]` in src/main.rs,
        // src/lib.rs, or src/bin/*.rs cascades to every module of the crate.
        let mut root_suppressions = HashSet::new();
        for candidate in [
            crate_root.join("src/main.rs"),
            crate_root.join("src/lib.rs"),
        ] {
            if candidate.exists()
                && let Ok(text) = fs::read_to_string(&candidate)
            {
                root_suppressions.extend(file_level_clippy_suppressions(&text));
            }
        }
        let bin_dir = crate_root.join("src/bin");
        if bin_dir.exists() {
            for entry in fs::read_dir(&bin_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs")
                    && let Ok(text) = fs::read_to_string(&path)
                {
                    root_suppressions.extend(file_level_clippy_suppressions(&text));
                }
            }
        }

        units.push(ScanUnit {
            files,
            root_suppressions,
        });
    }

    let _ = root;
    Ok(units)
}

fn walk_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_rust_sources(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn scan_panic_family(root: &Path, units: &[ScanUnit]) -> Result<Vec<PanicFinding>> {
    let mut findings = Vec::new();
    for unit in units {
        for path in &unit.files {
            let text = fs::read_to_string(path)?;
            let mut suppressed = file_level_clippy_suppressions(&text);
            suppressed.extend(unit.root_suppressions.iter().cloned());
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            scan_panic_in_file(&rel, &text, &suppressed, &mut findings);
        }
    }
    findings.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
    });
    Ok(findings)
}

fn total_files(units: &[ScanUnit]) -> usize {
    units.iter().map(|u| u.files.len()).sum()
}

/// Returns the set of clippy lint names that are suppressed somewhere in
/// this file via any of:
/// `#[allow(clippy::X)]`, `#![allow(clippy::X)]`,
/// `#[expect(clippy::X, ...)]`, `#![expect(clippy::X, ...)]`,
/// `#[cfg_attr(<cond>, expect(clippy::X, ...))]`, etc.
///
/// This is intentionally a file-wide approximation: if any item in the file
/// suppresses a panic-family lint with a Clippy attribute, that file's
/// findings for that family are considered governed by Rail A (Clippy +
/// `policy/clippy-debt.toml`). The semantic checker stays in lockstep with
/// Clippy and does not double-flag receipts that already exist there.
#[expect(
    clippy::indexing_slicing,
    reason = "Manual byte-level walk over a stripped buffer where indices are explicitly bounds-checked against bytes.len()."
)]
fn file_level_clippy_suppressions(text: &str) -> HashSet<String> {
    let mut suppressed = HashSet::new();
    let stripped = strip_strings_and_comments(text);
    let bytes = stripped.as_bytes();

    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'#' {
            i = i.saturating_add(1);
            continue;
        }
        let after_hash = i.saturating_add(1);
        let after_bang = if bytes.get(after_hash) == Some(&b'!') {
            after_hash.saturating_add(1)
        } else {
            after_hash
        };
        if bytes.get(after_bang) != Some(&b'[') {
            i = i.saturating_add(1);
            continue;
        }
        // Found an attribute: walk forward to its matching ']'.
        let mut j = after_bang;
        let mut depth = 0i32;
        while j < bytes.len() {
            match bytes[j] {
                b'[' => depth = depth.saturating_add(1),
                b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        j = j.saturating_add(1);
                        break;
                    }
                }
                _ => {}
            }
            j = j.saturating_add(1);
        }
        let span = stripped.get(after_bang..j).unwrap_or("");
        // Only treat this as a suppression scope if the attribute is one of
        // allow/expect/cfg_attr; otherwise (e.g. `#[derive(...)]`) skip.
        let span_trim = span.trim_start_matches('[');
        let is_relevant = span_trim.trim_start().starts_with("allow")
            || span_trim.trim_start().starts_with("expect")
            || span_trim.trim_start().starts_with("cfg_attr");
        if is_relevant {
            for token in span.split([',', ' ', '(', ')', '[', ']', '!', '#']) {
                let t = token.trim();
                if let Some(rest) = t.strip_prefix("clippy::") {
                    let name = rest.trim_end_matches(',').trim();
                    if !name.is_empty() {
                        suppressed.insert(name.to_string());
                    }
                }
            }
        }
        i = j.max(i.saturating_add(1));
    }
    suppressed
}

#[expect(
    clippy::string_slice,
    reason = "`line` is a single ASCII line from the stripped buffer; indices come from substring matches and saturating_add."
)]
fn scan_panic_in_file(
    rel_path: &str,
    text: &str,
    suppressed: &HashSet<String>,
    out: &mut Vec<PanicFinding>,
) {
    let stripped = strip_strings_and_comments(text);
    let mut current_fn: Option<(String, usize)> = None;

    for (line_idx, line) in stripped.lines().enumerate() {
        let line_no = line_idx.saturating_add(1);

        if let Some(name) = extract_fn_name(line) {
            current_fn = Some((name, line_no));
        }

        for family in PanicFamily::all() {
            if suppressed.contains(family.clippy_lint()) {
                continue;
            }
            let mut start = 0usize;
            while let Some(rel) = find_family_match(&line[start..], *family) {
                let abs = start.saturating_add(rel);
                let column = abs.saturating_add(1);
                out.push(PanicFinding {
                    path: rel_path.to_string(),
                    family: *family,
                    container: current_fn.as_ref().map(|(n, _)| n.clone()),
                    line: line_no,
                    column,
                });
                start = abs.saturating_add(1);
            }
        }
    }
}

fn extract_fn_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("pub fn ")
        .or_else(|| trimmed.strip_prefix("fn "))
        .or_else(|| trimmed.strip_prefix("async fn "))
        .or_else(|| trimmed.strip_prefix("pub async fn "))
        .or_else(|| trimmed.strip_prefix("const fn "))
        .or_else(|| trimmed.strip_prefix("pub const fn "))
        .or_else(|| trimmed.strip_prefix("unsafe fn "))
        .or_else(|| trimmed.strip_prefix("pub unsafe fn "))?;
    let mut name = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            name.push(ch);
        } else {
            break;
        }
    }
    if name.is_empty() { None } else { Some(name) }
}

fn find_family_match(haystack: &str, family: PanicFamily) -> Option<usize> {
    match family {
        PanicFamily::Unwrap => find_method_call(haystack, "unwrap"),
        PanicFamily::Expect => find_method_call(haystack, "expect"),
        PanicFamily::GetUnwrap => find_method_call(haystack, "get_unwrap"),
        PanicFamily::PanicMacro => find_macro_invocation(haystack, "panic"),
        PanicFamily::Todo => find_macro_invocation(haystack, "todo"),
        PanicFamily::Unimplemented => find_macro_invocation(haystack, "unimplemented"),
        PanicFamily::Unreachable => find_macro_invocation(haystack, "unreachable"),
    }
}

#[expect(
    clippy::string_slice,
    reason = "`haystack[search_from..]` slices on a byte offset returned by `str::find`, which is guaranteed to be a UTF-8 boundary."
)]
fn find_method_call(haystack: &str, name: &str) -> Option<usize> {
    let needle_dot = format!(".{name}(");
    let needle_turbofish = format!(".{name}::");
    let mut search_from = 0usize;
    loop {
        let dot_hit = haystack[search_from..]
            .find(&needle_dot)
            .and_then(|i| i.checked_add(search_from).map(|x| (x, needle_dot.len())));
        let turbofish_hit = haystack[search_from..]
            .find(&needle_turbofish)
            .and_then(|i| {
                i.checked_add(search_from)
                    .map(|x| (x, needle_turbofish.len()))
            });
        let next = match (dot_hit, turbofish_hit) {
            (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let (idx, span) = next?;
        if !is_method_boundary(haystack, idx) {
            search_from = idx.saturating_add(span);
            continue;
        }
        return Some(idx);
    }
}

fn is_method_boundary(haystack: &str, idx: usize) -> bool {
    if idx == 0 {
        return false;
    }
    let prev = haystack
        .as_bytes()
        .get(idx.saturating_sub(1))
        .copied()
        .unwrap_or(0);
    !matches!(prev, b'.')
}

#[expect(
    clippy::string_slice,
    reason = "`haystack[search_from..]` slices on a byte offset returned by `str::find`, which is guaranteed to be a UTF-8 boundary."
)]
fn find_macro_invocation(haystack: &str, name: &str) -> Option<usize> {
    let needle = format!("{name}!");
    let mut search_from = 0usize;
    loop {
        let next = haystack[search_from..].find(&needle)?;
        let idx = search_from.checked_add(next)?;
        let after = idx.checked_add(needle.len())?;
        if !is_macro_invocation_boundary(haystack, idx, after) {
            search_from = idx.saturating_add(needle.len());
            continue;
        }
        return Some(idx);
    }
}

fn is_macro_invocation_boundary(haystack: &str, start: usize, after: usize) -> bool {
    if start > 0 {
        let prev = haystack
            .as_bytes()
            .get(start.saturating_sub(1))
            .copied()
            .unwrap_or(0);
        if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b':' {
            return false;
        }
    }
    matches!(haystack.as_bytes().get(after), Some(b'(' | b'[' | b'{'))
}

/// Replace the contents of strings and comments with spaces so byte offsets
/// and line numbers remain stable while substring matches do not fire on
/// content inside literals or comments.
#[expect(
    clippy::indexing_slicing,
    reason = "Manual byte-level lexer over a freshly-allocated buffer with explicit `i < bytes.len()` bounds checks."
)]
fn strip_strings_and_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = vec![b' '; bytes.len()];

    let mut i = 0usize;
    let mut in_block_comment = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'\n' {
            out[i] = b'\n';
            i = i.saturating_add(1);
            continue;
        }

        if in_block_comment > 0 {
            if bytes[i] == b'/' && i > 0 && bytes[i.saturating_sub(1)] == b'*' {
                in_block_comment = in_block_comment.saturating_sub(1);
            } else if i.saturating_add(1) < bytes.len()
                && bytes[i] == b'/'
                && bytes[i.saturating_add(1)] == b'*'
            {
                in_block_comment = in_block_comment.saturating_add(1);
                i = i.saturating_add(2);
                continue;
            }
            i = i.saturating_add(1);
            continue;
        }

        // Line comment
        if i.saturating_add(1) < bytes.len()
            && bytes[i] == b'/'
            && bytes[i.saturating_add(1)] == b'/'
        {
            while i < bytes.len() && bytes[i] != b'\n' {
                i = i.saturating_add(1);
            }
            continue;
        }

        // Block comment start
        if i.saturating_add(1) < bytes.len()
            && bytes[i] == b'/'
            && bytes[i.saturating_add(1)] == b'*'
        {
            in_block_comment = 1;
            i = i.saturating_add(2);
            continue;
        }

        // Raw string literal: r"..." or r#"..."# (with N hashes)
        if bytes[i] == b'r' || (bytes[i] == b'b' && bytes.get(i.saturating_add(1)) == Some(&b'r')) {
            let mut probe = i;
            if bytes[probe] == b'b' {
                probe = probe.saturating_add(1);
            }
            if bytes.get(probe) == Some(&b'r') {
                let mut hashes = 0usize;
                let mut p = probe.saturating_add(1);
                while bytes.get(p) == Some(&b'#') {
                    hashes = hashes.saturating_add(1);
                    p = p.saturating_add(1);
                }
                if bytes.get(p) == Some(&b'"') {
                    // start of raw string; find closing "###...
                    let start = i;
                    let mut q = p.saturating_add(1);
                    let close_marker_len = hashes.saturating_add(1);
                    while q < bytes.len() {
                        if bytes[q] == b'"' {
                            let mut ok = true;
                            for h in 1..=hashes {
                                if bytes.get(q.saturating_add(h)) != Some(&b'#') {
                                    ok = false;
                                    break;
                                }
                            }
                            if ok {
                                q = q.saturating_add(close_marker_len);
                                break;
                            }
                        }
                        if bytes[q] == b'\n' {
                            out[q] = b'\n';
                        }
                        q = q.saturating_add(1);
                    }
                    let _ = start;
                    i = q;
                    continue;
                }
            }
        }

        // Char or string literal
        if bytes[i] == b'"' || (bytes[i] == b'b' && bytes.get(i.saturating_add(1)) == Some(&b'"')) {
            let mut q = if bytes[i] == b'b' {
                i.saturating_add(2)
            } else {
                i.saturating_add(1)
            };
            while q < bytes.len() {
                match bytes[q] {
                    b'\\' => {
                        q = q.saturating_add(2);
                        continue;
                    }
                    b'"' => {
                        q = q.saturating_add(1);
                        break;
                    }
                    b'\n' => {
                        out[q] = b'\n';
                    }
                    _ => {}
                }
                q = q.saturating_add(1);
            }
            i = q;
            continue;
        }

        if bytes[i] == b'\'' {
            // char literal: rough — find next unescaped single quote on same line
            let mut q = i.saturating_add(1);
            let mut closed = false;
            while q < bytes.len() && bytes[q] != b'\n' {
                if bytes[q] == b'\\' {
                    q = q.saturating_add(2);
                    continue;
                }
                if bytes[q] == b'\'' {
                    closed = true;
                    q = q.saturating_add(1);
                    break;
                }
                q = q.saturating_add(1);
            }
            if closed {
                i = q;
                continue;
            }
            // Lifetime — copy through
            out[i] = bytes[i];
            i = i.saturating_add(1);
            continue;
        }

        out[i] = bytes[i];
        i = i.saturating_add(1);
    }

    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

fn parse_no_panic_allowlist(text: &str) -> Result<Vec<NoPanicAllowEntry>> {
    let entries = table_array_entries(text, "[[allow]]");
    let mut parsed = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let entry_no = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("allowlist index overflow"))?;

        let id = top_level_quoted_value(entry, "id").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {entry_no} is missing `id`")
        })?;
        let path = top_level_quoted_value(entry, "path").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `path`")
        })?;
        let family = top_level_quoted_value(entry, "family").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `family`")
        })?;
        if PanicFamily::from_str(&family).is_none() {
            return Err(anyhow!(
                "policy/no-panic-allowlist.toml entry {id} has unknown family `{family}`"
            ));
        }
        let classification = top_level_quoted_value(entry, "classification").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `classification`")
        })?;
        if !NO_PANIC_CLASSIFICATIONS.contains(&classification.as_str()) {
            return Err(anyhow!(
                "policy/no-panic-allowlist.toml entry {id} has unknown classification `{classification}`"
            ));
        }
        let owner = top_level_quoted_value(entry, "owner").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `owner`")
        })?;
        let explanation = top_level_quoted_value(entry, "explanation").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `explanation`")
        })?;
        let expires = top_level_quoted_value(entry, "expires").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `expires`")
        })?;

        let selector_kind =
            sub_table_value(entry, "[allow.selector]", "kind").ok_or_else(|| {
                anyhow!(
                    "policy/no-panic-allowlist.toml entry {id} is missing `[allow.selector] kind`"
                )
            })?;
        if !NO_PANIC_SELECTOR_KINDS.contains(&selector_kind.as_str()) {
            return Err(anyhow!(
                "policy/no-panic-allowlist.toml entry {id} has unknown selector kind `{selector_kind}`"
            ));
        }
        let selector_callee =
            sub_table_value(entry, "[allow.selector]", "callee").ok_or_else(|| {
                anyhow!(
                    "policy/no-panic-allowlist.toml entry {id} is missing `[allow.selector] callee`"
                )
            })?;
        let selector_container = sub_table_value(entry, "[allow.selector]", "container");

        parsed.push(NoPanicAllowEntry {
            id,
            path,
            family,
            classification,
            owner,
            explanation,
            expires,
            selector_kind,
            selector_callee,
            selector_container,
        });
    }
    Ok(parsed)
}

fn enforce_no_panic_expirations(entries: &[NoPanicAllowEntry]) -> Result<()> {
    let today = "2026-05-06"; // CLAUDE.md fixes `today` for the policy ratchet.
    for entry in entries {
        if entry.expires.as_str() < today {
            return Err(anyhow!(
                "policy/no-panic-allowlist.toml entry {} expired on {}",
                entry.id,
                entry.expires
            ));
        }
    }
    let _ = NO_PANIC_CLASSIFICATIONS; // silence unused if all entries are empty
    Ok(())
}

fn match_findings_against_allowlist(
    findings: &[PanicFinding],
    entries: &[NoPanicAllowEntry],
) -> Vec<PanicFinding> {
    findings
        .iter()
        .filter(|f| !entries.iter().any(|e| no_panic_entry_matches_finding(e, f)))
        .cloned()
        .collect()
}

fn no_panic_entry_matches_finding(entry: &NoPanicAllowEntry, finding: &PanicFinding) -> bool {
    if entry.path != finding.path {
        return false;
    }
    if entry.family != finding.family.as_str() {
        return false;
    }
    if entry.selector_kind != finding.family.selector_kind() {
        return false;
    }
    if entry.selector_callee != finding.family.callee() {
        return false;
    }
    match (&entry.selector_container, &finding.container) {
        (Some(want), Some(got)) => want == got,
        (Some(_), None) => false,
        (None, _) => true,
    }
}

fn stale_no_panic_entries<'a>(
    entries: &'a [NoPanicAllowEntry],
    findings: &[PanicFinding],
) -> Vec<&'a NoPanicAllowEntry> {
    entries
        .iter()
        .filter(|entry| {
            !findings
                .iter()
                .any(|f| no_panic_entry_matches_finding(entry, f))
        })
        .collect()
}

fn sub_table_value(entry_text: &str, marker: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in entry_text.lines() {
        let trimmed = line.trim();
        if trimmed == marker {
            in_section = true;
            continue;
        }
        if in_section {
            if trimmed.starts_with('[') {
                return None;
            }
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((name, value)) = trimmed.split_once('=')
                && name.trim() == key
            {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// File-policy checker
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[expect(
    dead_code,
    reason = "kind/owner/surface/reason are validated at parse time; future policy-report subcommand will summarize them"
)]
struct FilePolicyEntry {
    pattern: String,
    is_glob: bool,
    kind: String,
    owner: String,
    surface: String,
    classification: String,
    reason: String,
    covered_by: Vec<String>,
    expires: Option<String>,
    retired: bool,
}

const FILE_POLICY_CLASSIFICATIONS: &[&str] = &[
    "production",
    "test",
    "tooling",
    "config",
    "generated",
    "docs",
];

fn check_file_policy() -> Result<()> {
    println!("🔎 Checking non-Rust file policy...");
    let root = env::current_dir()?;
    let allowlist_text = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;
    let entries = parse_file_policy_allowlist(&allowlist_text)?;
    enforce_file_policy_expirations(&entries)?;

    let tracked = git_output(&["ls-files"])?;
    let files: Vec<String> = tracked
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut unmatched: Vec<String> = Vec::new();
    let mut entry_hits: Vec<usize> = vec![0; entries.len()];

    for file in &files {
        if file_is_auto_allowed(file) {
            continue;
        }
        let mut matched = false;
        for (idx, entry) in entries.iter().enumerate() {
            if file_matches_entry(file, entry) {
                if let Some(slot) = entry_hits.get_mut(idx) {
                    *slot = slot.checked_add(1).unwrap_or(*slot);
                }
                matched = true;
            }
        }
        if !matched {
            unmatched.push(file.clone());
        }
    }

    if !unmatched.is_empty() {
        for f in unmatched.iter().take(40) {
            eprintln!("file-policy: unallowlisted non-Rust file: {f}");
        }
        if unmatched.len() > 40 {
            eprintln!(
                "file-policy: ... and {} more file(s)",
                unmatched.len().saturating_sub(40)
            );
        }
        return Err(anyhow!(
            "{} non-Rust file(s) lack a policy/non-rust-allowlist.toml entry",
            unmatched.len()
        ));
    }

    let mut stale: Vec<&FilePolicyEntry> = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        if entry.retired {
            continue;
        }
        if entry_hits.get(idx).copied().unwrap_or(0) == 0 {
            stale.push(entry);
        }
    }
    if !stale.is_empty() {
        for entry in &stale {
            eprintln!(
                "file-policy: stale entry pattern={} (no tracked file matched)",
                entry.pattern
            );
        }
        return Err(anyhow!(
            "{} stale non-Rust allowlist entr(ies); remove or set retired = true",
            stale.len()
        ));
    }

    println!(
        "✅ file policy: {} tracked file(s) checked, {} allowlist entr(ies)",
        files.len(),
        entries.len()
    );
    Ok(())
}

fn file_is_auto_allowed(path: &str) -> bool {
    if path.ends_with(".rs") {
        return true;
    }
    if path == "Cargo.toml" || path == "Cargo.lock" {
        return true;
    }
    if path.ends_with("/Cargo.toml") {
        return true;
    }
    if path == ".gitignore" || path == ".gitattributes" {
        return true;
    }
    if path == "LICENSE" || path == "NOTICE" {
        return true;
    }
    if path.ends_with(".md") {
        return true;
    }
    if path == ".envrc" {
        return true;
    }
    false
}

fn parse_file_policy_allowlist(text: &str) -> Result<Vec<FilePolicyEntry>> {
    let entries = table_array_entries(text, "[[allow]]");
    let mut parsed = Vec::with_capacity(entries.len());
    for (index, raw) in entries.iter().enumerate() {
        let entry_no = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("file-policy entry index overflow"))?;
        let glob = top_level_quoted_value(raw, "glob");
        let path = top_level_quoted_value(raw, "path");
        let (pattern, is_glob) = match (glob, path) {
            (Some(g), None) => (g, true),
            (None, Some(p)) => (p, false),
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "policy/non-rust-allowlist.toml entry {entry_no} cannot set both `glob` and `path`"
                ));
            }
            (None, None) => {
                return Err(anyhow!(
                    "policy/non-rust-allowlist.toml entry {entry_no} must set either `glob` or `path`"
                ));
            }
        };

        let kind = top_level_quoted_value(raw, "kind").ok_or_else(|| {
            anyhow!("policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) is missing `kind`")
        })?;
        let owner = top_level_quoted_value(raw, "owner").ok_or_else(|| {
            anyhow!(
                "policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) is missing `owner`"
            )
        })?;
        let surface = top_level_quoted_value(raw, "surface").ok_or_else(|| {
            anyhow!(
                "policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) is missing `surface`"
            )
        })?;
        let classification = top_level_quoted_value(raw, "classification").ok_or_else(|| {
            anyhow!(
                "policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) is missing `classification`"
            )
        })?;
        if !FILE_POLICY_CLASSIFICATIONS.contains(&classification.as_str()) {
            return Err(anyhow!(
                "policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) has unknown classification `{classification}`"
            ));
        }
        let reason = top_level_quoted_value(raw, "reason").ok_or_else(|| {
            anyhow!(
                "policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) is missing `reason`"
            )
        })?;
        let covered_by = string_array_after_root(raw, "covered_by").unwrap_or_default();
        if matches!(classification.as_str(), "production" | "test" | "tooling")
            && covered_by.is_empty()
        {
            return Err(anyhow!(
                "policy/non-rust-allowlist.toml entry {entry_no} ({pattern}) classification `{classification}` requires `covered_by`"
            ));
        }
        let expires = top_level_quoted_value(raw, "expires");
        let retired = top_level_quoted_value(raw, "retired")
            .map(|v| v == "true")
            .unwrap_or(false);

        parsed.push(FilePolicyEntry {
            pattern,
            is_glob,
            kind,
            owner,
            surface,
            classification,
            reason,
            covered_by,
            expires,
            retired,
        });
    }
    Ok(parsed)
}

fn enforce_file_policy_expirations(entries: &[FilePolicyEntry]) -> Result<()> {
    let today = "2026-05-06";
    for entry in entries {
        if let Some(expires) = &entry.expires
            && expires.as_str() < today
        {
            return Err(anyhow!(
                "policy/non-rust-allowlist.toml entry `{}` expired on {}",
                entry.pattern,
                expires
            ));
        }
    }
    Ok(())
}

fn file_matches_entry(file: &str, entry: &FilePolicyEntry) -> bool {
    if entry.is_glob {
        glob_match(&entry.pattern, file)
    } else {
        entry.pattern == file
    }
}

/// Minimal git-style glob matcher supporting `*`, `?`, and `**`.
/// `*` does not cross `/`. `**` does. `?` matches a single non-`/` char.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), 0, text.as_bytes(), 0)
}

#[expect(
    clippy::indexing_slicing,
    reason = "Recursive glob matcher with explicit `pi < pat.len()` and `ti < text.len()` bounds checks before each indexing operation."
)]
fn glob_match_inner(pat: &[u8], pi: usize, text: &[u8], ti: usize) -> bool {
    let mut pi = pi;
    let mut ti = ti;
    loop {
        if pi >= pat.len() {
            return ti >= text.len();
        }
        let ch = pat[pi];
        if ch == b'*' {
            // Detect "**"
            if pat.get(pi.saturating_add(1)) == Some(&b'*') {
                let after = pi.saturating_add(2);
                // Allow optional `/` after `**/`
                let next_pi = if pat.get(after) == Some(&b'/') {
                    after.saturating_add(1)
                } else {
                    after
                };
                if next_pi >= pat.len() {
                    return true;
                }
                let mut k = ti;
                loop {
                    if glob_match_inner(pat, next_pi, text, k) {
                        return true;
                    }
                    if k >= text.len() {
                        return false;
                    }
                    k = k.saturating_add(1);
                }
            }
            // Single '*' — match any chars except '/'
            let next_pi = pi.saturating_add(1);
            if next_pi >= pat.len() {
                return !text[ti..].contains(&b'/');
            }
            let mut k = ti;
            loop {
                if glob_match_inner(pat, next_pi, text, k) {
                    return true;
                }
                if k >= text.len() || text[k] == b'/' {
                    return false;
                }
                k = k.saturating_add(1);
            }
        }
        if ch == b'?' {
            if ti >= text.len() || text[ti] == b'/' {
                return false;
            }
            pi = pi.saturating_add(1);
            ti = ti.saturating_add(1);
            continue;
        }
        if ti >= text.len() || text[ti] != ch {
            return false;
        }
        pi = pi.saturating_add(1);
        ti = ti.saturating_add(1);
    }
}

fn string_array_after_root(text: &str, key: &str) -> Option<Vec<String>> {
    let mut buffer = String::new();
    let mut found_key = false;
    let mut depth = 0i32;
    for line in text.lines() {
        let trimmed = line.trim();
        if !found_key {
            if trimmed.starts_with('#') {
                continue;
            }
            if let Some((name, value)) = trimmed.split_once('=')
                && name.trim() == key
            {
                let v = value.trim();
                buffer.push_str(v);
                found_key = true;
                if v.starts_with('[') {
                    depth = depth.saturating_add(1);
                }
                if v.ends_with(']') {
                    depth = depth.saturating_sub(1);
                }
                if depth <= 0 {
                    break;
                }
            }
        } else {
            buffer.push(' ');
            buffer.push_str(trimmed);
            for c in trimmed.chars() {
                if c == '[' {
                    depth = depth.saturating_add(1);
                }
                if c == ']' {
                    depth = depth.saturating_sub(1);
                }
            }
            if depth <= 0 {
                break;
            }
        }
    }
    if !found_key {
        return None;
    }
    let trimmed = buffer.trim();
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_matches('"').to_string())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_order_uses_workspace_dependency_order() -> Result<()> {
        let ordered = publish_order(None)?;

        ensure_contains(&ordered, "hl7v2-core")?;
        ensure_contains(&ordered, "hl7v2")?;
        ensure_contains(&ordered, "hl7v2-template-values")?;
        if ordered.iter().any(|crate_name| crate_name == "xtask") {
            return Err(anyhow!("xtask should not be publishable"));
        }

        assert_dependency_precedes(&ordered, "hl7v2-datatype", "hl7v2-core")?;
        assert_dependency_precedes(&ordered, "hl7v2-core", "hl7v2")?;
        assert_dependency_precedes(&ordered, "hl7v2-template-values", "hl7v2-template")?;
        Ok(())
    }

    #[test]
    fn publish_order_can_resume_from_a_named_crate() -> Result<()> {
        let ordered = publish_order(None)?;
        let resumed = publish_order(Some("hl7v2-core"))?;
        let start = ordered
            .iter()
            .position(|crate_name| crate_name == "hl7v2-core")
            .ok_or_else(|| anyhow!("hl7v2-core should be publishable"))?;
        let expected = ordered
            .get(start..)
            .ok_or_else(|| anyhow!("resume start is outside publish order"))?
            .to_vec();

        if resumed != expected {
            return Err(anyhow!(
                "resumed publish order did not match expected suffix"
            ));
        }
        Ok(())
    }

    #[test]
    fn workspace_patch_dependencies_include_publishable_dev_dependencies() -> Result<()> {
        let metadata = MetadataCommand::new().exec()?;
        let packages = publishable_workspace_packages(&metadata);
        let dependencies = internal_workspace_dependency_closure("hl7v2-ack", &packages)?;

        for dependency in ["hl7v2-core", "hl7v2-writer"] {
            if !dependencies.contains(dependency) {
                return Err(anyhow!(
                    "workspace patch dependency closure should include {dependency}"
                ));
            }
        }
        if dependencies.contains("hl7v2-test-utils") {
            return Err(anyhow!(
                "workspace patch dependency closure should exclude non-publishable test utils"
            ));
        }
        Ok(())
    }

    fn assert_dependency_precedes(
        ordered: &[String],
        dependency: &str,
        dependent: &str,
    ) -> Result<()> {
        let dependency_index = ordered
            .iter()
            .position(|crate_name| crate_name == dependency)
            .ok_or_else(|| anyhow!("{dependency} should be present in publish order"))?;
        let dependent_index = ordered
            .iter()
            .position(|crate_name| crate_name == dependent)
            .ok_or_else(|| anyhow!("{dependent} should be present in publish order"))?;

        if dependency_index >= dependent_index {
            return Err(anyhow!("{dependency} should appear before {dependent}"));
        }
        Ok(())
    }

    fn ensure_contains(ordered: &[String], crate_name: &str) -> Result<()> {
        if ordered.iter().any(|name| name == crate_name) {
            return Ok(());
        }
        Err(anyhow!("{crate_name} should be present in publish order"))
    }
}
