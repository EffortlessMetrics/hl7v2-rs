//! Workspace task runner for repository automation and release checks.

use anyhow::{Context, Result, anyhow};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};
use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
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
    /// Verify the non-Rust file allowlist against tracked and untracked non-ignored files
    CheckFilePolicy,
    /// Verify explicit local Markdown links point at checked-in repository targets
    CheckDocLinks,
    /// Verify Python TestPyPI/PyPI release workflow safety controls
    CheckPythonPublishPolicy,
    /// Verify CI lane whitelist: coverage, required fields, expensive-default exceptions
    CheckCiLaneWhitelist,
    /// Regenerate committed public Shields badge endpoint JSON
    Badges {
        /// Check generated endpoint files for drift without updating committed badges/ files
        #[arg(long)]
        check: bool,
    },
    /// Produce PR-scoped RIPR exposure evidence under target/ripr/pr
    RiprPr {
        /// Verify the existing PR evidence output contract instead of producing evidence
        #[arg(long)]
        check: bool,
    },
    /// Produce RIPR review guidance under target/ripr/review
    RiprReviewComments {
        /// Verify the existing review guidance output contract instead of producing guidance
        #[arg(long)]
        check: bool,
    },
    /// Validate checked-in evidence fixtures against their JSON schemas
    EvidenceSchemaCheck,
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
        Commands::CheckDocLinks => check_doc_links()?,
        Commands::CheckPythonPublishPolicy => check_python_publish_policy()?,
        Commands::CheckCiLaneWhitelist => check_ci_lane_whitelist()?,
        Commands::Badges { check } => badges(check)?,
        Commands::RiprPr { check } => ripr_pr(check)?,
        Commands::RiprReviewComments { check } => ripr_review_comments(check)?,
        Commands::EvidenceSchemaCheck => evidence_schema_check()?,
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Badge endpoints and RIPR PR evidence
// ---------------------------------------------------------------------------

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";
const RIPR_PR_DIR: &str = "target/ripr/pr";
const RIPR_REVIEW_DIR: &str = "target/ripr/review";

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    label: String,
    message: String,
    color: String,
}

fn workspace_root_path() -> Result<PathBuf> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    Ok(PathBuf::from(metadata.workspace_root.as_str()))
}

fn badges(check: bool) -> Result<()> {
    let workspace_root = workspace_root_path()?;
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    fs::create_dir_all(&target_dir)?;

    let ripr_plus = ripr_plus_badge(&workspace_root)?;
    validate_shields_badge(&ripr_plus, Some("ripr+"))?;
    write_json_pretty(&target_dir.join("ripr-plus.json"), &ripr_plus)?;

    if check {
        let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
        compare_files(
            &committed_dir.join("ripr-plus.json"),
            &target_dir.join("ripr-plus.json"),
        )?;
        println!("badges: committed endpoints are current");
        return Ok(());
    }

    let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
    fs::create_dir_all(&committed_dir)?;
    fs::copy(
        target_dir.join("ripr-plus.json"),
        committed_dir.join("ripr-plus.json"),
    )?;
    println!("badges: refreshed public endpoint JSON under badges/");
    Ok(())
}

fn ripr_plus_badge(workspace_root: &Path) -> Result<ShieldsEndpointBadge> {
    let ripr_bin = env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());
    let output = Command::new(&ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--format")
        .arg("repo-badge-plus-shields")
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("failed to run {ripr_bin}; install ripr or set RIPR_BIN"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "{ripr_bin} repo-badge-plus-shields failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("{ripr_bin} emitted invalid Shields endpoint JSON"))
}

fn validate_shields_badge(
    badge: &ShieldsEndpointBadge,
    expected_label: Option<&str>,
) -> Result<()> {
    if badge.schema_version != 1 {
        return Err(anyhow!(
            "badge `{}` has unsupported schemaVersion",
            badge.label
        ));
    }
    if let Some(expected_label) = expected_label
        && badge.label != expected_label
    {
        return Err(anyhow!(
            "badge label drifted: got `{}`, expected `{expected_label}`",
            badge.label
        ));
    }
    if badge.message.trim().is_empty() {
        return Err(anyhow!("badge `{}` has empty message", badge.label));
    }
    if badge.color.trim().is_empty() {
        return Err(anyhow!("badge `{}` has empty color", badge.label));
    }
    Ok(())
}

fn write_json_pretty<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{text}\n"))?;
    Ok(())
}

fn compare_files(committed: &Path, generated: &Path) -> Result<()> {
    let committed_text = fs::read_to_string(committed)
        .with_context(|| format!("missing committed endpoint {}", committed.display()))?;
    let generated_text = fs::read_to_string(generated)?;
    if committed_text != generated_text {
        return Err(anyhow!(
            "{} is out of date; run cargo xtask badges",
            committed.display()
        ));
    }
    Ok(())
}

fn ripr_pr(check: bool) -> Result<()> {
    let workspace_root = workspace_root_path()?;
    let out_dir = workspace_root.join(RIPR_PR_DIR);
    let json_path = out_dir.join("repo-exposure.json");
    let md_path = out_dir.join("repo-exposure.md");

    if check {
        validate_json_file(&json_path)?;
        validate_non_empty_file(&md_path)?;
        println!("ripr-pr: output contract is current");
        return Ok(());
    }

    fs::create_dir_all(&out_dir)?;
    run_ripr_stdout(
        &workspace_root,
        &["check", "--root", ".", "--format", "repo-exposure-json"],
        &json_path,
    )?;
    run_ripr_stdout(
        &workspace_root,
        &["check", "--root", ".", "--format", "repo-exposure-md"],
        &md_path,
    )?;
    validate_json_file(&json_path)?;
    validate_non_empty_file(&md_path)?;
    println!("ripr-pr: wrote PR-scoped evidence under {RIPR_PR_DIR}");
    Ok(())
}

fn ripr_review_comments(check: bool) -> Result<()> {
    let workspace_root = workspace_root_path()?;
    let out_dir = workspace_root.join(RIPR_REVIEW_DIR);
    let json_path = out_dir.join("comments.json");
    let md_path = out_dir.join("comments.md");

    if check {
        validate_json_file(&json_path)?;
        validate_non_empty_file(&md_path)?;
        println!("ripr-review-comments: output contract is current");
        return Ok(());
    }

    fs::create_dir_all(&out_dir)?;
    let ripr_bin = env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());
    let status = Command::new(&ripr_bin)
        .arg("review-comments")
        .arg("--root")
        .arg(".")
        .arg("--base")
        .arg(ripr_base_ref())
        .arg("--head")
        .arg(ripr_head_ref())
        .arg("--out")
        .arg(&json_path)
        .current_dir(&workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run {ripr_bin}; install ripr or set RIPR_BIN"))?;
    if !status.success() {
        return Err(anyhow!(
            "{ripr_bin} review-comments failed with exit code {:?}",
            status.code()
        ));
    }
    validate_json_file(&json_path)?;
    validate_non_empty_file(&md_path)?;
    println!("ripr-review-comments: wrote review guidance under {RIPR_REVIEW_DIR}");
    Ok(())
}

fn run_ripr_stdout(workspace_root: &Path, args: &[&str], output_path: &Path) -> Result<()> {
    let ripr_bin = env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());
    let output = Command::new(&ripr_bin)
        .args(args)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("failed to run {ripr_bin}; install ripr or set RIPR_BIN"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "{ripr_bin} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    fs::write(output_path, output.stdout)?;
    Ok(())
}

fn ripr_base_ref() -> String {
    env::var("RIPR_BASE").unwrap_or_else(|_| "origin/main".to_string())
}

fn ripr_head_ref() -> String {
    env::var("RIPR_HEAD").unwrap_or_else(|_| "HEAD".to_string())
}

fn validate_json_file(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required JSON artifact {}", path.display()))?;
    if text.trim().is_empty() {
        return Err(anyhow!("{} is empty", path.display()));
    }
    let _: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("{} contains invalid JSON", path.display()))?;
    Ok(())
}

fn validate_non_empty_file(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required artifact {}", path.display()))?;
    if text.trim().is_empty() {
        return Err(anyhow!("{} is empty", path.display()));
    }
    Ok(())
}

fn gate(check: bool, changed_only: bool, only: Option<String>) -> Result<()> {
    println!("🚀 Running gate checks...");

    let (changed_only, crates, changed_docs_require_link_check) = if changed_only {
        match get_changed_scope()? {
            ChangedScope::Crates {
                crates,
                has_markdown,
            } => (true, crates, has_markdown),
            ChangedScope::Workspace => {
                println!("Non-crate files changed. Running full workspace gate.");
                (false, vec![], false)
            }
            ChangedScope::None => {
                println!("No files changed. Skipping checks.");
                return Ok(());
            }
        }
    } else {
        (false, vec![], false)
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
        println!("Checking Markdown local links...");
        check_doc_links()?;
        println!("Checking Python publish policy...");
        check_python_publish_policy()?;
    } else if changed_docs_require_link_check {
        println!("Checking Markdown local links for crate-scoped doc changes...");
        check_doc_links()?;
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

// ---------------------------------------------------------------------------
// Evidence schema checker
// ---------------------------------------------------------------------------

struct EvidenceSchemaTarget {
    schema: PathBuf,
    data: PathBuf,
}

const SUPPLEMENTAL_EVIDENCE_FIXTURES: &[(&str, &str)] = &[(
    "safe-analysis-redaction-output-receipt-v2.json",
    "safe-analysis-redaction-output-v1.schema.json",
)];

fn evidence_schema_check() -> Result<()> {
    println!("🔎 Checking evidence JSON schemas...");

    let root = env::current_dir()?;
    let targets = evidence_schema_targets(&root)?;
    if targets.is_empty() {
        return Err(anyhow!("no evidence schema targets found"));
    }

    for target in &targets {
        println!(
            "Validating {} against {}",
            display_repo_path(&target.data, &root),
            display_repo_path(&target.schema, &root)
        );
        run_ajv_validate(&target.schema, &target.data)?;
    }

    println!(
        "✅ evidence schemas: {} fixture(s) validated",
        targets.len()
    );
    Ok(())
}

fn evidence_schema_targets(root: &Path) -> Result<Vec<EvidenceSchemaTarget>> {
    let schema_dir = root.join("schemas/evidence");
    let fixture_dir = root.join("fixtures/evidence");

    let schema_names = evidence_json_schema_names(&schema_dir)?;
    let fixture_names = evidence_json_file_names(&fixture_dir)?;
    let mut covered_fixtures = BTreeSet::new();
    let mut targets = Vec::new();

    for schema_name in &schema_names {
        let fixture_name = evidence_fixture_name_for_schema(schema_name, &fixture_names)?;
        covered_fixtures.insert(fixture_name.clone());
        targets.push(EvidenceSchemaTarget {
            schema: schema_dir.join(schema_name),
            data: fixture_dir.join(fixture_name),
        });
    }

    for (fixture_name, schema_name) in SUPPLEMENTAL_EVIDENCE_FIXTURES {
        if fixture_names.contains(*fixture_name) {
            if !schema_names.contains(*schema_name) {
                return Err(anyhow!(
                    "supplemental evidence fixture {fixture_name} maps to missing schema {schema_name}"
                ));
            }
            covered_fixtures.insert((*fixture_name).to_string());
            targets.push(EvidenceSchemaTarget {
                schema: schema_dir.join(schema_name),
                data: fixture_dir.join(fixture_name),
            });
        }
    }

    let uncovered: Vec<&String> = fixture_names.difference(&covered_fixtures).collect();
    if !uncovered.is_empty() {
        for fixture_name in &uncovered {
            eprintln!("evidence-schema-check: fixture has no schema mapping: {fixture_name}");
        }
        return Err(anyhow!(
            "{} evidence fixture(s) have no schema mapping",
            uncovered.len()
        ));
    }

    targets.sort_by(|a, b| a.schema.cmp(&b.schema).then_with(|| a.data.cmp(&b.data)));
    Ok(targets)
}

fn evidence_json_schema_names(schema_dir: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(schema_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.ends_with(".schema.json") && name.contains("-v") {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}

fn evidence_json_file_names(fixture_dir: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(fixture_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.ends_with(".json") {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}

fn evidence_fixture_name_for_schema(
    schema_file_name: &str,
    fixture_names: &BTreeSet<String>,
) -> Result<String> {
    let schema_name = schema_file_name
        .strip_suffix(".schema.json")
        .ok_or_else(|| {
            anyhow!("evidence schema file must end with .schema.json: {schema_file_name}")
        })?;
    let direct = format!("{schema_name}.json");
    if fixture_names.contains(&direct) {
        return Ok(direct);
    }

    let mut tried = vec![direct];
    if let Some(legacy_name) = schema_name.strip_suffix("-v1") {
        let legacy = format!("{legacy_name}.json");
        if fixture_names.contains(&legacy) {
            return Ok(legacy);
        }
        tried.push(legacy);
    }

    Err(anyhow!(
        "no evidence fixture found for schema {schema_file_name}; tried {}",
        tried.join(", ")
    ))
}

fn run_ajv_validate(schema: &Path, data: &Path) -> Result<()> {
    let mut command = if command_exists("ajv") {
        let mut command = Command::new(command_program("ajv"));
        command.arg("validate");
        command
    } else if command_exists("npx") {
        let mut command = Command::new(command_program("npx"));
        command.args([
            "-y",
            "-p",
            "ajv-cli",
            "-p",
            "ajv-formats",
            "ajv",
            "validate",
        ]);
        command
    } else {
        return Err(anyhow!(
            "evidence schema check requires ajv-cli and ajv-formats; install with `npm install -g ajv-cli ajv-formats` or make npx available"
        ));
    };

    let status = command
        .args(["-c", "ajv-formats", "-s"])
        .arg(schema)
        .arg("-d")
        .arg(data)
        .arg("--spec=draft7")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!(
            "AJV validation failed for {} against {} with exit code: {:?}",
            data.display(),
            schema.display(),
            status.code()
        ));
    }

    Ok(())
}

fn command_program(cmd: &str) -> String {
    if cfg!(windows) {
        format!("{cmd}.cmd")
    } else {
        cmd.to_string()
    }
}

fn display_repo_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Debug, PartialEq, Eq)]
enum ChangedScope {
    /// Only `crates/<name>/` files changed — scoped gate possible
    Crates {
        crates: Vec<String>,
        has_markdown: bool,
    },
    /// Non-crate files changed — full workspace gate required
    Workspace,
    /// Nothing changed
    None,
}

fn get_changed_scope() -> Result<ChangedScope> {
    let diff_files = git_output(&["diff", "--name-only", "HEAD"])?;
    let untracked_files = git_output(&["ls-files", "--others", "--exclude-standard"])?;
    Ok(changed_scope_from_git_listings(
        &diff_files,
        &untracked_files,
    ))
}

fn changed_scope_from_git_listings(diff_files: &str, untracked_files: &str) -> ChangedScope {
    changed_scope_from_paths(diff_files.lines().chain(untracked_files.lines()))
}

fn changed_scope_from_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> ChangedScope {
    let mut changed_crates = HashSet::new();
    let mut has_non_crate_files = false;
    let mut has_markdown = false;

    for line in paths {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.ends_with(".md") {
            has_markdown = true;
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
        return ChangedScope::None;
    }

    if has_non_crate_files {
        return ChangedScope::Workspace;
    }

    let mut crates: Vec<String> = changed_crates.into_iter().collect();
    crates.sort();
    ChangedScope::Crates {
        crates,
        has_markdown,
    }
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

    // Dedup by allowlist identity (path + family + selector). `last_seen` is
    // advisory so multiple findings with the same selector identity collapse
    // into one proposed entry.
    let mut seen: BTreeSet<(String, String, String, String, Option<String>)> = BTreeSet::new();
    let mut deduped: Vec<&PanicFinding> = Vec::new();
    for finding in &findings {
        let key = (
            finding.path.clone(),
            finding.family.as_str().to_string(),
            finding.family.selector_kind().to_string(),
            finding.family.callee().to_string(),
            finding.container.clone(),
        );
        if seen.insert(key) {
            deduped.push(finding);
        }
    }

    let report_dir = root.join("target/policy");
    fs::create_dir_all(&report_dir)?;
    let report_path = report_dir.join("no-panic-proposed-allowlist.toml");

    let mut out = String::new();
    out.push_str("schema_version = \"0.3\"\n\n");
    out.push_str("# Proposed allowlist entries generated by `xtask no-panic propose`.\n");
    out.push_str("# Review each entry, set owner/classification/explanation/expires, then\n");
    out.push_str("# copy into policy/no-panic-allowlist.toml.\n\n");

    for (index, finding) in deduped.iter().enumerate() {
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
        "wrote {} proposed entr(ies) ({} raw findings deduped to {}) to {}",
        deduped.len(),
        findings.len(),
        deduped.len(),
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

    let tracked = git_output(&["ls-files", "--cached", "--others", "--exclude-standard"])?;
    let files = file_policy_inventory_from_git_listing(&tracked);

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
                "file-policy: stale entry pattern={} (no tracked or untracked non-ignored file matched)",
                entry.pattern
            );
        }
        return Err(anyhow!(
            "{} stale non-Rust allowlist entr(ies); remove or set retired = true",
            stale.len()
        ));
    }

    println!(
        "✅ file policy: {} tracked/untracked non-ignored file(s) checked, {} allowlist entr(ies)",
        files.len(),
        entries.len()
    );
    Ok(())
}

fn file_policy_inventory_from_git_listing(listing: &str) -> Vec<String> {
    listing
        .lines()
        .map(|s| s.trim().replace('\\', "/"))
        .filter(|s| !s.is_empty())
        .collect()
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

#[derive(Debug, PartialEq, Eq)]
struct MarkdownLocalLink {
    line: usize,
    target: String,
}

fn check_doc_links() -> Result<()> {
    println!("🔎 Checking Markdown local links...");
    let root = env::current_dir()?;
    let inventory = git_doc_link_inventory(&root)?;
    let stats = check_doc_links_with_inventory(&root, &inventory)?;
    println!(
        "✅ doc links: {} Markdown file(s), {} local link(s) checked",
        stats.markdown_files, stats.checked_links
    );
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct DocLinkCheckStats {
    markdown_files: usize,
    checked_links: usize,
}

#[derive(Debug)]
struct DocLinkInventory {
    markdown_files: Vec<PathBuf>,
    target_paths: BTreeSet<String>,
}

#[cfg(test)]
fn check_doc_links_at(root: &Path) -> Result<DocLinkCheckStats> {
    let inventory = filesystem_doc_link_inventory(root)?;
    check_doc_links_with_inventory(root, &inventory)
}

fn check_doc_links_with_inventory(
    root: &Path,
    inventory: &DocLinkInventory,
) -> Result<DocLinkCheckStats> {
    let mut missing = Vec::new();
    let mut checked_links = 0usize;

    for path in &inventory.markdown_files {
        let text = fs::read_to_string(path)?;
        let rel = relative_slash_path(root, path)?;
        for link in markdown_local_links(&text) {
            checked_links = checked_links.saturating_add(1);
            match resolve_doc_link_target(root, path, &link.target)? {
                Some(target) if inventory.target_paths.contains(&target) => {}
                Some(_) => {
                    missing.push(format!(
                        "{}:{} missing local link target `{}`",
                        rel, link.line, link.target
                    ));
                }
                None => {
                    missing.push(format!(
                        "{}:{} local link target escapes the repository `{}`",
                        rel, link.line, link.target
                    ));
                }
            }
        }
    }

    if !missing.is_empty() {
        for item in missing.iter().take(40) {
            eprintln!("doc-links: {item}");
        }
        if missing.len() > 40 {
            eprintln!(
                "doc-links: ... and {} more missing local link(s)",
                missing.len().saturating_sub(40)
            );
        }
        return Err(anyhow!(
            "{} Markdown local link(s) point at missing files",
            missing.len()
        ));
    }

    Ok(DocLinkCheckStats {
        markdown_files: inventory.markdown_files.len(),
        checked_links,
    })
}

fn git_doc_link_inventory(root: &Path) -> Result<DocLinkInventory> {
    let output = git_output(&["ls-files", "--cached", "--others", "--exclude-standard"])?;
    doc_link_inventory_from_repo_paths(root, output.lines())
}

#[cfg(test)]
fn filesystem_doc_link_inventory(root: &Path) -> Result<DocLinkInventory> {
    let mut inventory = DocLinkInventory {
        markdown_files: Vec::new(),
        target_paths: BTreeSet::new(),
    };
    collect_filesystem_doc_link_inventory(root, root, &mut inventory)?;
    inventory.markdown_files.sort();
    Ok(inventory)
}

fn doc_link_inventory_from_repo_paths<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<DocLinkInventory> {
    let mut inventory = DocLinkInventory {
        markdown_files: Vec::new(),
        target_paths: BTreeSet::new(),
    };

    for raw in paths {
        let rel = raw.trim().replace('\\', "/");
        if rel.is_empty() || should_skip_doc_link_rel(&rel) {
            continue;
        }
        insert_doc_link_target_path(&mut inventory.target_paths, &rel);
        if rel.ends_with(".md") {
            inventory.markdown_files.push(root.join(slash_path(&rel)));
        }
    }

    inventory.markdown_files.sort();
    Ok(inventory)
}

fn insert_doc_link_target_path(targets: &mut BTreeSet<String>, rel: &str) {
    targets.insert(rel.to_string());
    let mut parent = Path::new(rel).parent();
    while let Some(path) = parent {
        let as_string = path.to_string_lossy().replace('\\', "/");
        if as_string.is_empty() {
            break;
        }
        targets.insert(as_string);
        parent = path.parent();
    }
}

#[cfg(test)]
fn collect_filesystem_doc_link_inventory(
    root: &Path,
    dir: &Path,
    inventory: &mut DocLinkInventory,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        if path.is_dir() {
            if should_skip_doc_link_dir(name) {
                continue;
            }
            let rel = relative_slash_path(root, &path)?;
            inventory.target_paths.insert(rel);
            collect_filesystem_doc_link_inventory(root, &path, inventory)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let rel = relative_slash_path(root, &path)?;
            inventory.target_paths.insert(rel);
            inventory.markdown_files.push(path);
        } else {
            let rel = relative_slash_path(root, &path)?;
            inventory.target_paths.insert(rel);
        }
    }
    Ok(())
}

fn should_skip_doc_link_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | ".venv"
            | "venv"
            | ".mypy_cache"
            | ".pytest_cache"
            | "generated"
            | "vendor"
    )
}

fn should_skip_doc_link_rel(rel: &str) -> bool {
    rel.split('/').any(should_skip_doc_link_dir)
}

fn markdown_local_links(text: &str) -> Vec<MarkdownLocalLink> {
    let mut links = Vec::new();
    let mut in_fence = false;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        if let Some(raw) = markdown_reference_definition_target(line)
            && let Some(target) = markdown_link_target(raw)
            && is_local_markdown_target(&target)
        {
            links.push(MarkdownLocalLink {
                line: line_index.saturating_add(1),
                target,
            });
        }

        let mut offset = 0usize;
        while let Some(search) = line.get(offset..) {
            let Some(open_rel) = search.find('[') else {
                break;
            };
            let open = offset.saturating_add(open_rel);
            if open > 0 && line.as_bytes().get(open.saturating_sub(1)) == Some(&b'!') {
                offset = open.saturating_add(1);
                continue;
            }
            let Some(open_tail) = line.get(open..) else {
                break;
            };
            let Some(close_rel) = open_tail.find("](") else {
                break;
            };
            let target_start = open.saturating_add(close_rel).saturating_add(2);
            let Some(target_tail) = line.get(target_start..) else {
                break;
            };
            let Some(close_paren_rel) = target_tail.find(')') else {
                break;
            };
            let target_end = target_start.saturating_add(close_paren_rel);
            let Some(raw) = line.get(target_start..target_end).map(str::trim) else {
                break;
            };
            if let Some(target) = markdown_link_target(raw)
                && is_local_markdown_target(&target)
            {
                links.push(MarkdownLocalLink {
                    line: line_index.saturating_add(1),
                    target,
                });
            }
            offset = target_end.saturating_add(1);
        }
    }
    links
}

fn markdown_reference_definition_target(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let (_, target) = rest.split_once("]:")?;
    Some(target.trim())
}

fn markdown_link_target(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let target = if let Some(rest) = raw.strip_prefix('<') {
        let end = rest.find('>')?;
        rest.get(..end)?
    } else {
        raw.split_whitespace().next()?
    };
    let fragment_index = target.find('#');
    let query_index = target.find('?');
    let path_end = match (fragment_index, query_index) {
        (Some(fragment), Some(query)) => fragment.min(query),
        (Some(fragment), None) => fragment,
        (None, Some(query)) => query,
        (None, None) => target.len(),
    };
    let path = target.get(..path_end)?;
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

fn is_local_markdown_target(target: &str) -> bool {
    if target.starts_with('#') || target.starts_with('/') || target.starts_with('\\') {
        return false;
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("file:")
        || lower.starts_with("app://")
    {
        return false;
    }
    if let Some((scheme, _)) = target.split_once(':')
        && !scheme.is_empty()
        && scheme
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
    {
        return false;
    }
    true
}

fn percent_decode_path(path: &str) -> PathBuf {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%'
            && let (Some(high), Some(low)) = (
                bytes
                    .get(index.saturating_add(1))
                    .and_then(|b| hex_value(*b)),
                bytes
                    .get(index.saturating_add(2))
                    .and_then(|b| hex_value(*b)),
            )
        {
            decoded.push(high.saturating_mul(16).saturating_add(low));
            index = index.saturating_add(3);
            continue;
        }
        decoded.push(byte);
        index = index.saturating_add(1);
    }
    PathBuf::from(String::from_utf8_lossy(&decoded).replace('/', std::path::MAIN_SEPARATOR_STR))
}

fn resolve_doc_link_target(root: &Path, source: &Path, target: &str) -> Result<Option<String>> {
    let base = source
        .parent()
        .unwrap_or(root)
        .strip_prefix(root)
        .map_err(|err| {
            anyhow!(
                "source {} is not under workspace root {}: {err}",
                source.display(),
                root.display()
            )
        })?;
    let combined = base.join(percent_decode_path(target));
    let Some(normalized) = normalize_repo_relative_path(&combined) else {
        return Ok(None);
    };
    if normalized.as_os_str().is_empty() {
        return Ok(None);
    }
    Ok(Some(normalized.to_string_lossy().replace('\\', "/")))
}

fn normalize_repo_relative_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn slash_path(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|value| value.checked_add(10)),
        b'A'..=b'F' => byte
            .checked_sub(b'A')
            .and_then(|value| value.checked_add(10)),
        _ => None,
    }
}

fn relative_slash_path(root: &Path, path: &Path) -> Result<String> {
    let rel = path.strip_prefix(root).map_err(|err| {
        anyhow!(
            "path {} is not under workspace root {}: {err}",
            path.display(),
            root.display()
        )
    })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
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

// ---------------------------------------------------------------------------
// Python publish policy checker
// ---------------------------------------------------------------------------

struct PythonPublishWorkflowPolicy {
    path: &'static str,
    workflow_name: &'static str,
    input_name: &'static str,
    testpypi_proof_input: Option<&'static str>,
    publish_job: &'static str,
    install_job: &'static str,
    environment_name: &'static str,
    artifact_name: &'static str,
    package_index_url: &'static str,
    publish_repository_url: Option<&'static str>,
    publish_step_name: &'static str,
}

const PYTHON_PUBLISH_WORKFLOWS: &[PythonPublishWorkflowPolicy] = &[
    PythonPublishWorkflowPolicy {
        path: ".github/workflows/python-testpypi.yml",
        workflow_name: "Python TestPyPI Proof",
        input_name: "publish_to_testpypi",
        testpypi_proof_input: None,
        publish_job: "publish_testpypi",
        install_job: "install_from_testpypi",
        environment_name: "testpypi",
        artifact_name: "python-testpypi-wheel",
        package_index_url: "https://test.pypi.org/simple/",
        publish_repository_url: Some("https://test.pypi.org/legacy/"),
        publish_step_name: "Publish package distributions to TestPyPI",
    },
    PythonPublishWorkflowPolicy {
        path: ".github/workflows/python-pypi.yml",
        workflow_name: "Python PyPI Release Proof",
        input_name: "publish_to_pypi",
        testpypi_proof_input: Some("testpypi_proof_url"),
        publish_job: "publish_pypi",
        install_job: "install_from_pypi",
        environment_name: "pypi",
        artifact_name: "python-pypi-wheel",
        package_index_url: "https://pypi.org/simple/",
        publish_repository_url: None,
        publish_step_name: "Publish package distributions to PyPI",
    },
];

fn check_python_publish_policy() -> Result<()> {
    println!("🔎 Checking Python publish policy...");
    let root = env::current_dir()?;

    ensure_hl7v2_python_not_crates_io_published(&root)?;
    check_python_pyproject_policy(&root)?;
    for policy in PYTHON_PUBLISH_WORKFLOWS {
        check_python_publish_workflow(&root, policy)?;
    }

    println!(
        "✅ python publish policy: pyproject.toml and {} workflow(s) checked; hl7v2-python remains outside crates.io",
        PYTHON_PUBLISH_WORKFLOWS.len()
    );
    Ok(())
}

fn ensure_hl7v2_python_not_crates_io_published(root: &Path) -> Result<()> {
    let metadata = MetadataCommand::new().current_dir(root).no_deps().exec()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == "hl7v2-python")
        .ok_or_else(|| anyhow!("cargo metadata did not include hl7v2-python"))?;

    if package.publish.as_ref().is_some_and(Vec::is_empty) {
        Ok(())
    } else {
        Err(anyhow!(
            "crates/hl7v2-python/Cargo.toml must keep publish = false so Python stays outside the Rust crates.io graph"
        ))
    }
}

fn check_python_pyproject_policy(root: &Path) -> Result<()> {
    let text = fs::read_to_string(root.join("pyproject.toml"))?;
    check_python_pyproject_policy_text(&text)
}

fn check_python_pyproject_policy_text(text: &str) -> Result<()> {
    let pyproject: toml::Value = toml::from_str(text)
        .map_err(|error| anyhow!("pyproject.toml is not valid TOML: {error}"))?;

    ensure_pyproject_array_contains(
        &pyproject,
        "[build-system]",
        "requires",
        "maturin>=1.13.1,<2",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[build-system]",
        "build-backend",
        "maturin",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[project]",
        "name",
        "hl7v2-python",
        "pyproject.toml",
    )?;
    ensure_pyproject_array_contains(
        &pyproject,
        "[project]",
        "dynamic",
        "version",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[project]",
        "readme",
        "crates/hl7v2-python/README.md",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[project]",
        "requires-python",
        ">=3.10",
        "pyproject.toml",
    )?;
    ensure_pyproject_value_contains(
        &pyproject,
        "[project]",
        "license",
        "AGPL-3.0-or-later",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[tool.maturin]",
        "manifest-path",
        "crates/hl7v2-python/Cargo.toml",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[tool.maturin]",
        "module-name",
        "hl7v2",
        "pyproject.toml",
    )?;
    ensure_pyproject_string_value(
        &pyproject,
        "[tool.maturin]",
        "bindings",
        "pyo3",
        "pyproject.toml",
    )?;
    Ok(())
}

fn pyproject_value<'a>(
    pyproject: &'a toml::Value,
    section: &str,
    key: &str,
    context: &str,
) -> Result<&'a toml::Value> {
    let section_name = section
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| anyhow!("invalid TOML section marker `{section}`"))?;
    let mut current = pyproject;
    for part in section_name.split('.') {
        current = current
            .as_table()
            .and_then(|table| table.get(part))
            .ok_or_else(|| anyhow!("{context} {section} is missing"))?;
    }
    current
        .as_table()
        .and_then(|table| table.get(key))
        .ok_or_else(|| anyhow!("{context} {section}.{key} is missing"))
}

fn ensure_pyproject_string_value(
    pyproject: &toml::Value,
    section: &str,
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    let actual = pyproject_value(pyproject, section, key, context)?
        .as_str()
        .ok_or_else(|| anyhow!("{context} {section}.{key} must be a string"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} {section}.{key} must be `{expected}`, found `{actual}`"
        ))
    }
}

fn ensure_pyproject_array_contains(
    pyproject: &toml::Value,
    section: &str,
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    let values = pyproject_value(pyproject, section, key, context)?
        .as_array()
        .ok_or_else(|| anyhow!("{context} {section}.{key} must be an array"))?;
    if values
        .iter()
        .any(|value| value.as_str().is_some_and(|value| value == expected))
    {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} {section}.{key} must include `{expected}`"
        ))
    }
}

fn ensure_pyproject_value_contains(
    pyproject: &toml::Value,
    section: &str,
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    let value = pyproject_value(pyproject, section, key, context)?;
    let contains_expected = value.as_str().is_some_and(|value| value.contains(expected))
        || value.as_table().is_some_and(|table| {
            table
                .values()
                .any(|value| value.as_str().is_some_and(|value| value.contains(expected)))
        });
    if contains_expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} {section}.{key} must contain `{expected}`"
        ))
    }
}

fn check_python_publish_workflow(root: &Path, policy: &PythonPublishWorkflowPolicy) -> Result<()> {
    let text = fs::read_to_string(root.join(policy.path))?;
    check_python_publish_workflow_text(policy, &text)
}

fn check_python_publish_workflow_text(
    policy: &PythonPublishWorkflowPolicy,
    text: &str,
) -> Result<()> {
    let workflow: serde_yaml::Value = serde_yaml::from_str(text)
        .map_err(|error| anyhow!("{} is not valid YAML: {error}", policy.path))?;
    let root_map = yaml_mapping(&workflow, policy.path)?;

    ensure_yaml_string(root_map, policy.path, "name", policy.workflow_name)?;
    ensure_workflow_dispatch_input(policy, root_map)?;

    let permissions = yaml_child_mapping(root_map, policy.path, "permissions")?;
    ensure_yaml_permission(permissions, policy.path, "contents", "read")?;
    ensure_yaml_missing(permissions, policy.path, "id-token")?;

    let jobs = yaml_child_mapping(root_map, policy.path, "jobs")?;
    let wheel_job = yaml_mapping_child(jobs, policy.path, "jobs", "wheel_proof")?;
    ensure_python_non_publish_job_permissions(policy, wheel_job, "wheel_proof")?;
    ensure_python_publish_ref_guard(policy, wheel_job)?;
    ensure_python_production_preflight(policy, wheel_job)?;
    ensure_python_wheel_proof_job(policy, wheel_job)?;
    ensure_python_publish_artifact(policy, wheel_job)?;

    let publish_job = yaml_mapping_child(jobs, policy.path, "jobs", policy.publish_job)?;
    ensure_python_publish_job(policy, publish_job)?;

    let install_job = yaml_mapping_child(jobs, policy.path, "jobs", policy.install_job)?;
    ensure_python_non_publish_job_permissions(policy, install_job, policy.install_job)?;
    ensure_python_install_back_job(policy, install_job)?;

    Ok(())
}

fn ensure_workflow_dispatch_input(
    policy: &PythonPublishWorkflowPolicy,
    root_map: &serde_yaml::Mapping,
) -> Result<()> {
    let on_value = yaml_mapping_value_with_yaml11_bool_alias(root_map, policy.path, "on")?;
    let on_mapping = on_value
        .as_mapping()
        .ok_or_else(|| anyhow!("{} `on` must be a mapping", policy.path))?;
    let workflow_dispatch_key = serde_yaml::Value::String("workflow_dispatch".to_string());
    if on_mapping.len() != 1 || !on_mapping.contains_key(&workflow_dispatch_key) {
        return Err(anyhow!(
            "{} must be manual-only and define only `workflow_dispatch`",
            policy.path
        ));
    }

    let workflow_dispatch = on_mapping
        .get(&workflow_dispatch_key)
        .ok_or_else(|| anyhow!("{} is missing `workflow_dispatch`", policy.path))?
        .as_mapping()
        .ok_or_else(|| anyhow!("{} `workflow_dispatch` must be a mapping", policy.path))?;
    let inputs = yaml_child_mapping(workflow_dispatch, policy.path, "inputs")?;
    let input = yaml_mapping_child(
        inputs,
        policy.path,
        "workflow_dispatch.inputs",
        policy.input_name,
    )?;

    ensure_yaml_string(input, policy.path, "type", "boolean")?;
    ensure_yaml_bool(input, policy.path, "required", true)?;
    ensure_yaml_bool(input, policy.path, "default", false)?;
    if let Some(testpypi_proof_input) = policy.testpypi_proof_input {
        let input = yaml_mapping_child(
            inputs,
            policy.path,
            "workflow_dispatch.inputs",
            testpypi_proof_input,
        )?;
        ensure_yaml_string(input, policy.path, "type", "string")?;
        ensure_yaml_bool(input, policy.path, "required", false)?;
        ensure_yaml_string(input, policy.path, "default", "")?;
    }
    Ok(())
}

fn ensure_python_publish_ref_guard(
    policy: &PythonPublishWorkflowPolicy,
    wheel_job: &serde_yaml::Mapping,
) -> Result<()> {
    let steps = yaml_child_sequence(wheel_job, policy.path, "steps")?;
    let guard = yaml_step_named(steps, policy.path, "Validate publish ref")?;
    let condition = yaml_mapping_string(guard, policy.path, "if")?;
    if !condition.contains(policy.input_name) || !condition.contains("refs/heads/main") {
        return Err(anyhow!(
            "{} Validate publish ref condition must require `{}` and refs/heads/main",
            policy.path,
            policy.input_name
        ));
    }
    let run = yaml_mapping_string(guard, policy.path, "run")?;
    if !run.contains("refs/heads/main") || !run.contains("exit 1") {
        return Err(anyhow!(
            "{} Validate publish ref step must fail closed outside main",
            policy.path
        ));
    }
    Ok(())
}

fn ensure_python_production_preflight(
    policy: &PythonPublishWorkflowPolicy,
    wheel_job: &serde_yaml::Mapping,
) -> Result<()> {
    let Some(testpypi_proof_input) = policy.testpypi_proof_input else {
        return Ok(());
    };

    let permissions = yaml_child_mapping(wheel_job, policy.path, "permissions")?;
    ensure_yaml_permission(permissions, policy.path, "contents", "read")?;
    ensure_yaml_permission(permissions, policy.path, "actions", "read")?;
    ensure_yaml_missing(permissions, policy.path, "id-token")?;

    let steps = yaml_child_sequence(wheel_job, policy.path, "steps")?;
    let preflight = yaml_step_named(steps, policy.path, "Validate production PyPI preconditions")?;
    let condition = yaml_mapping_string(preflight, policy.path, "if")?;
    if !condition.contains(policy.input_name) {
        return Err(anyhow!(
            "{} production PyPI preflight must be gated by `{}`",
            policy.path,
            policy.input_name
        ));
    }
    let env = yaml_child_mapping(preflight, policy.path, "env")?;
    ensure_yaml_string(
        env,
        policy.path,
        "PACKAGE_VERSION",
        "${{ steps.package.outputs.version }}",
    )?;
    ensure_yaml_string(
        env,
        policy.path,
        "TESTPYPI_PROOF_URL",
        "${{ inputs.testpypi_proof_url }}",
    )?;
    ensure_yaml_string(env, policy.path, "GITHUB_TOKEN", "${{ github.token }}")?;
    ensure_yaml_string(env, policy.path, "GITHUB_SHA", "${{ github.sha }}")?;
    let run = yaml_mapping_string(preflight, policy.path, "run")?;
    for expected in [
        testpypi_proof_input,
        "https://github\\.com/EffortlessMetrics/hl7v2-rs/actions/runs/",
        "https://api.github.com/repos/EffortlessMetrics/hl7v2-rs/actions/runs/",
        "https://api.github.com/repos/EffortlessMetrics/hl7v2-rs/actions/runs/{run_id}/jobs?per_page=100",
        "Python TestPyPI Proof",
        "Publish to TestPyPI",
        "Install from TestPyPI and smoke",
        "workflow_dispatch",
        "head_branch",
        "head_sha",
        "conclusion",
        "success",
        "job_conclusions",
        "https://test.pypi.org/pypi/{package}/json",
        "https://pypi.org/pypi/{package}/json",
        "version not in testpypi_versions",
        "version in pypi_versions",
        "sys.exit(1)",
    ] {
        if !run.contains(expected) {
            return Err(anyhow!(
                "{} production PyPI preflight step must contain `{expected}`",
                policy.path
            ));
        }
    }
    Ok(())
}

fn ensure_python_non_publish_job_permissions(
    policy: &PythonPublishWorkflowPolicy,
    job: &serde_yaml::Mapping,
    job_name: &str,
) -> Result<()> {
    if let Some(permissions) = job.get(serde_yaml::Value::String("permissions".to_string())) {
        let permissions = permissions
            .as_mapping()
            .ok_or_else(|| anyhow!("{} `{job_name}.permissions` must be a mapping", policy.path))?;
        ensure_yaml_missing(permissions, policy.path, "id-token")?;
    }
    Ok(())
}

fn ensure_python_wheel_proof_job(
    policy: &PythonPublishWorkflowPolicy,
    wheel_job: &serde_yaml::Mapping,
) -> Result<()> {
    let steps = yaml_child_sequence(wheel_job, policy.path, "steps")?;
    let install_maturin = yaml_step_named(steps, policy.path, "Install maturin")?;
    let install_maturin_run = yaml_mapping_string(install_maturin, policy.path, "run")?;
    if !install_maturin_run.contains("maturin==1.13.1") {
        return Err(anyhow!(
            "{} Install maturin step must pin maturin==1.13.1",
            policy.path
        ));
    }

    let build = yaml_step_named(steps, policy.path, "Build wheel")?;
    let build_run = yaml_mapping_string(build, policy.path, "run")?;
    if !build_run.contains("maturin build --release --out dist") {
        return Err(anyhow!(
            "{} Build wheel step must run `maturin build --release --out dist`",
            policy.path
        ));
    }

    let smoke = yaml_step_named(steps, policy.path, "Install built wheel in fresh venv")?;
    let smoke_run = yaml_mapping_string(smoke, policy.path, "run")?;
    for expected in [
        "python -m venv",
        "python -m pip install --force-reinstall dist/*.whl",
        "tests/python_smoke/smoke.py",
        "tests/python_smoke/evidence_workflow_guide.py",
    ] {
        if !smoke_run.contains(expected) {
            return Err(anyhow!(
                "{} local wheel proof step must contain `{expected}`",
                policy.path
            ));
        }
    }
    Ok(())
}

fn ensure_python_publish_artifact(
    policy: &PythonPublishWorkflowPolicy,
    wheel_job: &serde_yaml::Mapping,
) -> Result<()> {
    let steps = yaml_child_sequence(wheel_job, policy.path, "steps")?;
    let upload = yaml_step_named(steps, policy.path, "Upload wheel artifact")?;
    ensure_yaml_string(upload, policy.path, "uses", "actions/upload-artifact@v7")?;
    let with = yaml_child_mapping(upload, policy.path, "with")?;
    ensure_yaml_string(with, policy.path, "name", policy.artifact_name)?;
    ensure_yaml_string(with, policy.path, "path", "dist/*.whl")?;
    Ok(())
}

fn ensure_python_publish_job(
    policy: &PythonPublishWorkflowPolicy,
    publish_job: &serde_yaml::Mapping,
) -> Result<()> {
    let condition = yaml_mapping_string(publish_job, policy.path, "if")?;
    if !condition.contains(policy.input_name) {
        return Err(anyhow!(
            "{} publish job must be gated by `{}`",
            policy.path,
            policy.input_name
        ));
    }

    let environment = yaml_child_mapping(publish_job, policy.path, "environment")?;
    ensure_yaml_string(environment, policy.path, "name", policy.environment_name)?;
    let permissions = yaml_child_mapping(publish_job, policy.path, "permissions")?;
    ensure_yaml_permission(permissions, policy.path, "contents", "read")?;
    ensure_yaml_permission(permissions, policy.path, "id-token", "write")?;
    ensure_yaml_mapping_has_no_forbidden_text(
        publish_job,
        policy.path,
        "publish job",
        &[
            "secrets.",
            "PYPI_API_TOKEN",
            "TEST_PYPI_API_TOKEN",
            "TWINE_PASSWORD",
            "TWINE_USERNAME",
        ],
    )?;

    let steps = yaml_child_sequence(publish_job, policy.path, "steps")?;
    let download = yaml_step_named(steps, policy.path, "Download wheel artifact")?;
    ensure_yaml_string(
        download,
        policy.path,
        "uses",
        "actions/download-artifact@v7",
    )?;
    let download_with = yaml_child_mapping(download, policy.path, "with")?;
    ensure_yaml_string(download_with, policy.path, "name", policy.artifact_name)?;
    ensure_yaml_string(download_with, policy.path, "path", "dist")?;

    let publish = yaml_step_named(steps, policy.path, policy.publish_step_name)?;
    ensure_yaml_string(
        publish,
        policy.path,
        "uses",
        "pypa/gh-action-pypi-publish@v1.14.0",
    )?;
    let publish_with = yaml_child_mapping(publish, policy.path, "with")?;
    ensure_yaml_string(publish_with, policy.path, "packages-dir", "dist/")?;
    match policy.publish_repository_url {
        Some(expected) => {
            ensure_yaml_string(publish_with, policy.path, "repository-url", expected)?
        }
        None => ensure_yaml_missing(publish_with, policy.path, "repository-url")?,
    }
    for forbidden in ["password", "user", "skip-existing"] {
        ensure_yaml_missing(publish_with, policy.path, forbidden)?;
    }
    Ok(())
}

fn ensure_python_install_back_job(
    policy: &PythonPublishWorkflowPolicy,
    install_job: &serde_yaml::Mapping,
) -> Result<()> {
    let condition = yaml_mapping_string(install_job, policy.path, "if")?;
    if !condition.contains(policy.input_name) {
        return Err(anyhow!(
            "{} install-back job must be gated by `{}`",
            policy.path,
            policy.input_name
        ));
    }

    let needs = yaml_child_sequence(install_job, policy.path, "needs")?;
    ensure_yaml_sequence_contains(needs, policy.path, "needs", "wheel_proof")?;
    ensure_yaml_sequence_contains(needs, policy.path, "needs", policy.publish_job)?;

    let steps = yaml_child_sequence(install_job, policy.path, "steps")?;
    let install = steps
        .iter()
        .filter_map(serde_yaml::Value::as_mapping)
        .find(|step| {
            yaml_mapping_string(step, policy.path, "name")
                .is_ok_and(|name| name.contains("Install published wheel from"))
        })
        .ok_or_else(|| anyhow!("{} is missing install-back step", policy.path))?;
    let run = yaml_mapping_string(install, policy.path, "run")?;
    for expected in [
        policy.package_index_url,
        "--no-deps",
        "--force-reinstall",
        "hl7v2-python==${PACKAGE_VERSION}",
        "tests/python_smoke/smoke.py",
        "tests/python_smoke/evidence_workflow_guide.py",
    ] {
        if !run.contains(expected) {
            return Err(anyhow!(
                "{} install-back step must contain `{expected}`",
                policy.path
            ));
        }
    }
    Ok(())
}

fn yaml_mapping<'a>(
    value: &'a serde_yaml::Value,
    context: &str,
) -> Result<&'a serde_yaml::Mapping> {
    value
        .as_mapping()
        .ok_or_else(|| anyhow!("{context} must be a YAML mapping"))
}

fn yaml_child_mapping<'a>(
    mapping: &'a serde_yaml::Mapping,
    context: &str,
    key: &str,
) -> Result<&'a serde_yaml::Mapping> {
    yaml_mapping_value(mapping, context, key)?
        .as_mapping()
        .ok_or_else(|| anyhow!("{context} `{key}` must be a mapping"))
}

fn yaml_mapping_child<'a>(
    mapping: &'a serde_yaml::Mapping,
    context: &str,
    parent: &str,
    key: &str,
) -> Result<&'a serde_yaml::Mapping> {
    yaml_mapping_value(mapping, context, key)?
        .as_mapping()
        .ok_or_else(|| anyhow!("{context} `{parent}.{key}` must be a mapping"))
}

fn yaml_child_sequence<'a>(
    mapping: &'a serde_yaml::Mapping,
    context: &str,
    key: &str,
) -> Result<&'a Vec<serde_yaml::Value>> {
    yaml_mapping_value(mapping, context, key)?
        .as_sequence()
        .ok_or_else(|| anyhow!("{context} `{key}` must be a sequence"))
}

fn yaml_mapping_value<'a>(
    mapping: &'a serde_yaml::Mapping,
    context: &str,
    key: &str,
) -> Result<&'a serde_yaml::Value> {
    mapping
        .get(serde_yaml::Value::String(key.to_string()))
        .ok_or_else(|| anyhow!("{context} is missing `{key}`"))
}

fn yaml_mapping_value_with_yaml11_bool_alias<'a>(
    mapping: &'a serde_yaml::Mapping,
    context: &str,
    key: &str,
) -> Result<&'a serde_yaml::Value> {
    mapping
        .get(serde_yaml::Value::String(key.to_string()))
        .or_else(|| match key {
            "on" => mapping.get(serde_yaml::Value::Bool(true)),
            "off" => mapping.get(serde_yaml::Value::Bool(false)),
            _ => None,
        })
        .ok_or_else(|| anyhow!("{context} is missing `{key}`"))
}

fn yaml_mapping_string<'a>(
    mapping: &'a serde_yaml::Mapping,
    context: &str,
    key: &str,
) -> Result<&'a str> {
    yaml_mapping_value(mapping, context, key)?
        .as_str()
        .ok_or_else(|| anyhow!("{context} `{key}` must be a string"))
}

fn ensure_yaml_string(
    mapping: &serde_yaml::Mapping,
    context: &str,
    key: &str,
    expected: &str,
) -> Result<()> {
    let actual = yaml_mapping_string(mapping, context, key)?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} `{key}` must be `{expected}`, found `{actual}`"
        ))
    }
}

fn ensure_yaml_bool(
    mapping: &serde_yaml::Mapping,
    context: &str,
    key: &str,
    expected: bool,
) -> Result<()> {
    let actual = yaml_mapping_value(mapping, context, key)?
        .as_bool()
        .ok_or_else(|| anyhow!("{context} `{key}` must be a boolean"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} `{key}` must be `{expected}`, found `{actual}`"
        ))
    }
}

fn ensure_yaml_permission(
    mapping: &serde_yaml::Mapping,
    context: &str,
    key: &str,
    expected: &str,
) -> Result<()> {
    ensure_yaml_string(mapping, context, key, expected)
}

fn ensure_yaml_missing(mapping: &serde_yaml::Mapping, context: &str, key: &str) -> Result<()> {
    if mapping
        .get(serde_yaml::Value::String(key.to_string()))
        .is_some()
    {
        Err(anyhow!("{context} must not set `{key}` at this scope"))
    } else {
        Ok(())
    }
}

fn yaml_step_named<'a>(
    steps: &'a [serde_yaml::Value],
    context: &str,
    name: &str,
) -> Result<&'a serde_yaml::Mapping> {
    steps
        .iter()
        .filter_map(serde_yaml::Value::as_mapping)
        .find(|step| yaml_mapping_string(step, context, "name").is_ok_and(|value| value == name))
        .ok_or_else(|| anyhow!("{context} is missing step `{name}`"))
}

fn ensure_yaml_sequence_contains(
    sequence: &[serde_yaml::Value],
    context: &str,
    key: &str,
    expected: &str,
) -> Result<()> {
    if sequence
        .iter()
        .any(|value| value.as_str().is_some_and(|actual| actual == expected))
    {
        Ok(())
    } else {
        Err(anyhow!("{context} `{key}` must include `{expected}`"))
    }
}

fn ensure_yaml_mapping_has_no_forbidden_text(
    mapping: &serde_yaml::Mapping,
    context: &str,
    label: &str,
    forbidden_values: &[&str],
) -> Result<()> {
    let value = serde_yaml::Value::Mapping(mapping.clone());
    if let Some(forbidden) = yaml_value_forbidden_text(&value, forbidden_values) {
        Err(anyhow!(
            "{context} {label} must not reference `{forbidden}`; use Trusted Publishing instead"
        ))
    } else {
        Ok(())
    }
}

fn yaml_value_forbidden_text<'a>(
    value: &serde_yaml::Value,
    forbidden_values: &'a [&'a str],
) -> Option<&'a str> {
    match value {
        serde_yaml::Value::String(text) => forbidden_values
            .iter()
            .copied()
            .find(|forbidden| text.contains(forbidden)),
        serde_yaml::Value::Sequence(sequence) => sequence
            .iter()
            .find_map(|item| yaml_value_forbidden_text(item, forbidden_values)),
        serde_yaml::Value::Mapping(mapping) => mapping.iter().find_map(|(key, value)| {
            yaml_value_forbidden_text(key, forbidden_values)
                .or_else(|| yaml_value_forbidden_text(value, forbidden_values))
        }),
        _ => None,
    }
}

// ============================================================================
// CI Lane Whitelist
// ============================================================================

struct CiLaneEntry {
    id: String,
    workflow: String,
    job: String,
    owner: String,
    intent: String,
    failure_mode: String,
    proof_obligation: String,
    evidence: Vec<String>,
    duplicate_of: Vec<String>,
    default_pr: bool,
    blocking: bool,
    expensive: bool,
    default_pr_exception: Option<String>,
    expires: String,
}

struct CiException {
    id: String,
    lane: String,
    allowed: bool,
    expires: String,
}

struct CiRiskPack {
    name: String,
    lanes: Vec<String>,
    deep_lanes: Vec<String>,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct CiDate {
    year: u16,
    month: u8,
    day: u8,
}

fn parse_ci_date(value: &str, context: &str) -> Result<CiDate> {
    let mut parts = value.split('-');
    let year = parts
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("{context}: invalid date `{value}`; expected YYYY-MM-DD"))?;
    let month = parts
        .next()
        .and_then(|p| p.parse::<u8>().ok())
        .ok_or_else(|| anyhow!("{context}: invalid date `{value}`; expected YYYY-MM-DD"))?;
    let day = parts
        .next()
        .and_then(|p| p.parse::<u8>().ok())
        .ok_or_else(|| anyhow!("{context}: invalid date `{value}`; expected YYYY-MM-DD"))?;
    if parts.next().is_some()
        || value.len() != 10
        || !matches!(month, 1..=12)
        || !matches!(day, 1..=31)
    {
        return Err(anyhow!(
            "{context}: invalid date `{value}`; expected YYYY-MM-DD"
        ));
    }
    Ok(CiDate { year, month, day })
}

fn parse_ci_lane_whitelist(text: &str) -> Result<Vec<CiLaneEntry>> {
    let raw_entries = table_array_entries(text, "[[lane]]");
    let mut out = Vec::with_capacity(raw_entries.len());
    for (idx, raw) in raw_entries.iter().enumerate() {
        let n = idx.checked_add(1).unwrap_or(idx);
        let field = |key: &str| -> Result<String> {
            top_level_quoted_value(raw, key).ok_or_else(|| {
                anyhow!("ci-lane-whitelist.toml entry {n}: missing required field `{key}`")
            })
        };
        let id = field("id")?;
        let workflow = field("workflow")?;
        let job = field("job")?;
        let owner = field("owner")?;
        let intent = field("intent")?;
        let failure_mode = field("failure_mode")?;
        let proof_obligation = field("proof_obligation")?;
        let expires = field("expires")?;
        parse_ci_date(
            &expires,
            &format!("ci-lane-whitelist.toml entry {n} (id={id}) expires"),
        )?;
        let evidence = string_array_after_root(raw, "evidence").unwrap_or_default();
        let duplicate_of = string_array_after_root(raw, "duplicate_of").unwrap_or_default();
        let default_pr = top_level_quoted_value(raw, "default_pr")
            .map(|v| v == "true")
            .unwrap_or(false);
        let blocking = top_level_quoted_value(raw, "blocking")
            .map(|v| v == "true")
            .unwrap_or(false);
        let expensive = top_level_quoted_value(raw, "expensive")
            .map(|v| v == "true")
            .unwrap_or(false);
        let default_pr_exception = top_level_quoted_value(raw, "default_pr_exception");

        if workflow.is_empty() || !workflow.starts_with(".github/workflows/") {
            return Err(anyhow!(
                "ci-lane-whitelist.toml entry {n} (id={id}): `workflow` must start with `.github/workflows/`"
            ));
        }
        if job.is_empty() {
            return Err(anyhow!(
                "ci-lane-whitelist.toml entry {n} (id={id}): `job` must not be empty"
            ));
        }

        out.push(CiLaneEntry {
            id,
            workflow,
            job,
            owner,
            intent,
            failure_mode,
            proof_obligation,
            evidence,
            duplicate_of,
            default_pr,
            blocking,
            expensive,
            default_pr_exception,
            expires,
        });
    }
    Ok(out)
}

fn parse_ci_exceptions(text: &str) -> Result<Vec<CiException>> {
    let raw_entries = table_array_entries(text, "[[exception]]");
    let mut out = Vec::with_capacity(raw_entries.len());
    for (idx, raw) in raw_entries.iter().enumerate() {
        let n = idx.checked_add(1).unwrap_or(idx);
        let field = |key: &str| -> Result<String> {
            top_level_quoted_value(raw, key).ok_or_else(|| {
                anyhow!("ci-whitelist-exceptions.toml entry {n}: missing required field `{key}`")
            })
        };
        let id = field("id")?;
        let lane = field("lane")?;
        let expires = field("expires")?;
        parse_ci_date(
            &expires,
            &format!("ci-whitelist-exceptions.toml entry {n} (id={id}) expires"),
        )?;
        let allowed = top_level_quoted_value(raw, "allowed")
            .map(|v| v == "true")
            .unwrap_or(false);
        out.push(CiException {
            id,
            lane,
            allowed,
            expires,
        });
    }
    Ok(out)
}

fn parse_ci_risk_packs(text: &str) -> Vec<CiRiskPack> {
    let mut packs = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[risk_pack.") && trimmed.ends_with(']') {
            if let Some(name) = current_name.take() {
                let raw = current_lines.join("\n");
                packs.push(CiRiskPack {
                    name,
                    lanes: string_array_after_root(&raw, "lanes").unwrap_or_default(),
                    deep_lanes: string_array_after_root(&raw, "deep_lanes").unwrap_or_default(),
                });
                current_lines.clear();
            }
            let name = trimmed
                .trim_start_matches("[risk_pack.")
                .trim_end_matches(']')
                .to_string();
            current_name = Some(name);
            continue;
        }

        if current_name.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(name) = current_name {
        let raw = current_lines.join("\n");
        packs.push(CiRiskPack {
            name,
            lanes: string_array_after_root(&raw, "lanes").unwrap_or_default(),
            deep_lanes: string_array_after_root(&raw, "deep_lanes").unwrap_or_default(),
        });
    }

    packs
}

fn workflow_declares_job(workflow_text: &str, job: &str) -> bool {
    let mut in_jobs = false;
    for line in workflow_text.lines() {
        let trimmed = line.trim_end();
        if trimmed == "jobs:" {
            in_jobs = true;
            continue;
        }

        if !in_jobs {
            continue;
        }

        if !line.starts_with(' ') && !trimmed.is_empty() {
            break;
        }

        if line.starts_with("  ") && !line.starts_with("    ") {
            let candidate = trimmed.trim().trim_end_matches(':');
            if candidate == job {
                return true;
            }
        }
    }
    false
}

fn today_iso() -> String {
    if let Ok(d) = env::var("CI_TODAY") {
        return d;
    }
    for (cmd, args) in [
        ("date", vec!["+%Y-%m-%d"]),
        (
            "powershell",
            vec!["-NoProfile", "-Command", "Get-Date -Format yyyy-MM-dd"],
        ),
    ] {
        if let Ok(output) = Command::new(cmd).args(args).output()
            && output.status.success()
        {
            let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !date.is_empty() {
                return date;
            }
        }
    }
    "1970-01-01".to_string()
}

fn check_ci_lane_whitelist() -> Result<()> {
    println!("🔎 Checking CI lane whitelist...");
    let root = env::current_dir()?;

    let whitelist_text = fs::read_to_string(root.join("policy/ci-lane-whitelist.toml"))
        .map_err(|e| anyhow!("Cannot read policy/ci-lane-whitelist.toml: {e}"))?;
    let exceptions_text = fs::read_to_string(root.join("policy/ci-whitelist-exceptions.toml"))
        .map_err(|e| anyhow!("Cannot read policy/ci-whitelist-exceptions.toml: {e}"))?;
    let risk_pack_text = fs::read_to_string(root.join("policy/ci-risk-packs.toml"))
        .map_err(|e| anyhow!("Cannot read policy/ci-risk-packs.toml: {e}"))?;

    let lanes = parse_ci_lane_whitelist(&whitelist_text)?;
    let exceptions = parse_ci_exceptions(&exceptions_text)?;
    let risk_packs = parse_ci_risk_packs(&risk_pack_text);

    let today = today_iso();
    let today_date = parse_ci_date(&today, "current date")?;
    let mut lane_ids: HashSet<String> = HashSet::new();
    let mut exception_by_id: HashMap<String, &CiException> = HashMap::new();

    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for lane in &lanes {
        if !lane_ids.insert(lane.id.clone()) {
            errors.push(format!("duplicate CI lane id '{}'", lane.id));
        }
    }

    for ex in &exceptions {
        if exception_by_id.insert(ex.id.clone(), ex).is_some() {
            errors.push(format!("duplicate CI exception id '{}'", ex.id));
        }
    }

    for ex in &exceptions {
        let expires = parse_ci_date(&ex.expires, "exception expires")?;
        if expires < today_date {
            warnings.push(format!(
                "exception '{}' for lane '{}' expired on {} (today: {}); update or remove",
                ex.id, ex.lane, ex.expires, today
            ));
        }
        if !lane_ids.contains(&ex.lane) {
            warnings.push(format!(
                "exception '{}' references unknown lane '{}'",
                ex.id, ex.lane
            ));
        }
    }

    for lane in &lanes {
        let expires = parse_ci_date(&lane.expires, "lane expires")?;
        if expires < today_date {
            warnings.push(format!(
                "lane '{}' `expires` date {} has passed (today: {}); review required",
                lane.id, lane.expires, today
            ));
        }

        for (fname, fval) in [
            ("intent", lane.intent.as_str()),
            ("failure_mode", lane.failure_mode.as_str()),
            ("proof_obligation", lane.proof_obligation.as_str()),
            ("owner", lane.owner.as_str()),
            ("workflow", lane.workflow.as_str()),
            ("job", lane.job.as_str()),
        ] {
            if fval.is_empty() {
                errors.push(format!(
                    "lane '{}' has empty required field `{fname}`",
                    lane.id
                ));
            }
        }

        if lane.blocking && lane.evidence.is_empty() {
            warnings.push(format!(
                "blocking lane '{}' has an empty evidence list",
                lane.id
            ));
        }

        let workflow_path = root.join(&lane.workflow);
        match fs::read_to_string(&workflow_path) {
            Ok(workflow_text) => {
                if lane.job != "*" && !workflow_declares_job(&workflow_text, &lane.job) {
                    errors.push(format!(
                        "lane '{}' declares job '{}' but {} does not contain that job id",
                        lane.id, lane.job, lane.workflow
                    ));
                }
            }
            Err(e) => errors.push(format!(
                "lane '{}' declares workflow '{}' but it cannot be read: {e}",
                lane.id, lane.workflow
            )),
        }

        for dep in &lane.duplicate_of {
            if !lane_ids.contains(dep) {
                warnings.push(format!(
                    "lane '{}' duplicate_of references unknown lane '{}'",
                    lane.id, dep
                ));
            }
        }

        if lane.default_pr && lane.expensive {
            match &lane.default_pr_exception {
                None => {
                    errors.push(format!(
                        "lane '{}' has default_pr=true and expensive=true but no default_pr_exception",
                        lane.id
                    ));
                }
                Some(exc_id) => match exception_by_id.get(exc_id) {
                    None => {
                        errors.push(format!(
                            "lane '{}' default_pr_exception '{}' not found in ci-whitelist-exceptions.toml",
                            lane.id, exc_id
                        ));
                    }
                    Some(ex) => {
                        if ex.lane != lane.id {
                            errors.push(format!(
                                "lane '{}' default_pr_exception '{}' belongs to lane '{}'",
                                lane.id, exc_id, ex.lane
                            ));
                        }
                        if !ex.allowed {
                            errors.push(format!(
                                "lane '{}' default_pr_exception '{}' has allowed=false",
                                lane.id, exc_id
                            ));
                        }
                        let expires = parse_ci_date(&ex.expires, "exception expires")?;
                        if expires < today_date {
                            errors.push(format!(
                                "lane '{}' default_pr_exception '{}' expired on {}; remove expensive=true or renew exception",
                                lane.id, exc_id, ex.expires
                            ));
                        }
                    }
                },
            }
        }
    }

    for pack in &risk_packs {
        for lane in pack.lanes.iter().chain(pack.deep_lanes.iter()) {
            if !lane_ids.contains(lane) {
                errors.push(format!(
                    "risk pack '{}' references unknown lane '{}'",
                    pack.name, lane
                ));
            }
        }
    }

    for w in &warnings {
        eprintln!("ci-lane-whitelist: warning: {w}");
    }
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ci-lane-whitelist: error: {e}");
        }
        return Err(anyhow!(
            "ci-lane-whitelist: {} error(s), {} warning(s)",
            errors.len(),
            warnings.len()
        ));
    }

    println!(
        "✅ ci-lane-whitelist: {} lane(s), {} exception(s), {} warning(s)",
        lanes.len(),
        exceptions.len(),
        warnings.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn read_policy_workflow_for_mutation(
        root: &Path,
        policy: &PythonPublishWorkflowPolicy,
    ) -> Result<String> {
        let workflow = fs::read_to_string(root.join(policy.path))?;
        Ok(workflow.replace("\r\n", "\n"))
    }

    #[test]
    fn ripr_plus_badge_shape_is_stable() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "0".to_string(),
            color: "brightgreen".to_string(),
        };

        validate_shields_badge(&badge, Some("ripr+"))
    }

    #[test]
    fn shields_badge_rejects_empty_message() {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "".to_string(),
            color: "brightgreen".to_string(),
        };

        assert!(validate_shields_badge(&badge, Some("ripr+")).is_err());
    }

    #[test]
    fn publish_order_uses_workspace_dependency_order() -> Result<()> {
        let ordered = publish_order(None)?;

        for public_surface in ["hl7v2", "hl7v2-server", "hl7v2-cli"] {
            ensure_contains(&ordered, public_surface)?;
        }
        ensure_not_contains(&ordered, "hl7v2-python")?;
        for frozen_shim in [
            "hl7v2-ack",
            "hl7v2-batch",
            "hl7v2-core",
            "hl7v2-corpus",
            "hl7v2-datatype",
            "hl7v2-datetime",
            "hl7v2-escape",
            "hl7v2-faker",
            "hl7v2-gen",
            "hl7v2-guard",
            "hl7v2-json",
            "hl7v2-lifecycle",
            "hl7v2-mllp",
            "hl7v2-model",
            "hl7v2-network",
            "hl7v2-normalize",
            "hl7v2-parser",
            "hl7v2-path",
            "hl7v2-prof",
            "hl7v2-query",
            "hl7v2-redact",
            "hl7v2-stream",
            "hl7v2-template",
            "hl7v2-template-values",
            "hl7v2-validation",
            "hl7v2-writer",
        ] {
            ensure_not_contains(&ordered, frozen_shim)?;
        }
        if ordered.iter().any(|crate_name| crate_name == "xtask") {
            return Err(anyhow!("xtask should not be publishable"));
        }

        assert_dependency_precedes(&ordered, "hl7v2", "hl7v2-server")?;
        assert_dependency_precedes(&ordered, "hl7v2", "hl7v2-cli")?;
        Ok(())
    }

    #[test]
    fn publish_order_can_resume_from_a_named_crate() -> Result<()> {
        let ordered = publish_order(None)?;
        let resumed = publish_order(Some("hl7v2"))?;
        let start = ordered
            .iter()
            .position(|crate_name| crate_name == "hl7v2")
            .ok_or_else(|| anyhow!("hl7v2 should be publishable"))?;
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
    fn workspace_patch_dependencies_exclude_private_shims() -> Result<()> {
        let metadata = MetadataCommand::new().exec()?;
        let packages = publishable_workspace_packages(&metadata);
        let dependencies = internal_workspace_dependency_closure("hl7v2", &packages)?;

        for excluded in [
            "hl7v2-model",
            "hl7v2-escape",
            "hl7v2-mllp",
            "hl7v2-parser",
            "hl7v2-query",
            "hl7v2-test-utils",
        ] {
            if dependencies.contains(excluded) {
                return Err(anyhow!(
                    "workspace patch dependency closure should exclude non-publishable crate {excluded}"
                ));
            }
        }
        Ok(())
    }

    fn ensure_not_contains(ordered: &[String], crate_name: &str) -> Result<()> {
        if ordered.iter().any(|name| name == crate_name) {
            return Err(anyhow!(
                "{crate_name} should not be present in publish order"
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

    // ---- evidence schema mapping ----------------------------------------

    #[test]
    fn evidence_schema_mapping_uses_legacy_v1_fixture_name() -> Result<()> {
        let fixtures = BTreeSet::from(["validation-report.json".to_string()]);

        let fixture =
            evidence_fixture_name_for_schema("validation-report-v1.schema.json", &fixtures)?;

        if fixture == "validation-report.json" {
            Ok(())
        } else {
            Err(anyhow!("expected validation-report.json, got {fixture}"))
        }
    }

    #[test]
    fn evidence_schema_mapping_uses_versioned_v2_fixture_name() -> Result<()> {
        let fixtures = BTreeSet::from(["validation-report-v2.json".to_string()]);

        let fixture =
            evidence_fixture_name_for_schema("validation-report-v2.schema.json", &fixtures)?;

        if fixture == "validation-report-v2.json" {
            Ok(())
        } else {
            Err(anyhow!("expected validation-report-v2.json, got {fixture}"))
        }
    }

    #[test]
    fn evidence_schema_mapping_reports_missing_fixture() -> Result<()> {
        let fixtures = BTreeSet::new();

        match evidence_fixture_name_for_schema("validation-report-v1.schema.json", &fixtures) {
            Ok(fixture) => Err(anyhow!("missing fixture should fail, got {fixture}")),
            Err(err) if err.to_string().contains("validation-report.json") => Ok(()),
            Err(err) => Err(anyhow!(
                "error should list the legacy fixture candidate, got {err}"
            )),
        }
    }

    #[test]
    fn evidence_schema_targets_cover_supplemental_receipt_fixture() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let targets = evidence_schema_targets(&root)?;

        if targets.iter().any(|target| {
            target
                .data
                .ends_with("safe-analysis-redaction-output-receipt-v2.json")
        }) {
            Ok(())
        } else {
            Err(anyhow!(
                "supplemental receipt fixture should be schema-validated"
            ))
        }
    }

    // ---- Python publish policy -----------------------------------------

    #[test]
    fn python_publish_policy_covers_checked_in_workflows() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();

        ensure_hl7v2_python_not_crates_io_published(&root)?;
        check_python_pyproject_policy(&root)?;
        for policy in PYTHON_PUBLISH_WORKFLOWS {
            check_python_publish_workflow(&root, policy)?;
        }
        Ok(())
    }

    #[test]
    fn python_pyproject_policy_accepts_expected_metadata() -> Result<()> {
        let pyproject = r#"
[build-system]
requires = ["maturin>=1.13.1,<2"]
build-backend = "maturin"

[project]
name = "hl7v2-python"
dynamic = ["version"]
readme = "crates/hl7v2-python/README.md"
requires-python = ">=3.10"
license = { text = "AGPL-3.0-or-later" }

[tool.maturin]
manifest-path = "crates/hl7v2-python/Cargo.toml"
module-name = "hl7v2"
bindings = "pyo3"
"#;

        check_python_pyproject_policy_text(pyproject)
    }

    #[test]
    fn python_pyproject_policy_rejects_wrong_maturin_manifest_path() -> Result<()> {
        let pyproject = r#"
[build-system]
requires = ["maturin>=1.13.1,<2"]
build-backend = "maturin"

[project]
name = "hl7v2-python"
dynamic = ["version"]
readme = "crates/hl7v2-python/README.md"
requires-python = ">=3.10"
license = { text = "AGPL-3.0-or-later" }

[tool.maturin]
manifest-path = "Cargo.toml"
module-name = "hl7v2"
bindings = "pyo3"
"#;

        match check_python_pyproject_policy_text(pyproject) {
            Ok(()) => Err(anyhow!(
                "pyproject policy should reject a maturin manifest outside crates/hl7v2-python"
            )),
            Err(err) if err.to_string().contains("[tool.maturin].manifest-path") => Ok(()),
            Err(err) => Err(anyhow!("unexpected pyproject policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_automatic_workflow_triggers() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            "on:\n  workflow_dispatch:",
            "on:\n  push:\n  workflow_dispatch:",
        );

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject automatic workflow triggers"
            )),
            Err(err)
                if err.to_string().contains("manual-only")
                    && err.to_string().contains("workflow_dispatch") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_missing_local_evidence_guide_smoke() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace("python tests/python_smoke/evidence_workflow_guide.py", "");

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject workflows without local evidence guide smoke"
            )),
            Err(err)
                if err.to_string().contains("local wheel proof step")
                    && err.to_string().contains("evidence_workflow_guide.py") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_skip_existing_uploads() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            "packages-dir: dist/",
            "packages-dir: dist/\n          skip-existing: true",
        );

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject skip-existing uploads"
            )),
            Err(err) if err.to_string().contains("skip-existing") => Ok(()),
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_secret_token_uploads() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            "timeout-minutes: 15\n    environment:",
            "timeout-minutes: 15\n    env:\n      PYPI_API_TOKEN: ${{ secrets.PYPI_API_TOKEN }}\n    environment:",
        );

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject secret-backed upload tokens"
            )),
            Err(err)
                if err.to_string().contains("Trusted Publishing")
                    && err.to_string().contains("PYPI_API_TOKEN") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_oidc_on_non_publish_jobs() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            "    timeout-minutes: 30\n    outputs:",
            "    timeout-minutes: 30\n    permissions:\n      contents: read\n      id-token: write\n    outputs:",
        );

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject OIDC on non-publish jobs"
            )),
            Err(err) if err.to_string().contains("must not set `id-token`") => Ok(()),
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_production_workflow_without_testpypi_proof_input() -> Result<()>
    {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .iter()
            .find(|policy| policy.path == ".github/workflows/python-pypi.yml")
            .ok_or_else(|| anyhow!("expected production PyPI workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            r#"      testpypi_proof_url:
        description: "Successful Python TestPyPI Proof workflow run URL for this package version"
        required: false
        type: string
        default: ""
"#,
            "",
        );

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject production workflow without TestPyPI proof URL input"
            )),
            Err(err) if err.to_string().contains("testpypi_proof_url") => Ok(()),
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_production_workflow_without_preflight_step() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .iter()
            .find(|policy| policy.path == ".github/workflows/python-pypi.yml")
            .ok_or_else(|| anyhow!("expected production PyPI workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            "Validate production PyPI preconditions",
            "Validate production PyPI preconditions removed",
        );

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject production workflow without package-index preflight"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("Validate production PyPI preconditions") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_production_workflow_without_testpypi_job_checks() -> Result<()>
    {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .iter()
            .find(|policy| policy.path == ".github/workflows/python-pypi.yml")
            .ok_or_else(|| anyhow!("expected production PyPI workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace("Install from TestPyPI and smoke", "Install from TestPyPI");

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject production workflow without TestPyPI install-back job verification"
            )),
            Err(err) if err.to_string().contains("Install from TestPyPI and smoke") => Ok(()),
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    // ---- changed gate scope ---------------------------------------------

    #[test]
    fn changed_scope_detects_crate_rust_changes_without_doc_links() {
        let scope =
            changed_scope_from_paths(["crates/hl7v2/src/lib.rs", "crates/hl7v2-cli/src/main.rs"]);

        assert_eq!(
            scope,
            ChangedScope::Crates {
                crates: vec!["hl7v2".to_string(), "hl7v2-cli".to_string()],
                has_markdown: false
            }
        );
    }

    #[test]
    fn changed_scope_marks_crate_markdown_for_doc_link_check() {
        let scope = changed_scope_from_paths(["crates/hl7v2/README.md"]);

        assert_eq!(
            scope,
            ChangedScope::Crates {
                crates: vec!["hl7v2".to_string()],
                has_markdown: true
            }
        );
    }

    #[test]
    fn changed_scope_includes_untracked_git_listing_entries() {
        let scope = changed_scope_from_git_listings("", "crates/hl7v2/README.md\n");

        assert_eq!(
            scope,
            ChangedScope::Crates {
                crates: vec!["hl7v2".to_string()],
                has_markdown: true
            }
        );
    }

    #[test]
    fn changed_scope_promotes_non_crate_files_to_workspace() {
        let scope = changed_scope_from_paths(["docs/CI_PIPELINE.md"]);

        assert_eq!(scope, ChangedScope::Workspace);
    }

    #[test]
    fn changed_scope_reports_none_for_empty_diff() {
        let scope = changed_scope_from_paths(["", "   "]);

        assert_eq!(scope, ChangedScope::None);
    }

    // ---- doc links -------------------------------------------------------

    #[test]
    fn markdown_local_links_extracts_only_local_inline_targets() {
        let markdown = "\
[local](docs/guide.md)
![image](images/logo.png)
[remote](https://example.com/docs/guide.md)
[anchor](#section)
[mail](mailto:team@example.com)
[with title](docs/titled.md \"Title\")
[angle](<docs/has space.md>)
[encoded](docs/space%20file.md#section)
[query](docs/query.md?plain=1)
```markdown
[fenced](docs/fenced.md)
```
[ref]: docs/ref.md
";

        let links = markdown_local_links(markdown);

        assert_eq!(
            links,
            vec![
                MarkdownLocalLink {
                    line: 1,
                    target: "docs/guide.md".to_string()
                },
                MarkdownLocalLink {
                    line: 6,
                    target: "docs/titled.md".to_string()
                },
                MarkdownLocalLink {
                    line: 7,
                    target: "docs/has space.md".to_string()
                },
                MarkdownLocalLink {
                    line: 8,
                    target: "docs/space%20file.md".to_string()
                },
                MarkdownLocalLink {
                    line: 9,
                    target: "docs/query.md".to_string()
                },
                MarkdownLocalLink {
                    line: 13,
                    target: "docs/ref.md".to_string()
                },
            ]
        );
    }

    #[test]
    fn check_doc_links_accepts_existing_relative_and_percent_encoded_targets() -> Result<()> {
        let root = doc_link_temp_root("valid")?;
        fs::create_dir_all(root.join("docs"))?;
        fs::write(
            root.join("README.md"),
            "[ok](docs/ok.md)\n[encoded](docs/space%20file.md#section)\n",
        )?;
        fs::write(root.join("docs/ok.md"), "# OK\n")?;
        fs::write(root.join("docs/space file.md"), "# Encoded\n")?;

        let stats = check_doc_links_at(&root)?;

        let expected = DocLinkCheckStats {
            markdown_files: 3,
            checked_links: 2,
        };
        if stats != expected {
            return Err(anyhow!(
                "expected doc link stats {expected:?}, got {stats:?}"
            ));
        }
        remove_doc_link_temp_root(&root)?;
        Ok(())
    }

    #[test]
    fn check_doc_links_reports_missing_relative_targets() -> Result<()> {
        let root = doc_link_temp_root("missing")?;
        fs::write(root.join("README.md"), "[missing](docs/missing.md)\n")?;

        let err = check_doc_links_at(&root)
            .err()
            .ok_or_else(|| anyhow!("missing doc link should fail"))?;

        if !err
            .to_string()
            .contains("1 Markdown local link(s) point at missing files")
        {
            return Err(anyhow!("unexpected error: {err}"));
        }
        remove_doc_link_temp_root(&root)?;
        Ok(())
    }

    #[test]
    fn check_doc_links_rejects_repo_escape_targets() -> Result<()> {
        let root = doc_link_temp_root("escape")?;
        fs::write(root.join("README.md"), "[escape](../outside.md)\n")?;

        let err = check_doc_links_at(&root)
            .err()
            .ok_or_else(|| anyhow!("escaping doc link should fail"))?;

        if !err
            .to_string()
            .contains("1 Markdown local link(s) point at missing files")
        {
            return Err(anyhow!("unexpected error: {err}"));
        }
        remove_doc_link_temp_root(&root)?;
        Ok(())
    }

    #[test]
    fn check_doc_links_requires_case_exact_repo_targets() -> Result<()> {
        let root = doc_link_temp_root("case")?;
        fs::create_dir_all(root.join("docs"))?;
        fs::write(root.join("README.md"), "[case](Docs/ok.md)\n")?;
        fs::write(root.join("docs/ok.md"), "# OK\n")?;

        let err = check_doc_links_at(&root)
            .err()
            .ok_or_else(|| anyhow!("case-mismatched doc link should fail"))?;

        if !err
            .to_string()
            .contains("1 Markdown local link(s) point at missing files")
        {
            return Err(anyhow!("unexpected error: {err}"));
        }
        remove_doc_link_temp_root(&root)?;
        Ok(())
    }

    #[test]
    fn doc_link_inventory_includes_markdown_sources_and_parent_dirs() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let inventory = doc_link_inventory_from_repo_paths(
            &root,
            [
                "README.md",
                "docs/guides/first.md",
                "docs/guides/assets/logo.svg",
                "target/generated.md",
                "generated/output.md",
                "vendor/README.md",
            ],
        )?;

        let markdown: Vec<String> = inventory
            .markdown_files
            .iter()
            .map(|path| relative_slash_path(&root, path))
            .collect::<Result<_>>()?;

        if markdown != vec!["README.md", "docs/guides/first.md"] {
            return Err(anyhow!("unexpected Markdown inventory: {markdown:?}"));
        }
        for expected in [
            "README.md",
            "docs",
            "docs/guides",
            "docs/guides/first.md",
            "docs/guides/assets",
            "docs/guides/assets/logo.svg",
        ] {
            if !inventory.target_paths.contains(expected) {
                return Err(anyhow!("missing inventory target: {expected}"));
            }
        }
        if inventory.target_paths.contains("target/generated.md") {
            return Err(anyhow!("target directory should be skipped"));
        }
        if inventory.target_paths.contains("generated/output.md")
            || inventory.target_paths.contains("vendor/README.md")
        {
            return Err(anyhow!("generated/vendor directories should be skipped"));
        }
        Ok(())
    }

    fn doc_link_temp_root(name: &str) -> Result<PathBuf> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string();
        let root = env::temp_dir().join(format!(
            "hl7v2-rs-xtask-doc-links-{name}-{}-{nonce}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root)?;
        }
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn remove_doc_link_temp_root(root: &Path) -> Result<()> {
        if root.exists() {
            fs::remove_dir_all(root)?;
        }
        Ok(())
    }

    // ---- glob_match ------------------------------------------------------

    #[test]
    fn glob_star_does_not_cross_slashes() {
        assert!(glob_match("foo/*.rs", "foo/bar.rs"));
        assert!(!glob_match("foo/*.rs", "foo/sub/bar.rs"));
    }

    #[test]
    fn glob_double_star_crosses_slashes() {
        assert!(glob_match("foo/**", "foo/bar.rs"));
        assert!(glob_match("foo/**", "foo/sub/bar.rs"));
        assert!(glob_match("foo/**/baz", "foo/a/b/baz"));
    }

    #[test]
    fn glob_question_matches_single_non_slash() {
        assert!(glob_match("ab?", "abc"));
        assert!(!glob_match("ab?", "ab/"));
        assert!(!glob_match("ab?", "ab"));
    }

    #[test]
    fn glob_literal_match_and_mismatch() {
        assert!(glob_match("Cargo.toml", "Cargo.toml"));
        assert!(!glob_match("Cargo.toml", "cargo.toml"));
    }

    // ---- file-policy auto-allow -----------------------------------------

    #[test]
    fn file_policy_inventory_keeps_untracked_git_listing_entries() {
        let listing = "\
Cargo.toml
docs\\README.md
.github/workflows/python-pypi.yml

";

        assert_eq!(
            file_policy_inventory_from_git_listing(listing),
            vec![
                "Cargo.toml",
                "docs/README.md",
                ".github/workflows/python-pypi.yml"
            ]
        );
    }

    #[test]
    fn auto_allow_covers_rust_and_repo_metadata() {
        for path in [
            "src/lib.rs",
            "Cargo.toml",
            "Cargo.lock",
            "crates/foo/Cargo.toml",
            "README.md",
            "docs/X.md",
            "LICENSE",
            ".gitignore",
            ".gitattributes",
            ".envrc",
        ] {
            assert!(
                file_is_auto_allowed(path),
                "{path} should be auto-allowed without an entry"
            );
        }
    }

    #[test]
    fn auto_allow_does_not_cover_non_rust_programming_surfaces() {
        for path in [
            ".github/workflows/ci.yml",
            "policy/clippy-lints.toml",
            "schemas/message.schema.json",
            "infrastructure/k8s/deployment.yaml",
            "scripts/tests/test.sh",
            "flake.nix",
        ] {
            assert!(
                !file_is_auto_allowed(path),
                "{path} must require a non-rust-allowlist entry"
            );
        }
    }

    // ---- panic-family scanning ------------------------------------------

    fn first_finding(rel: &str, src: &str) -> Option<PanicFinding> {
        let mut out = Vec::new();
        let suppressed = file_level_clippy_suppressions(src);
        scan_panic_in_file(rel, src, &suppressed, &mut out);
        out.into_iter().next()
    }

    #[test]
    fn scanner_detects_unwrap_method_call() {
        let src = "fn parses_msh() {\n    let _ = some.unwrap();\n}\n";
        let finding = first_finding("a.rs", src);
        assert!(finding.is_some(), "unwrap should be detected");
        if let Some(finding) = finding {
            assert_eq!(finding.family.as_str(), "unwrap");
            assert_eq!(finding.family.callee(), "unwrap");
            assert_eq!(finding.family.selector_kind(), "method_call");
            assert_eq!(finding.container.as_deref(), Some("parses_msh"));
            assert_eq!(finding.line, 2);
        }
    }

    #[test]
    fn scanner_detects_panic_macro() {
        let src = "fn boom() {\n    panic!(\"x\");\n}\n";
        let finding = first_finding("a.rs", src);
        assert!(finding.is_some(), "panic! should be detected");
        if let Some(finding) = finding {
            assert_eq!(finding.family.as_str(), "panic_macro");
            assert_eq!(finding.family.selector_kind(), "macro");
        }
    }

    #[test]
    fn scanner_skips_unwrap_inside_string_literal() {
        let src = "fn f() {\n    let _ = \".unwrap()\";\n}\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    #[test]
    fn scanner_skips_unwrap_inside_line_comment() {
        let src = "fn f() {\n    // foo.unwrap();\n}\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    #[test]
    fn scanner_skips_unwrap_inside_block_comment() {
        let src = "fn f() {\n    /* foo.unwrap(); */\n}\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    #[test]
    fn scanner_honors_file_level_expect_attribute() {
        let src = "#![expect(clippy::unwrap_used, reason = \"r\")]\nfn f() { x.unwrap(); }\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    #[test]
    fn scanner_honors_inner_cfg_attr_test_expect() {
        let src = "#![cfg_attr(test, expect(clippy::unwrap_used, reason = \"r\"))]\nfn f() { x.unwrap(); }\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    #[test]
    fn scanner_honors_item_level_expect_attribute() {
        let src = "#[expect(clippy::unwrap_used, reason = \"r\")]\nfn f() { x.unwrap(); }\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    #[test]
    fn scanner_does_not_treat_dotted_unwrap_as_method() {
        // `..unwrap()` is not a real call (won't compile), but nothing should
        // match if the previous char is `.`.
        let src = "fn f() { x..unwrap(); }\n";
        assert!(first_finding("a.rs", src).is_none());
    }

    // ---- allowlist matching ---------------------------------------------

    #[test]
    fn allowlist_entry_matches_when_path_family_and_selector_align() {
        let entry = NoPanicAllowEntry {
            id: "panic-0001".into(),
            path: "crates/x/src/lib.rs".into(),
            family: "unwrap".into(),
            classification: "test_helper".into(),
            owner: "x".into(),
            explanation: "y".into(),
            expires: "2027-01-01".into(),
            selector_kind: "method_call".into(),
            selector_callee: "unwrap".into(),
            selector_container: Some("parse_msh".into()),
        };
        let finding = PanicFinding {
            path: "crates/x/src/lib.rs".into(),
            family: PanicFamily::Unwrap,
            container: Some("parse_msh".into()),
            line: 99,
            column: 99,
        };
        assert!(no_panic_entry_matches_finding(&entry, &finding));
    }

    #[test]
    fn allowlist_entry_with_no_container_matches_any_container() {
        let entry = NoPanicAllowEntry {
            id: "panic-0002".into(),
            path: "crates/x/src/lib.rs".into(),
            family: "unwrap".into(),
            classification: "test_helper".into(),
            owner: "x".into(),
            explanation: "y".into(),
            expires: "2027-01-01".into(),
            selector_kind: "method_call".into(),
            selector_callee: "unwrap".into(),
            selector_container: None,
        };
        let finding = PanicFinding {
            path: "crates/x/src/lib.rs".into(),
            family: PanicFamily::Unwrap,
            container: Some("anything".into()),
            line: 1,
            column: 1,
        };
        assert!(no_panic_entry_matches_finding(&entry, &finding));
    }

    #[test]
    fn allowlist_entry_does_not_match_different_family() {
        let entry = NoPanicAllowEntry {
            id: "panic-0003".into(),
            path: "crates/x/src/lib.rs".into(),
            family: "unwrap".into(),
            classification: "test_helper".into(),
            owner: "x".into(),
            explanation: "y".into(),
            expires: "2027-01-01".into(),
            selector_kind: "method_call".into(),
            selector_callee: "unwrap".into(),
            selector_container: None,
        };
        let finding = PanicFinding {
            path: "crates/x/src/lib.rs".into(),
            family: PanicFamily::Expect,
            container: None,
            line: 1,
            column: 1,
        };
        assert!(!no_panic_entry_matches_finding(&entry, &finding));
    }
}
