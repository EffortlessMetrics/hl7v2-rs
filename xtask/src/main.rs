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
    /// Verify workspace lint policy, debt ledger, and Clippy configuration
    CheckLintPolicy,
    /// Check panic-family calls against the semantic allowlist
    CheckNoPanicFamily,
    /// Check non-Rust files against the TOML file-policy allowlist
    CheckFilePolicy,
    /// Print a summary of policy debt and allowlist coverage
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
        Commands::Scaffold { name, description } => scaffold(&name, description)?,
        Commands::Docs { no_open } => docs(no_open)?,
        Commands::HookPreCommit => hook_pre_commit()?,
        Commands::HookPrePush => hook_pre_push()?,
        Commands::CheckLintPolicy => check_lint_policy()?,
        Commands::CheckNoPanicFamily => check_no_panic_family()?,
        Commands::CheckFilePolicy => check_file_policy()?,
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

fn check_lint_policy() -> Result<()> {
    println!("🔎 Checking lint policy...");

    let root = env::current_dir()?;
    let cargo_toml = fs::read_to_string(root.join("Cargo.toml"))?;
    let policy = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let debt = fs::read_to_string(root.join("policy/clippy-debt.toml"))?;

    let cargo_msrv = required_value(&cargo_toml, "rust-version")?;
    let policy_msrv = required_value(&policy, "msrv")?;
    ensure(
        cargo_msrv == policy_msrv,
        format!(
            "workspace.package.rust-version ({cargo_msrv}) must match policy/clippy-lints.toml msrv ({policy_msrv})"
        ),
    )?;

    ensure(
        cargo_toml.contains("[workspace.lints.rust]")
            && cargo_toml.contains("[workspace.lints.clippy]"),
        "root Cargo.toml must define [workspace.lints.rust] and [workspace.lints.clippy]",
    )?;
    for lint in required_active_lints() {
        let cargo_name = lint
            .strip_prefix("clippy::")
            .or_else(|| lint.strip_prefix("rust::"))
            .unwrap_or(lint);
        ensure(
            cargo_toml.contains(&format!("{cargo_name} =")) || cargo_toml.contains(lint),
            format!("root Cargo.toml is missing active lint {lint}"),
        )?;
        ensure(
            policy.contains(&format!("name = \"{lint}\"")),
            format!("policy/clippy-lints.toml is missing active lint {lint}"),
        )?;
    }

    ensure(
        policy.contains("panic_free_tests = true")
            && policy.contains("allow_test_carveouts = false")
            && policy.contains("suppression_style = \"expect-with-reason\""),
        "policy/clippy-lints.toml must encode panic-free tests, no test carveouts, and expect-with-reason suppressions",
    )?;

    for planned in planned_lints() {
        ensure(
            policy.contains(&format!("name = \"{}\"", planned.name))
                && policy.contains(&format!("activate_when_msrv = \"{}\"", planned.msrv)),
            format!(
                "policy/clippy-lints.toml must track planned lint {} for MSRV {}",
                planned.name, planned.msrv
            ),
        )?;
        let cargo_name = planned
            .name
            .strip_prefix("clippy::")
            .unwrap_or(planned.name);
        ensure(
            !version_less_than(&cargo_msrv, planned.msrv)
                || !cargo_toml.contains(&format!("{cargo_name} =")),
            format!(
                "planned lint {} must not be active before MSRV {}",
                planned.name, planned.msrv
            ),
        )?;
    }

    for carveout in [
        "allow-unwrap-in-tests",
        "allow-expect-in-tests",
        "allow-panic-in-tests",
        "allow-indexing-slicing-in-tests",
        "allow-dbg-in-tests",
    ] {
        let clippy_toml = fs::read_to_string(root.join("clippy.toml")).unwrap_or_default();
        ensure(
            !clippy_toml.contains(carveout),
            format!("clippy.toml must not contain test carveout {carveout}"),
        )?;
    }

    let metadata = workspace_metadata()?;
    let root_manifest = root.join("Cargo.toml");
    for package in metadata.workspace_packages() {
        let manifest = package.manifest_path.as_std_path();
        if manifest == root_manifest {
            continue;
        }
        let manifest_text = fs::read_to_string(manifest)?;
        ensure(
            manifest_text.contains("[lints]") && manifest_text.contains("workspace = true"),
            format!("{} must inherit workspace lints", manifest.display()),
        )?;
    }

    validate_debt_entries(&debt)?;

    println!("✅ Lint policy checks passed");
    Ok(())
}

fn check_no_panic_family() -> Result<()> {
    println!("🔎 Checking panic-family calls...");

    let root = env::current_dir()?;
    let allowlist = fs::read_to_string(root.join("policy/no-panic-allowlist.toml"))?;
    validate_allow_entries(
        &allowlist,
        &["path", "family", "classification", "owner", "explanation"],
    )?;

    let mut violations = Vec::new();
    let rust_files = collect_files(&root, |path| path.extension() == Some(OsStr::new("rs")))?;
    for path in rust_files {
        let rel = relative_path(&root, &path);
        let text = fs::read_to_string(&path)?;
        for (line_idx, line) in text.lines().enumerate() {
            let Some(family) = panic_family(line) else {
                continue;
            };
            if !allowlist_contains_panic(&allowlist, &rel, family) {
                violations.push(format!(
                    "{}:{} contains unallowlisted {family}",
                    rel,
                    line_idx + 1
                ));
            }
        }
    }

    if !violations.is_empty() {
        return Err(anyhow!(
            "panic-family policy found {} unallowlisted call(s):\n{}",
            violations.len(),
            violations
                .into_iter()
                .take(50)
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    println!("✅ Panic-family checks passed");
    Ok(())
}

fn check_file_policy() -> Result<()> {
    println!("🔎 Checking non-Rust file policy...");

    let root = env::current_dir()?;
    let allowlist_text = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;
    validate_allow_entries(
        &allowlist_text,
        &[
            "kind",
            "owner",
            "reason",
            "surface",
            "classification",
            "covered_by",
        ],
    )?;
    let allows = parse_file_allows(&allowlist_text);
    let mut uncovered = Vec::new();
    for path in collect_files(&root, |_| true)? {
        let rel = relative_path(&root, &path);
        if is_ignored_policy_path(&rel) || is_rust_owned_path(&path, &rel) {
            continue;
        }
        if !allows.iter().any(|allow| allow.matches(&rel)) {
            uncovered.push(rel);
        }
    }

    if !uncovered.is_empty() {
        return Err(anyhow!(
            "non-Rust file policy found {} uncovered file(s):\n{}",
            uncovered.len(),
            uncovered
                .into_iter()
                .take(50)
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    println!("✅ File policy checks passed");
    Ok(())
}

fn policy_report() -> Result<()> {
    let root = env::current_dir()?;
    let debt = fs::read_to_string(root.join("policy/clippy-debt.toml"))?;
    let panic_allowlist = fs::read_to_string(root.join("policy/no-panic-allowlist.toml"))?;
    let non_rust_allowlist = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;

    println!("📋 Policy report");
    println!("clippy debt entries: {}", count_tables(&debt, "[[debt]]"));
    println!(
        "panic allowlist entries: {}",
        count_tables(&panic_allowlist, "[[allow]]")
    );
    println!(
        "non-Rust allowlist entries: {}",
        count_tables(&non_rust_allowlist, "[[allow]]")
    );
    println!("planned lint flips: {}", planned_lints().len());
    Ok(())
}

fn workspace_metadata() -> Result<Metadata> {
    Ok(MetadataCommand::new().exec()?)
}

fn required_active_lints() -> &'static [&'static str] {
    &[
        "rust::unsafe_code",
        "rust::unused_must_use",
        "clippy::unwrap_used",
        "clippy::expect_used",
        "clippy::panic",
        "clippy::indexing_slicing",
        "clippy::string_slice",
        "clippy::map_err_ignore",
        "clippy::allow_attributes_without_reason",
    ]
}

struct PlannedLint {
    name: &'static str,
    msrv: &'static str,
}

fn planned_lints() -> &'static [PlannedLint] {
    &[
        PlannedLint {
            name: "clippy::same_length_and_capacity",
            msrv: "1.94",
        },
        PlannedLint {
            name: "clippy::manual_ilog2",
            msrv: "1.94",
        },
        PlannedLint {
            name: "clippy::decimal_bitwise_operands",
            msrv: "1.94",
        },
        PlannedLint {
            name: "clippy::needless_type_cast",
            msrv: "1.94",
        },
        PlannedLint {
            name: "clippy::disallowed_fields",
            msrv: "1.95",
        },
        PlannedLint {
            name: "clippy::manual_checked_ops",
            msrv: "1.95",
        },
        PlannedLint {
            name: "clippy::manual_take",
            msrv: "1.95",
        },
        PlannedLint {
            name: "clippy::manual_pop_if",
            msrv: "1.95",
        },
        PlannedLint {
            name: "clippy::duration_suboptimal_units",
            msrv: "1.95",
        },
        PlannedLint {
            name: "clippy::unnecessary_trailing_comma",
            msrv: "1.95",
        },
    ]
}

fn validate_debt_entries(text: &str) -> Result<()> {
    for (idx, entry) in split_tables(text, "[[debt]]").iter().enumerate() {
        for field in ["lint", "path", "owner", "reason", "expires"] {
            ensure(
                entry.contains(&format!("{field} =")),
                format!(
                    "policy/clippy-debt.toml debt entry {} is missing {field}",
                    idx + 1
                ),
            )?;
        }
        let expires = required_value(entry, "expires")?;
        ensure(
            expires.as_str() >= "2026-05-06",
            format!(
                "policy/clippy-debt.toml debt entry {} expired on {expires}",
                idx + 1
            ),
        )?;
    }
    Ok(())
}

fn validate_allow_entries(text: &str, required_fields: &[&str]) -> Result<()> {
    for (idx, entry) in split_tables(text, "[[allow]]").iter().enumerate() {
        for field in required_fields {
            ensure(
                entry.contains(&format!("{field} =")),
                format!("allowlist entry {} is missing {field}", idx + 1),
            )?;
        }
        if entry.contains("expires =") {
            let expires = required_value(entry, "expires")?;
            ensure(
                expires.as_str() >= "2026-05-06",
                format!("allowlist entry {} expired on {expires}", idx + 1),
            )?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct FileAllow {
    path: Option<String>,
    glob: Option<String>,
}

impl FileAllow {
    fn matches(&self, rel: &str) -> bool {
        if self.path.as_deref() == Some(rel) {
            return true;
        }
        self.glob
            .as_deref()
            .is_some_and(|glob| glob_matches(glob, rel))
    }
}

fn parse_file_allows(text: &str) -> Vec<FileAllow> {
    split_tables(text, "[[allow]]")
        .into_iter()
        .map(|entry| FileAllow {
            path: optional_value(&entry, "path"),
            glob: optional_value(&entry, "glob"),
        })
        .collect()
}

fn split_tables(text: &str, marker: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let mut current = Vec::new();
    let mut in_table = false;

    for line in text.lines() {
        if line.trim() == marker {
            if in_table && !current.is_empty() {
                tables.push(current.join("\n"));
                current.clear();
            }
            in_table = true;
            continue;
        }
        if in_table {
            current.push(line.to_string());
        }
    }

    if in_table && !current.is_empty() {
        tables.push(current.join("\n"));
    }

    tables
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn count_tables(text: &str, marker: &str) -> usize {
    text.lines().filter(|line| line.trim() == marker).count()
}

fn required_value(text: &str, key: &str) -> Result<String> {
    optional_value(text, key).ok_or_else(|| anyhow!("missing required key {key}"))
}

fn optional_value(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} =");
    text.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix(&prefix)
            .map(str::trim)
            .map(|value| value.trim_matches('"').to_string())
    })
}

fn version_less_than(left: &str, right: &str) -> bool {
    let parse = |version: &str| -> Vec<u32> {
        version
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect()
    };
    parse(left) < parse(right)
}

fn collect_files(root: &Path, predicate: impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_inner(root, root, &predicate, &mut files)?;
    Ok(files)
}

fn collect_files_inner(
    root: &Path,
    current: &Path,
    predicate: &impl Fn(&Path) -> bool,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let rel = relative_path(root, &path);
        if path.is_dir() {
            if matches!(rel.as_str(), ".git" | "target") {
                continue;
            }
            collect_files_inner(root, &path, predicate, files)?;
        } else if predicate(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn panic_family(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    [
        ("unwrap", ".unwrap("),
        ("expect", ".expect("),
        ("panic", "panic!("),
        ("todo", "todo!("),
        ("unimplemented", "unimplemented!("),
        ("unreachable", "unreachable!("),
    ]
    .into_iter()
    .find_map(|(family, needle)| line.contains(needle).then_some(family))
}

fn allowlist_contains_panic(allowlist: &str, path: &str, family: &str) -> bool {
    split_tables(allowlist, "[[allow]]").iter().any(|entry| {
        optional_value(entry, "path").as_deref() == Some(path)
            && optional_value(entry, "family").as_deref() == Some(family)
    })
}

fn is_ignored_policy_path(rel: &str) -> bool {
    rel.starts_with(".git/")
        || rel.starts_with("target/")
        || rel == "Cargo.lock"
        || rel.starts_with("docs/")
        || rel.ends_with(".md")
        || rel == "LICENSE"
        || rel == "CLA.md"
        || rel == "CODE_OF_CONDUCT.md"
        || rel == "CONTRIBUTING.md"
        || rel == "CHANGELOG.md"
        || rel == "README.md"
        || rel == "NIX_USAGE.md"
        || rel == "ROADMAP.md"
        || rel == "SESSION_SUMMARY.md"
        || rel == "TESTING.md"
        || rel == "DEPLOYMENT.md"
        || rel == "GEMINI.md"
        || rel == "CLAUDE.md"
        || rel.ends_with("/CLAUDE.md")
        || rel.ends_with("/README.md")
        || rel.ends_with(".txt")
        || rel == "policy/clippy-lints.toml"
        || rel == "policy/clippy-debt.toml"
        || rel == "policy/no-panic-allowlist.toml"
        || rel == "policy/non-rust-allowlist.toml"
        || rel == "clippy.toml"
}

fn is_rust_owned_path(path: &Path, rel: &str) -> bool {
    path.extension() == Some(OsStr::new("rs"))
        || rel.ends_with("Cargo.toml")
        || rel.starts_with("crates/") && rel.ends_with("/Cargo.toml")
}

fn glob_matches(glob: &str, rel: &str) -> bool {
    if let Some(prefix) = glob.strip_suffix("/**") {
        return rel == prefix || rel.starts_with(&format!("{prefix}/"));
    }
    if !glob.contains('*') {
        return glob == rel;
    }

    let mut remaining = rel;
    let mut parts = glob.split('*').peekable();
    let Some(first) = parts.next() else {
        return true;
    };
    if !remaining.starts_with(first) {
        return false;
    }
    remaining = &remaining[first.len()..];

    while let Some(part) = parts.next() {
        if part.is_empty() {
            continue;
        }
        if parts.peek().is_none() {
            return remaining.ends_with(part)
                || remaining.find(part).is_some_and(|idx| {
                    let after = idx + part.len();
                    after == remaining.len()
                });
        }
        let Some(idx) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[idx + part.len()..];
    }

    true
}

fn ensure(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(anyhow!(message.into()))
    }
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
