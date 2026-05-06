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
