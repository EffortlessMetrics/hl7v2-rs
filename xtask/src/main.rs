//! Workspace task runner for repository automation and release checks.

mod cli;
mod publish;
mod verification_surface;

use anyhow::{Result, anyhow};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use clap::Parser;
use cli::{Cli, Commands, NoPanicAction, PythonPackageIndex};
use publish::package_is_publishable;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

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
        Commands::PublishPlan { from, surface } => publish::publish_plan(from, surface)?,
        Commands::Publish {
            from,
            yes,
            retry_attempts,
            retry_delay_secs,
        } => publish::publish(from, yes, retry_attempts, retry_delay_secs)?,
        Commands::PublishDryRun {
            from,
            surface,
            workspace_patches,
            allow_dirty,
        } => publish::publish_dry_run(from, surface, workspace_patches, allow_dirty)?,
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
            NoPanicAction::Baseline { reset } => no_panic_baseline(reset)?,
        },
        Commands::CheckFilePolicy => check_file_policy()?,
        Commands::CheckDocLinks => check_doc_links()?,
        Commands::CheckPythonPublishPolicy => check_python_publish_policy()?,
        Commands::PythonLocalWheelProof {
            root,
            python,
            rust_toolchain,
            keep_existing,
        } => python_local_wheel_proof(root, &python, &rust_toolchain, keep_existing)?,
        Commands::PythonPublicRegistryProof {
            root,
            python,
            index,
            version,
            keep_existing,
        } => python_public_registry_proof(root, &python, index, version, keep_existing)?,
        Commands::CheckCiLaneWhitelist => check_ci_lane_whitelist()?,
        Commands::CheckEvidenceParity => check_evidence_parity()?,
        Commands::CheckEvidenceParityAcceptance { include_python } => {
            check_evidence_parity_acceptance(include_python)?;
        }
        Commands::CheckFirstUseGuides {
            include_python,
            include_public_crates,
        } => check_first_use_guides(include_python, include_public_crates)?,
        Commands::CheckFirst10MinutesGuide => check_first_10_minutes_guide()?,
        Commands::CheckFirstUseBySurfaceGuide => check_first_use_by_surface_guide()?,
        Commands::CheckVendorUpgradeDiffGuide => check_vendor_upgrade_diff_guide()?,
        Commands::CheckOperatorErrorGuidanceGuide => check_operator_error_guidance_guide()?,
        Commands::CheckSafeSupportBundleGuide => check_safe_support_bundle_guide()?,
        Commands::CheckEvidenceArtifactsGuide => check_evidence_artifacts_guide()?,
        Commands::CheckSidecarGuide => check_sidecar_guide()?,
        Commands::CheckSafeErrorPhiParity { include_python } => {
            check_safe_error_phi_parity(include_python)?;
        }
        Commands::CheckProfileParity { include_python } => {
            check_profile_parity(include_python)?;
        }
        Commands::CheckSchemaVersionParity { include_python } => {
            check_schema_version_parity(include_python)?;
        }
        Commands::CheckDirtyCorpusParity { include_python } => {
            check_dirty_corpus_parity(include_python)?;
        }
        Commands::CheckBundleReplayParity { include_python } => {
            check_bundle_replay_parity(include_python)?;
        }
        Commands::EvidenceSchemaCheck => evidence_schema_check()?,
        Commands::Badges { check } => verification_surface::badges(check)?,
        Commands::RiprPr {
            root,
            base,
            head,
            check,
        } => verification_surface::ripr_pr(&root, &base, &head, check)?,
        Commands::RiprReviewComments {
            root,
            base,
            head,
            check,
        } => verification_surface::ripr_review_comments(&root, &base, &head, check)?,
        Commands::RiprPrSummary { check } => verification_surface::ripr_pr_summary(check)?,
        Commands::RiprAnnotations {
            comments,
            out,
            check,
        } => verification_surface::ripr_annotations(&comments, &out, check)?,
        Commands::ImpactedEvidence {
            pr_evidence,
            label,
            labels,
            check,
        } => {
            verification_surface::impacted_evidence(&pr_evidence, &label, labels.as_deref(), check)?
        }
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
        println!("Checking evidence parity manifest...");
        check_evidence_parity()?;
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

    let tools = [
        ("cargo-deny", "cargo install cargo-deny"),
        ("cargo-audit", "cargo install cargo-audit"),
        ("cargo-nextest", "cargo install cargo-nextest"),
        ("just", "cargo install just"),
    ];
    let missing_tools: Vec<_> = tools
        .iter()
        .filter(|(tool, _)| !command_exists(tool))
        .collect();
    if !missing_tools.is_empty() {
        println!("Optional development tools are missing:");
        for (tool, install_command) in missing_tools {
            println!("  - {tool}: install with `{install_command}` or run `nix develop`");
        }
        println!(
            "The repository hooks are configured; missing optional tools only limit local DevEx commands."
        );
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

fn check_lint_policy() -> Result<()> {
    println!("🔎 Checking lint policy...");

    let root = env::current_dir()?;
    let cargo_toml = root.join("Cargo.toml");
    let policy_lints = root.join("policy/clippy-lints.toml");
    let policy_debt = root.join("policy/clippy-debt.toml");
    let policy_exceptions = root.join("policy/clippy-exceptions.toml");
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
    ensure_clippy_exceptions(&policy_exceptions)?;

    println!("✅ Lint policy checks passed!");
    Ok(())
}

fn policy_report() -> Result<()> {
    let root = env::current_dir()?;
    let cargo_text = fs::read_to_string(root.join("Cargo.toml"))?;
    let policy_text = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let debt_text = fs::read_to_string(root.join("policy/clippy-debt.toml"))?;
    let exceptions_text = fs::read_to_string(root.join("policy/clippy-exceptions.toml"))?;

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
    let exception_count = parse_clippy_exceptions(&exceptions_text)?.len();

    let no_panic_text = fs::read_to_string(root.join("policy/no-panic-allowlist.toml"))?;
    let no_panic_entries = parse_no_panic_allowlist(&no_panic_text)?;
    let no_panic_baseline_text = fs::read_to_string(root.join("policy/no-panic-baseline.toml"))?;
    let no_panic_baseline_mode = no_panic_baseline_mode(&no_panic_baseline_text)?;
    let parsed_no_panic_baseline_entries = parse_no_panic_baseline(&no_panic_baseline_text)?;
    let no_panic_baseline_entries = effective_no_panic_baseline_entries(
        &no_panic_baseline_mode,
        &parsed_no_panic_baseline_entries,
    );
    let file_policy_text = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;
    let file_policy_entries = parse_file_policy_allowlist(&file_policy_text)?;
    let companion_policy_summary = check_companion_policy_ledgers(&root)?;

    let metadata = MetadataCommand::new().current_dir(&root).exec()?;
    let strict_units = collect_rust_files_for(&root, &metadata, &required_packages)?;
    let advisory_units = collect_rust_files_for(&root, &metadata, &staged_packages)?;
    let strict_findings = scan_panic_family(&root, &strict_units)?;
    let advisory_findings = scan_panic_family(&root, &advisory_units)?;
    let all_no_panic_findings = combined_no_panic_findings(&strict_findings, &advisory_findings);
    let no_panic_unreceipted =
        match_findings_against_allowlist(&all_no_panic_findings, &no_panic_entries);
    let no_panic_new_debt =
        match_findings_against_baseline(&no_panic_unreceipted, &no_panic_baseline_entries);
    let no_panic_stale_allowlist =
        stale_no_panic_entries(&no_panic_entries, &all_no_panic_findings);
    let no_panic_stale_baseline =
        stale_no_panic_baseline_entries(&no_panic_baseline_entries, &no_panic_unreceipted);
    write_no_panic_report(
        &root,
        &NoPanicReport {
            baseline_mode: no_panic_baseline_mode.clone(),
            baseline_ignored: no_panic_baseline_mode == "blocking",
            allowlist_entries: no_panic_entries.len(),
            baseline_entries: parsed_no_panic_baseline_entries.len(),
            baseline_occurrences: no_panic_baseline_occurrences(&parsed_no_panic_baseline_entries),
            strict_findings: strict_findings.len(),
            advisory_findings: advisory_findings.len(),
            new_debt: no_panic_new_debt,
            stale_allowlist: no_panic_stale_allowlist
                .iter()
                .map(|entry| (*entry).clone())
                .collect(),
            stale_baseline: no_panic_stale_baseline.clone(),
        },
    )?;

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
    println!("  Retained exceptions: {exception_count}");
    println!();
    println!("No-panic policy");
    println!("  Allowlist entries: {}", no_panic_entries.len());
    println!("  Baseline mode: {no_panic_baseline_mode}");
    println!(
        "  Baseline entries: {}",
        parsed_no_panic_baseline_entries.len()
    );
    println!(
        "  Baseline occurrences: {}",
        no_panic_baseline_occurrences(&parsed_no_panic_baseline_entries)
    );
    println!(
        "  Strict findings (required-inheriting crates): {}",
        strict_findings.len()
    );
    println!(
        "  Advisory findings (staged crates):           {}",
        advisory_findings.len()
    );
    println!(
        "  Stale baseline entries:                      {}",
        no_panic_stale_baseline.len()
    );
    println!("  Report: target/policy/no-panic-report.md");
    println!("  Report JSON: target/policy/no-panic-report.json");
    println!();
    println!("File policy");
    println!(
        "  Non-Rust allowlist entries: {}",
        file_policy_entries.len()
    );
    println!(
        "  Companion ledgers: {} ledger(s), {} allow entr(ies)",
        companion_policy_summary.ledgers, companion_policy_summary.entries
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

fn ensure_clippy_exceptions(policy_exceptions: &Path) -> Result<()> {
    let text = fs::read_to_string(policy_exceptions)?;
    for (key, expected) in [
        ("schema_version", "1.0"),
        ("policy", "clippy-exceptions"),
        ("owner", "EffortlessMetrics"),
        ("status", "active"),
    ] {
        let actual = top_level_quoted_value(&text, key)
            .ok_or_else(|| anyhow!("policy/clippy-exceptions.toml is missing {key}"))?;
        if actual != expected {
            return Err(anyhow!(
                "policy/clippy-exceptions.toml {key} must be {expected}, found {actual}"
            ));
        }
    }
    parse_clippy_exceptions(&text)?;
    Ok(())
}

fn parse_clippy_exceptions(text: &str) -> Result<Vec<String>> {
    let mut ids = BTreeSet::new();
    let mut parsed = Vec::new();
    for (index, entry) in table_array_entries(text, "[[exception]]")
        .iter()
        .enumerate()
    {
        let entry_number = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("clippy exception entry index overflow"))?;
        for key in [
            "id",
            "lint",
            "path",
            "selector",
            "owner",
            "reason",
            "covered_by",
            "expires",
        ] {
            if top_level_quoted_value(entry, key).is_none() {
                return Err(anyhow!(
                    "policy/clippy-exceptions.toml exception entry {entry_number} is missing required field `{key}`"
                ));
            }
        }

        let id = top_level_quoted_value(entry, "id").ok_or_else(|| {
            anyhow!("policy/clippy-exceptions.toml exception entry {entry_number} is missing id")
        })?;
        if !ids.insert(id.clone()) {
            return Err(anyhow!(
                "policy/clippy-exceptions.toml duplicate exception id `{id}`"
            ));
        }

        let expires = top_level_quoted_value(entry, "expires").ok_or_else(|| {
            anyhow!(
                "policy/clippy-exceptions.toml exception entry {entry_number} is missing expires"
            )
        })?;
        if expires.as_str() < "2026-05-06" {
            return Err(anyhow!(
                "policy/clippy-exceptions.toml exception entry {entry_number} expired on {expires}"
            ));
        }

        parsed.push(id);
    }
    Ok(parsed)
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

pub(crate) fn escape_toml_basic_string(value: &str) -> String {
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

fn run_command_owned(cmd: &str, args: &[String]) -> Result<()> {
    run_command_owned_allow_codes(cmd, args, &[0])
}

fn run_command_owned_allow_codes(cmd: &str, args: &[String], allowed_codes: &[i32]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    let code = status.code();
    if !matches!(code, Some(code) if allowed_codes.contains(&code)) {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}",
            cmd,
            args.join(" "),
            code
        ));
    }

    Ok(())
}

fn run_command_capture_owned(cmd: &str, args: &[String]) -> Result<String> {
    let output = Command::new(cmd).args(args).output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}\nstdout:\n{}\nstderr:\n{}",
            cmd,
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn run_command_with_env_in_dir(
    cmd: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
    cwd: Option<&Path>,
) -> Result<()> {
    let mut command = Command::new(cmd);
    command
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for (key, value) in envs {
        command.env(key, value);
    }

    let status = command.status()?;

    if !status.success() {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}",
            cmd.display(),
            args.join(" "),
            status.code()
        ));
    }

    Ok(())
}

fn run_program_with_env_in_dir(
    cmd: &str,
    args: &[&str],
    envs: &[(&str, &str)],
    cwd: Option<&Path>,
) -> Result<()> {
    let mut command = Command::new(cmd);
    command
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for (key, value) in envs {
        command.env(key, value);
    }

    let status = command.status()?;

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
    std::cfg_select! {
        windows => {
        Command::new("where")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        }
        _ => {
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
            "ajv-cli@5.0.0",
            "-p",
            "ajv-formats@3.0.1",
            "ajv",
            "validate",
        ]);
        command
    } else {
        return Err(anyhow!(
            "evidence schema check requires ajv-cli 5.0.0 and ajv-formats 3.0.1; install with `npm install -g ajv-cli@5.0.0 ajv-formats@3.0.1` or make npx available"
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
    std::cfg_select! {
        windows => {
            format!("{cmd}.cmd")
        }
        _ => {
            cmd.to_string()
        }
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
// `policy/no-panic-allowlist.toml`. Identity is
// `path + family + selector_kind + selector_callee + snippet`, with `count`
// consumed per occurrence. `container` and `last_seen.{line,column}` are
// advisory locators.
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
    snippet: String,
    line: usize,
    column: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NoPanicIdentity {
    path: String,
    family: String,
    selector_kind: String,
    selector_callee: String,
    snippet: String,
}

impl PanicFinding {
    fn identity(&self) -> NoPanicIdentity {
        NoPanicIdentity {
            path: self.path.clone(),
            family: self.family.as_str().to_string(),
            selector_kind: self.family.selector_kind().to_string(),
            selector_callee: self.family.callee().to_string(),
            snippet: self.snippet.clone(),
        }
    }
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
    snippet: String,
    count: usize,
    selector_kind: String,
    selector_callee: String,
    selector_container: Option<String>,
}

impl NoPanicAllowEntry {
    fn identity(&self) -> NoPanicIdentity {
        NoPanicIdentity {
            path: self.path.clone(),
            family: self.family.clone(),
            selector_kind: self.selector_kind.clone(),
            selector_callee: self.selector_callee.clone(),
            snippet: self.snippet.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct NoPanicBaselineEntry {
    path: String,
    family: String,
    snippet: String,
    count: usize,
    selector_kind: String,
    selector_callee: String,
    selector_container: Option<String>,
    last_seen_line: usize,
    last_seen_column: usize,
}

impl NoPanicBaselineEntry {
    fn identity(&self) -> NoPanicIdentity {
        NoPanicIdentity {
            path: self.path.clone(),
            family: self.family.clone(),
            selector_kind: self.selector_kind.clone(),
            selector_callee: self.selector_callee.clone(),
            snippet: self.snippet.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct NoPanicBaselineDelta {
    path: String,
    family: String,
    selector_kind: String,
    selector_callee: String,
    selector_container: Option<String>,
    snippet: String,
    baseline_count: usize,
    current_count: usize,
    last_seen_line: usize,
    last_seen_column: usize,
}

impl NoPanicBaselineDelta {
    fn from_entry(
        entry: &NoPanicBaselineEntry,
        baseline_count: usize,
        current_count: usize,
    ) -> Self {
        Self {
            path: entry.path.clone(),
            family: entry.family.clone(),
            selector_kind: entry.selector_kind.clone(),
            selector_callee: entry.selector_callee.clone(),
            selector_container: entry.selector_container.clone(),
            snippet: entry.snippet.clone(),
            baseline_count,
            current_count,
            last_seen_line: entry.last_seen_line,
            last_seen_column: entry.last_seen_column,
        }
    }

    fn surplus_count(&self) -> usize {
        self.baseline_count.saturating_sub(self.current_count)
    }

    fn new_debt_count(&self) -> usize {
        self.current_count.saturating_sub(self.baseline_count)
    }
}

struct NoPanicReport {
    baseline_mode: String,
    baseline_ignored: bool,
    allowlist_entries: usize,
    baseline_entries: usize,
    baseline_occurrences: usize,
    strict_findings: usize,
    advisory_findings: usize,
    new_debt: Vec<PanicFinding>,
    stale_allowlist: Vec<NoPanicAllowEntry>,
    stale_baseline: Vec<NoPanicBaselineDelta>,
}

const NO_PANIC_CLASSIFICATIONS: &[&str] = &[
    "production",
    "test_helper",
    "generated",
    "fixture",
    "external_api",
];

const NO_PANIC_SELECTOR_KINDS: &[&str] = &["method_call", "macro", "indexing"];
const NO_PANIC_REPORT_STALE_LIMIT: usize = 50;

fn check_no_panic_family(include_staged_in_strict: bool) -> Result<()> {
    println!("🔎 Checking no-panic-family policy...");
    let root = env::current_dir()?;
    let policy_text = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let allowlist_text = fs::read_to_string(root.join("policy/no-panic-allowlist.toml"))?;
    let baseline_text = fs::read_to_string(root.join("policy/no-panic-baseline.toml"))
        .map_err(missing_no_panic_baseline_error)?;

    let entries = parse_no_panic_allowlist(&allowlist_text)?;
    let baseline_mode = no_panic_baseline_mode(&baseline_text)?;
    let parsed_baseline_entries = parse_no_panic_baseline(&baseline_text)?;
    if let Some(message) =
        no_panic_blocking_mode_message(&baseline_mode, parsed_baseline_entries.len())
    {
        eprintln!("{message}");
    }
    let baseline_entries =
        effective_no_panic_baseline_entries(&baseline_mode, &parsed_baseline_entries);
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
    let mut strict_files = collect_rust_files_for(&root, &metadata, &required)?;
    let advisory_files = if include_staged_in_strict {
        strict_files.extend(collect_rust_files_for(&root, &metadata, &staged)?);
        Vec::new()
    } else {
        collect_rust_files_for(&root, &metadata, &staged)?
    };

    let strict_findings = scan_panic_family(&root, &strict_files)?;
    let advisory_findings = scan_panic_family(&root, &advisory_files)?;
    let all_findings = combined_no_panic_findings(&strict_findings, &advisory_findings);

    let unreceipted = match_findings_against_allowlist(&all_findings, &entries);
    let unmatched = match_findings_against_baseline(&unreceipted, &baseline_entries);
    let stale_baseline = stale_no_panic_baseline_entries(&baseline_entries, &unreceipted);
    let stale = stale_no_panic_entries(&entries, &all_findings);
    write_no_panic_report(
        &root,
        &NoPanicReport {
            baseline_mode: baseline_mode.clone(),
            baseline_ignored: baseline_mode == "blocking",
            allowlist_entries: entries.len(),
            baseline_entries: parsed_baseline_entries.len(),
            baseline_occurrences: no_panic_baseline_occurrences(&parsed_baseline_entries),
            strict_findings: strict_findings.len(),
            advisory_findings: advisory_findings.len(),
            new_debt: unmatched.clone(),
            stale_allowlist: stale.iter().map(|entry| (*entry).clone()).collect(),
            stale_baseline: stale_baseline.clone(),
        },
    )?;
    if !unmatched.is_empty() {
        for f in unmatched.iter().take(20) {
            eprintln!(
                "no-panic: {}:{}:{}: new {} debt ({} {})",
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
            "{} panic-family finding(s) are outside policy/no-panic-allowlist.toml and the no-new-debt baseline; \
             remove the call or refresh the baseline with --reset only in the dedicated baseline PR",
            unmatched.len()
        ));
    }

    if !stale.is_empty() {
        for entry in stale.iter().take(NO_PANIC_REPORT_STALE_LIMIT) {
            eprintln!(
                "no-panic: stale entry id={} path={} family={} (no matching finding)",
                entry.id, entry.path, entry.family
            );
        }
        if stale.len() > NO_PANIC_REPORT_STALE_LIMIT {
            eprintln!(
                "no-panic: ... and {} more stale allowlist entr(ies)",
                stale.len().saturating_sub(NO_PANIC_REPORT_STALE_LIMIT)
            );
        }
        return Err(anyhow!(
            "{} stale no-panic-allowlist entr(ies); remove or update them",
            stale.len()
        ));
    }

    if !stale_baseline.is_empty() {
        eprintln!(
            "no-panic: {} stale/surplus baseline entr(ies); run `cargo run -p xtask -- no-panic baseline` to drop or reduce them",
            stale_baseline.len()
        );
        for entry in stale_baseline.iter().take(NO_PANIC_REPORT_STALE_LIMIT) {
            eprintln!(
                "no-panic: stale baseline path={} family={} selector={} baseline={} current={} surplus={} snippet={}",
                entry.path,
                entry.family,
                entry.selector_callee,
                entry.baseline_count,
                entry.current_count,
                entry.surplus_count(),
                entry.snippet
            );
        }
        if stale_baseline.len() > NO_PANIC_REPORT_STALE_LIMIT {
            eprintln!(
                "no-panic: ... and {} more stale baseline entr(ies)",
                stale_baseline
                    .len()
                    .saturating_sub(NO_PANIC_REPORT_STALE_LIMIT)
            );
        }
    }

    println!(
        "✅ no-panic policy: {} required-inheriting source file(s) scanned, \
          {} allowlist entr(ies), {} baseline entr(ies), {} baseline occurrence(s), \
          {} advisory finding(s) in staged crates, {} stale baseline entr(ies)",
        total_files(&strict_files),
        entries.len(),
        parsed_baseline_entries.len(),
        no_panic_baseline_occurrences(&parsed_baseline_entries),
        advisory_findings.len(),
        stale_baseline.len(),
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

    // Group by exact allowlist identity. `count` receipts repeated occurrences
    // of the same snippet without letting one entry cover different code.
    let mut grouped: BTreeMap<NoPanicIdentity, (&PanicFinding, usize)> = BTreeMap::new();
    for finding in &findings {
        grouped
            .entry(finding.identity())
            .and_modify(|(_, count)| *count = count.saturating_add(1))
            .or_insert((finding, 1));
    }

    let report_dir = root.join("target/policy");
    fs::create_dir_all(&report_dir)?;
    let report_path = report_dir.join("no-panic-proposed-allowlist.toml");

    let mut out = String::new();
    out.push_str("schema_version = \"0.4\"\n\n");
    out.push_str("# Proposed allowlist entries generated by `xtask no-panic propose`.\n");
    out.push_str("# Review each entry, set owner/classification/explanation/expires, then\n");
    out.push_str("# copy into policy/no-panic-allowlist.toml.\n\n");

    for (index, (_identity, (finding, count))) in grouped.iter().enumerate() {
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
        out.push_str(&format!(
            "snippet = \"{}\"\n",
            escape_toml_basic_string(&finding.snippet),
        ));
        out.push_str(&format!("count = {count}\n"));
        out.push_str("classification = \"FILL_ME_IN\"\n");
        out.push_str("owner = \"FILL_ME_IN\"\n");
        out.push_str("explanation = \"FILL_ME_IN\"\n");
        out.push_str("expires = \"FILL_ME_IN\"\n");
        out.push_str("\n[allow.selector]\n");
        out.push_str(&format!("kind = \"{}\"\n", finding.family.selector_kind()));
        out.push_str(&format!("callee = \"{}\"\n", finding.family.callee()));
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
        "wrote {} proposed entr(ies) ({} raw findings grouped to {} exact identities) to {}",
        grouped.len(),
        findings.len(),
        grouped.len(),
        report_path.display()
    );
    Ok(())
}

fn no_panic_baseline(reset: bool) -> Result<()> {
    let root = env::current_dir()?;
    let policy_text = fs::read_to_string(root.join("policy/clippy-lints.toml"))?;
    let metadata = MetadataCommand::new().current_dir(&root).exec()?;

    let mut packages =
        string_array_after(&policy_text, "[rollout]", "required_inheriting_packages")
            .ok_or_else(|| anyhow!("policy is missing required_inheriting_packages"))?;
    let staged = string_array_after(&policy_text, "[rollout]", "staged_inheriting_packages")
        .ok_or_else(|| anyhow!("policy is missing staged_inheriting_packages"))?;
    packages.extend(staged);

    let files = collect_rust_files_for(&root, &metadata, &packages)?;
    let findings = scan_panic_family(&root, &files)?;
    let current = no_panic_baseline_entries_from_findings(&findings);

    let baseline_path = root.join("policy/no-panic-baseline.toml");
    let entries_to_write = if reset {
        current
    } else {
        let existing_text =
            fs::read_to_string(&baseline_path).map_err(missing_no_panic_baseline_error)?;
        let existing_mode = no_panic_baseline_mode(&existing_text)?;
        let existing = parse_no_panic_baseline(&existing_text)?;
        if let Some(message) = no_panic_blocking_mode_message(&existing_mode, existing.len()) {
            eprintln!("{message}");
        }
        let existing = effective_no_panic_baseline_entries(&existing_mode, &existing);
        refresh_no_panic_baseline_entries(&current, &existing, false)?
    };

    let rendered = render_no_panic_baseline(&entries_to_write);
    fs::write(&baseline_path, rendered)?;
    let written_text = fs::read_to_string(&baseline_path)?;
    let written = parse_no_panic_baseline(&written_text)?;
    println!(
        "wrote {} no-panic baseline entr(ies) covering {} occurrence(s) to {}",
        written.len(),
        no_panic_baseline_occurrences(&written),
        baseline_path.display()
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
                    snippet: panic_finding_snippet(line),
                    line: line_no,
                    column,
                });
                start = abs.saturating_add(1);
            }
        }
    }
}

fn panic_finding_snippet(line: &str) -> String {
    line.trim().to_string()
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
    let schema_version = top_level_quoted_value(text, "schema_version")
        .ok_or_else(|| anyhow!("policy/no-panic-allowlist.toml is missing `schema_version`"))?;
    if schema_version != "0.4" {
        return Err(anyhow!(
            "policy/no-panic-allowlist.toml schema_version must be `0.4`, found `{schema_version}`"
        ));
    }

    let entries = table_array_entries(text, "[[allow]]");
    let mut parsed = Vec::with_capacity(entries.len());
    let mut identities: BTreeMap<NoPanicIdentity, String> = BTreeMap::new();
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
        let snippet = top_level_quoted_value(entry, "snippet").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing `snippet`")
        })?;
        let count = top_level_usize_value(entry, "count").ok_or_else(|| {
            anyhow!("policy/no-panic-allowlist.toml entry {id} is missing numeric `count`")
        })?;
        if count == 0 {
            return Err(anyhow!(
                "policy/no-panic-allowlist.toml entry {id} must set `count` greater than zero"
            ));
        }

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

        let parsed_entry = NoPanicAllowEntry {
            id,
            path,
            family,
            classification,
            owner,
            explanation,
            expires,
            snippet,
            count,
            selector_kind,
            selector_callee,
            selector_container,
        };
        let identity = parsed_entry.identity();
        if let Some(existing_id) = identities.insert(identity, parsed_entry.id.clone()) {
            return Err(anyhow!(
                "policy/no-panic-allowlist.toml entry {} duplicates exact identity already used by {}",
                parsed_entry.id,
                existing_id
            ));
        }
        parsed.push(parsed_entry);
    }
    Ok(parsed)
}

fn missing_no_panic_baseline_error(err: std::io::Error) -> anyhow::Error {
    anyhow!(
        "missing no-panic no-new-debt baseline: failed to read policy/no-panic-baseline.toml: {err}\n\
         Create it only in the dedicated baseline PR with:\n\
         cargo run -p xtask -- no-panic baseline --reset\n\
         Normal PRs must remove new panic-family code or add a reviewed allowlist receipt; they must not reset the baseline."
    )
}

fn no_panic_baseline_mode(text: &str) -> Result<String> {
    let mode = top_level_quoted_value(text, "mode")
        .ok_or_else(|| anyhow!("policy/no-panic-baseline.toml is missing `mode`"))?;
    if mode != "no-new-debt" && mode != "blocking" {
        return Err(anyhow!(
            "policy/no-panic-baseline.toml mode must be `no-new-debt` or `blocking`, found `{mode}`"
        ));
    }
    Ok(mode)
}

fn no_panic_blocking_mode_message(mode: &str, ignored_entries: usize) -> Option<String> {
    if mode == "blocking" {
        Some(format!(
            "no-panic: policy/no-panic-baseline.toml mode is `blocking`; ignoring {ignored_entries} baseline entr(ies), so every unallowlisted panic-family finding blocks the check"
        ))
    } else {
        None
    }
}

fn effective_no_panic_baseline_entries(
    mode: &str,
    entries: &[NoPanicBaselineEntry],
) -> Vec<NoPanicBaselineEntry> {
    if mode == "blocking" {
        Vec::new()
    } else {
        entries.to_vec()
    }
}

fn parse_no_panic_baseline(text: &str) -> Result<Vec<NoPanicBaselineEntry>> {
    let schema_version = top_level_quoted_value(text, "schema_version")
        .ok_or_else(|| anyhow!("policy/no-panic-baseline.toml is missing `schema_version`"))?;
    if schema_version != "1.0" {
        return Err(anyhow!(
            "policy/no-panic-baseline.toml schema_version must be `1.0`, found `{schema_version}`"
        ));
    }
    let policy = top_level_quoted_value(text, "policy")
        .ok_or_else(|| anyhow!("policy/no-panic-baseline.toml is missing `policy`"))?;
    if policy != "no-panic-baseline" {
        return Err(anyhow!(
            "policy/no-panic-baseline.toml policy must be `no-panic-baseline`, found `{policy}`"
        ));
    }
    let _mode = no_panic_baseline_mode(text)?;

    let entries = table_array_entries(text, "[[baseline]]");
    let mut parsed = Vec::with_capacity(entries.len());
    let mut identities: BTreeMap<NoPanicIdentity, usize> = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry_no = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("baseline index overflow"))?;
        let path = top_level_quoted_value(entry, "path").ok_or_else(|| {
            anyhow!("policy/no-panic-baseline.toml entry {entry_no} is missing `path`")
        })?;
        let family = top_level_quoted_value(entry, "family").ok_or_else(|| {
            anyhow!("policy/no-panic-baseline.toml entry {entry_no} is missing `family`")
        })?;
        if PanicFamily::from_str(&family).is_none() {
            return Err(anyhow!(
                "policy/no-panic-baseline.toml entry {entry_no} has unknown family `{family}`"
            ));
        }
        let snippet = top_level_quoted_value(entry, "snippet").ok_or_else(|| {
            anyhow!("policy/no-panic-baseline.toml entry {entry_no} is missing `snippet`")
        })?;
        let count = top_level_usize_value(entry, "count").ok_or_else(|| {
            anyhow!("policy/no-panic-baseline.toml entry {entry_no} is missing numeric `count`")
        })?;
        if count == 0 {
            return Err(anyhow!(
                "policy/no-panic-baseline.toml entry {entry_no} must set `count` greater than zero"
            ));
        }
        let selector_kind =
            sub_table_value(entry, "[baseline.selector]", "kind").ok_or_else(|| {
                anyhow!(
                    "policy/no-panic-baseline.toml entry {entry_no} is missing `[baseline.selector] kind`"
                )
            })?;
        if !NO_PANIC_SELECTOR_KINDS.contains(&selector_kind.as_str()) {
            return Err(anyhow!(
                "policy/no-panic-baseline.toml entry {entry_no} has unknown selector kind `{selector_kind}`"
            ));
        }
        let selector_callee =
            sub_table_value(entry, "[baseline.selector]", "callee").ok_or_else(|| {
                anyhow!(
                    "policy/no-panic-baseline.toml entry {entry_no} is missing `[baseline.selector] callee`"
                )
            })?;
        let selector_container = sub_table_value(entry, "[baseline.selector]", "container");
        let last_seen_line =
            sub_table_usize_value(entry, "[baseline.last_seen]", "line").ok_or_else(|| {
                anyhow!(
                    "policy/no-panic-baseline.toml entry {entry_no} is missing `[baseline.last_seen] line`"
                )
            })?;
        let last_seen_column =
            sub_table_usize_value(entry, "[baseline.last_seen]", "column").ok_or_else(|| {
                anyhow!(
                    "policy/no-panic-baseline.toml entry {entry_no} is missing `[baseline.last_seen] column`"
                )
            })?;

        let parsed_entry = NoPanicBaselineEntry {
            path,
            family,
            snippet,
            count,
            selector_kind,
            selector_callee,
            selector_container,
            last_seen_line,
            last_seen_column,
        };
        let identity = parsed_entry.identity();
        if let Some(existing_entry_no) = identities.insert(identity, entry_no) {
            return Err(anyhow!(
                "policy/no-panic-baseline.toml entry {entry_no} duplicates exact identity already used by entry {existing_entry_no}"
            ));
        }
        parsed.push(parsed_entry);
    }
    Ok(parsed)
}

fn no_panic_baseline_entries_from_findings(findings: &[PanicFinding]) -> Vec<NoPanicBaselineEntry> {
    let mut grouped: BTreeMap<NoPanicIdentity, (PanicFinding, usize)> = BTreeMap::new();
    for finding in findings {
        grouped
            .entry(finding.identity())
            .and_modify(|(_, count)| *count = count.saturating_add(1))
            .or_insert((finding.clone(), 1));
    }

    grouped
        .into_values()
        .map(|(finding, count)| NoPanicBaselineEntry {
            path: finding.path,
            family: finding.family.as_str().to_string(),
            snippet: finding.snippet,
            count,
            selector_kind: finding.family.selector_kind().to_string(),
            selector_callee: finding.family.callee().to_string(),
            selector_container: finding.container,
            last_seen_line: finding.line,
            last_seen_column: finding.column,
        })
        .collect()
}

fn refresh_no_panic_baseline_entries(
    current: &[NoPanicBaselineEntry],
    existing: &[NoPanicBaselineEntry],
    reset: bool,
) -> Result<Vec<NoPanicBaselineEntry>> {
    if reset {
        return Ok(current.to_vec());
    }

    let existing_counts = no_panic_baseline_counts(existing);
    let mut new_debt = Vec::new();
    for entry in current {
        let allowed = existing_counts
            .get(&entry.identity())
            .copied()
            .unwrap_or_default();
        if entry.count > allowed {
            new_debt.push(NoPanicBaselineDelta::from_entry(
                entry,
                allowed,
                entry.count,
            ));
        }
    }
    if !new_debt.is_empty() {
        for entry in new_debt.iter().take(20) {
            eprintln!(
                "no-panic baseline: new debt {} {} {} current={} baseline={} delta={} snippet={}",
                entry.path,
                entry.family,
                entry.selector_callee,
                entry.current_count,
                entry.baseline_count,
                entry.new_debt_count(),
                entry.snippet
            );
        }
        if new_debt.len() > 20 {
            eprintln!(
                "no-panic baseline: ... and {} more new baseline entr(ies)",
                new_debt.len().saturating_sub(20)
            );
        }
        let first_delta = new_debt
            .first()
            .map(|entry| {
                format!(
                    "{} {} {} current={} baseline={} delta={}",
                    entry.path,
                    entry.family,
                    entry.selector_callee,
                    entry.current_count,
                    entry.baseline_count,
                    entry.new_debt_count()
                )
            })
            .unwrap_or_else(|| "no first delta".to_string());
        return Err(anyhow!(
            "{} no-panic baseline entr(ies) would add new debt; first delta: {}; rerun with --reset only in the dedicated baseline PR",
            new_debt.len(),
            first_delta
        ));
    }

    let stale = stale_no_panic_baseline_entries(existing, &baseline_entries_to_findings(current));
    if !stale.is_empty() {
        eprintln!(
            "no-panic baseline: refresh will drop or reduce {} stale/surplus entr(ies)",
            stale.len()
        );
        for entry in stale.iter().take(NO_PANIC_REPORT_STALE_LIMIT) {
            eprintln!(
                "no-panic baseline: stale {} {} {} baseline={} current={} surplus={} snippet={}",
                entry.path,
                entry.family,
                entry.selector_callee,
                entry.baseline_count,
                entry.current_count,
                entry.surplus_count(),
                entry.snippet
            );
        }
        if stale.len() > NO_PANIC_REPORT_STALE_LIMIT {
            eprintln!(
                "no-panic baseline: ... and {} more stale baseline entr(ies)",
                stale.len().saturating_sub(NO_PANIC_REPORT_STALE_LIMIT)
            );
        }
    }

    Ok(current.to_vec())
}

fn render_no_panic_baseline(entries: &[NoPanicBaselineEntry]) -> String {
    let mut out = String::new();
    out.push_str("schema_version = \"1.0\"\n");
    out.push_str("policy = \"no-panic-baseline\"\n");
    out.push_str("mode = \"no-new-debt\"\n\n");
    out.push_str("# Generated by `cargo run -p xtask -- no-panic baseline --reset`.\n");
    out.push_str(
        "# Refresh without --reset may drop disappeared entries but refuses new debt.\n\n",
    );

    for entry in entries {
        out.push_str("[[baseline]]\n");
        out.push_str(&format!(
            "path = \"{}\"\n",
            escape_toml_basic_string(&entry.path)
        ));
        out.push_str(&format!("family = \"{}\"\n", entry.family));
        out.push_str(&format!(
            "snippet = \"{}\"\n",
            escape_toml_basic_string(&entry.snippet)
        ));
        out.push_str(&format!("count = {}\n", entry.count));
        out.push_str("\n[baseline.selector]\n");
        out.push_str(&format!("kind = \"{}\"\n", entry.selector_kind));
        out.push_str(&format!("callee = \"{}\"\n", entry.selector_callee));
        if let Some(container) = &entry.selector_container {
            out.push_str(&format!(
                "container = \"{}\"\n",
                escape_toml_basic_string(container)
            ));
        }
        out.push_str("\n[baseline.last_seen]\n");
        out.push_str(&format!("line = {}\n", entry.last_seen_line));
        out.push_str(&format!("column = {}\n\n", entry.last_seen_column));
    }

    out
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
    let mut remaining = no_panic_allowlist_counts(entries);
    let mut unmatched = Vec::new();
    for finding in findings {
        let key = finding.identity();
        if let Some(count) = remaining.get_mut(&key)
            && *count > 0
        {
            *count = count.saturating_sub(1);
            continue;
        }
        unmatched.push(finding.clone());
    }
    unmatched
}

fn match_findings_against_baseline(
    findings: &[PanicFinding],
    entries: &[NoPanicBaselineEntry],
) -> Vec<PanicFinding> {
    let mut remaining = no_panic_baseline_counts(entries);
    let mut unmatched = Vec::new();
    for finding in findings {
        let key = finding.identity();
        if let Some(count) = remaining.get_mut(&key)
            && *count > 0
        {
            *count = count.saturating_sub(1);
            continue;
        }
        unmatched.push(finding.clone());
    }
    unmatched
}

fn stale_no_panic_baseline_entries(
    entries: &[NoPanicBaselineEntry],
    findings: &[PanicFinding],
) -> Vec<NoPanicBaselineDelta> {
    let current_counts = no_panic_finding_counts(findings);
    entries
        .iter()
        .filter_map(|entry| {
            let current_count = current_counts
                .get(&entry.identity())
                .copied()
                .unwrap_or_default();
            if current_count < entry.count {
                Some(NoPanicBaselineDelta::from_entry(
                    entry,
                    entry.count,
                    current_count,
                ))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
fn no_panic_entry_matches_finding(entry: &NoPanicAllowEntry, finding: &PanicFinding) -> bool {
    entry.identity() == finding.identity()
}

fn no_panic_allowlist_counts(entries: &[NoPanicAllowEntry]) -> BTreeMap<NoPanicIdentity, usize> {
    entries
        .iter()
        .map(|entry| (entry.identity(), entry.count))
        .collect()
}

fn no_panic_baseline_counts(entries: &[NoPanicBaselineEntry]) -> BTreeMap<NoPanicIdentity, usize> {
    entries
        .iter()
        .map(|entry| (entry.identity(), entry.count))
        .collect()
}

fn no_panic_finding_counts(findings: &[PanicFinding]) -> BTreeMap<NoPanicIdentity, usize> {
    let mut counts: BTreeMap<NoPanicIdentity, usize> = BTreeMap::new();
    for finding in findings {
        let slot = counts.entry(finding.identity()).or_default();
        *slot = slot.saturating_add(1);
    }
    counts
}

fn no_panic_baseline_occurrences(entries: &[NoPanicBaselineEntry]) -> usize {
    entries
        .iter()
        .fold(0usize, |total, entry| total.saturating_add(entry.count))
}

fn baseline_entries_to_findings(entries: &[NoPanicBaselineEntry]) -> Vec<PanicFinding> {
    let mut findings = Vec::new();
    for entry in entries {
        let Some(family) = PanicFamily::from_str(&entry.family) else {
            continue;
        };
        for _ in 0..entry.count {
            findings.push(PanicFinding {
                path: entry.path.clone(),
                family,
                container: entry.selector_container.clone(),
                snippet: entry.snippet.clone(),
                line: entry.last_seen_line,
                column: entry.last_seen_column,
            });
        }
    }
    findings
}

fn combined_no_panic_findings(
    strict_findings: &[PanicFinding],
    advisory_findings: &[PanicFinding],
) -> Vec<PanicFinding> {
    let mut findings = Vec::with_capacity(
        strict_findings
            .len()
            .saturating_add(advisory_findings.len()),
    );
    findings.extend(strict_findings.iter().cloned());
    findings.extend(advisory_findings.iter().cloned());
    findings.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
    });
    findings
}

fn stale_no_panic_entries<'a>(
    entries: &'a [NoPanicAllowEntry],
    findings: &[PanicFinding],
) -> Vec<&'a NoPanicAllowEntry> {
    let mut consumed: BTreeMap<NoPanicIdentity, usize> = BTreeMap::new();
    for finding in findings {
        let slot = consumed.entry(finding.identity()).or_default();
        *slot = slot.saturating_add(1);
    }
    entries
        .iter()
        .filter(|entry| {
            let seen = consumed.get(&entry.identity()).copied().unwrap_or_default();
            seen < entry.count
        })
        .collect()
}

fn write_no_panic_report(root: &Path, report: &NoPanicReport) -> Result<()> {
    let report_dir = root.join("target/policy");
    fs::create_dir_all(&report_dir)?;
    fs::write(
        report_dir.join("no-panic-report.md"),
        render_no_panic_report_markdown(report),
    )?;
    fs::write(
        report_dir.join("no-panic-report.json"),
        render_no_panic_report_json(report),
    )?;
    Ok(())
}

fn render_no_panic_report_markdown(report: &NoPanicReport) -> String {
    let mut out = String::new();
    out.push_str("# No-panic Policy Report\n\n");
    out.push_str("| Field | Value |\n");
    out.push_str("| --- | --- |\n");
    out.push_str(&format!(
        "| baseline_mode | `{}` |\n",
        escape_markdown_table_cell(&report.baseline_mode)
    ));
    out.push_str(&format!(
        "| baseline_ignored | `{}` |\n",
        report.baseline_ignored
    ));
    out.push_str(&format!(
        "| allowlist_entries | `{}` |\n",
        report.allowlist_entries
    ));
    out.push_str(&format!(
        "| baseline_entries | `{}` |\n",
        report.baseline_entries
    ));
    out.push_str(&format!(
        "| baseline_occurrences | `{}` |\n",
        report.baseline_occurrences
    ));
    out.push_str(&format!(
        "| strict_findings | `{}` |\n",
        report.strict_findings
    ));
    out.push_str(&format!(
        "| advisory_findings | `{}` |\n",
        report.advisory_findings
    ));
    out.push_str(&format!("| new_debt | `{}` |\n", report.new_debt.len()));
    out.push_str(&format!(
        "| stale_allowlist_entries | `{}` |\n",
        report.stale_allowlist.len()
    ));
    out.push_str(&format!(
        "| stale_baseline_entries | `{}` |\n",
        report.stale_baseline.len()
    ));

    render_no_panic_findings_markdown(&mut out, "New Debt", &report.new_debt);
    render_no_panic_allowlist_markdown(
        &mut out,
        "Stale Allowlist Entries",
        &report.stale_allowlist,
    );
    render_no_panic_baseline_deltas_markdown(
        &mut out,
        "Stale Baseline Entries",
        &report.stale_baseline,
    );
    out
}

fn render_no_panic_findings_markdown(out: &mut String, heading: &str, findings: &[PanicFinding]) {
    out.push_str(&format!("\n## {heading}\n\n"));
    if findings.is_empty() {
        out.push_str("None.\n");
        return;
    }
    out.push_str("| path | line | column | family | selector | snippet |\n");
    out.push_str("| --- | ---: | ---: | --- | --- | --- |\n");
    for finding in findings.iter().take(NO_PANIC_REPORT_STALE_LIMIT) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            escape_markdown_table_cell(&finding.path),
            finding.line,
            finding.column,
            finding.family.as_str(),
            finding.family.callee(),
            escape_markdown_table_cell(&finding.snippet),
        ));
    }
    append_markdown_truncation(out, findings.len());
}

fn render_no_panic_allowlist_markdown(
    out: &mut String,
    heading: &str,
    entries: &[NoPanicAllowEntry],
) {
    out.push_str(&format!("\n## {heading}\n\n"));
    if entries.is_empty() {
        out.push_str("None.\n");
        return;
    }
    out.push_str("| id | path | family | selector | count | snippet |\n");
    out.push_str("| --- | --- | --- | --- | ---: | --- |\n");
    for entry in entries.iter().take(NO_PANIC_REPORT_STALE_LIMIT) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            escape_markdown_table_cell(&entry.id),
            escape_markdown_table_cell(&entry.path),
            entry.family,
            entry.selector_callee,
            entry.count,
            escape_markdown_table_cell(&entry.snippet),
        ));
    }
    append_markdown_truncation(out, entries.len());
}

fn render_no_panic_baseline_deltas_markdown(
    out: &mut String,
    heading: &str,
    entries: &[NoPanicBaselineDelta],
) {
    out.push_str(&format!("\n## {heading}\n\n"));
    if entries.is_empty() {
        out.push_str("None.\n");
        return;
    }
    out.push_str("| path | family | selector | baseline | current | surplus | snippet |\n");
    out.push_str("| --- | --- | --- | ---: | ---: | ---: | --- |\n");
    for entry in entries.iter().take(NO_PANIC_REPORT_STALE_LIMIT) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            escape_markdown_table_cell(&entry.path),
            entry.family,
            entry.selector_callee,
            entry.baseline_count,
            entry.current_count,
            entry.surplus_count(),
            escape_markdown_table_cell(&entry.snippet),
        ));
    }
    append_markdown_truncation(out, entries.len());
}

fn append_markdown_truncation(out: &mut String, len: usize) {
    if len > NO_PANIC_REPORT_STALE_LIMIT {
        out.push_str(&format!(
            "\nShowing first {NO_PANIC_REPORT_STALE_LIMIT} of {len} entr(ies).\n"
        ));
    }
}

fn render_no_panic_report_json(report: &NoPanicReport) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"baseline_mode\": \"{}\",\n",
        escape_json_string(&report.baseline_mode)
    ));
    out.push_str(&format!(
        "  \"baseline_ignored\": {},\n",
        report.baseline_ignored
    ));
    out.push_str(&format!(
        "  \"allowlist_entries\": {},\n",
        report.allowlist_entries
    ));
    out.push_str(&format!(
        "  \"baseline_entries\": {},\n",
        report.baseline_entries
    ));
    out.push_str(&format!(
        "  \"baseline_occurrences\": {},\n",
        report.baseline_occurrences
    ));
    out.push_str(&format!(
        "  \"strict_findings\": {},\n",
        report.strict_findings
    ));
    out.push_str(&format!(
        "  \"advisory_findings\": {},\n",
        report.advisory_findings
    ));
    out.push_str(&format!("  \"new_debt\": {},\n", report.new_debt.len()));
    out.push_str(&format!(
        "  \"stale_allowlist_entries\": {},\n",
        report.stale_allowlist.len()
    ));
    out.push_str(&format!(
        "  \"stale_baseline_entries\": {},\n",
        report.stale_baseline.len()
    ));
    out.push_str(&format!(
        "  \"stale_baseline_entries_truncated\": {},\n",
        report.stale_baseline.len() > NO_PANIC_REPORT_STALE_LIMIT
    ));
    out.push_str("  \"stale_baseline_entry_sample\": [\n");
    for (index, entry) in report
        .stale_baseline
        .iter()
        .take(NO_PANIC_REPORT_STALE_LIMIT)
        .enumerate()
    {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!(
            "    {{\"path\":\"{}\",\"family\":\"{}\",\"selector_kind\":\"{}\",\"selector_callee\":\"{}\",\"selector_container\":{},\"snippet\":\"{}\",\"baseline_count\":{},\"current_count\":{},\"surplus_count\":{},\"last_seen_line\":{},\"last_seen_column\":{}}}",
            escape_json_string(&entry.path),
            escape_json_string(&entry.family),
            escape_json_string(&entry.selector_kind),
            escape_json_string(&entry.selector_callee),
            json_optional_string(entry.selector_container.as_deref()),
            escape_json_string(&entry.snippet),
            entry.baseline_count,
            entry.current_count,
            entry.surplus_count(),
            entry.last_seen_line,
            entry.last_seen_column,
        ));
    }
    out.push_str("\n  ]\n");
    out.push_str("}\n");
    out
}

fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn json_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json_string(value)),
        None => "null".to_string(),
    }
}

fn top_level_usize_value(text: &str, key: &str) -> Option<usize> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return None;
        }
        let (name, value) = trimmed.split_once('=')?;
        if name.trim() != key {
            return None;
        }
        value
            .split('#')
            .next()
            .map(str::trim)
            .and_then(|value| value.parse::<usize>().ok())
    })
}

fn sub_table_usize_value(entry_text: &str, marker: &str, key: &str) -> Option<usize> {
    let value = sub_table_value(entry_text, marker, key)?;
    value.parse::<usize>().ok()
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

#[derive(Debug)]
#[expect(
    dead_code,
    reason = "companion entries are parsed for schema validation and policy-report counts"
)]
struct CompanionPolicyEntry {
    id: String,
    owner: String,
    surface: String,
    behavior: String,
    reason: String,
    covered_by: Vec<String>,
}

struct CompanionPolicySpec {
    path: &'static str,
    policy: &'static str,
    required_locator: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct CompanionPolicyLedgerSummary {
    ledgers: usize,
    entries: usize,
}

const FILE_POLICY_CLASSIFICATIONS: &[&str] = &[
    "production",
    "test",
    "tooling",
    "config",
    "generated",
    "docs",
];

const COMPANION_POLICY_SPECS: &[CompanionPolicySpec] = &[
    CompanionPolicySpec {
        path: "policy/generated-allowlist.toml",
        policy: "generated-allowlist",
        required_locator: &["paths"],
    },
    CompanionPolicySpec {
        path: "policy/executable-allowlist.toml",
        policy: "executable-allowlist",
        required_locator: &["paths", "commands"],
    },
    CompanionPolicySpec {
        path: "policy/dependency-surface-allowlist.toml",
        policy: "dependency-surface-allowlist",
        required_locator: &["paths", "dependencies"],
    },
    CompanionPolicySpec {
        path: "policy/workflow-allowlist.toml",
        policy: "workflow-allowlist",
        required_locator: &["workflows"],
    },
    CompanionPolicySpec {
        path: "policy/process-allowlist.toml",
        policy: "process-allowlist",
        required_locator: &["commands"],
    },
    CompanionPolicySpec {
        path: "policy/network-allowlist.toml",
        policy: "network-allowlist",
        required_locator: &["destinations"],
    },
];

fn check_file_policy() -> Result<()> {
    println!("🔎 Checking non-Rust file policy...");
    let root = env::current_dir()?;
    let allowlist_text = fs::read_to_string(root.join("policy/non-rust-allowlist.toml"))?;
    let entries = parse_file_policy_allowlist(&allowlist_text)?;
    enforce_file_policy_expirations(&entries)?;
    let companion_summary = check_companion_policy_ledgers(&root)?;

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
        "✅ file policy: {} tracked/untracked non-ignored file(s) checked, {} allowlist entr(ies), {} companion ledger entr(ies)",
        files.len(),
        entries.len(),
        companion_summary.entries
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

fn check_companion_policy_ledgers(root: &Path) -> Result<CompanionPolicyLedgerSummary> {
    let mut entries = 0usize;
    for spec in COMPANION_POLICY_SPECS {
        let text = fs::read_to_string(root.join(spec.path))?;
        let parsed = parse_companion_policy_ledger(spec, &text)?;
        entries = entries.saturating_add(parsed.len());
    }
    Ok(CompanionPolicyLedgerSummary {
        ledgers: COMPANION_POLICY_SPECS.len(),
        entries,
    })
}

fn parse_companion_policy_ledger(
    spec: &CompanionPolicySpec,
    text: &str,
) -> Result<Vec<CompanionPolicyEntry>> {
    let schema_version = top_level_quoted_value(text, "schema_version")
        .ok_or_else(|| anyhow!("{} is missing `schema_version`", spec.path))?;
    if schema_version != "1.0" {
        return Err(anyhow!(
            "{} schema_version must be `1.0`, found `{schema_version}`",
            spec.path
        ));
    }

    let policy = top_level_quoted_value(text, "policy")
        .ok_or_else(|| anyhow!("{} is missing `policy`", spec.path))?;
    if policy != spec.policy {
        return Err(anyhow!(
            "{} policy must be `{}`, found `{policy}`",
            spec.path,
            spec.policy
        ));
    }

    for key in ["owner", "status"] {
        if top_level_quoted_value(text, key).is_none() {
            return Err(anyhow!("{} is missing `{key}`", spec.path));
        }
    }

    let entries = table_array_entries(text, "[[allow]]");
    if entries.is_empty() {
        return Err(anyhow!(
            "{} must contain at least one [[allow]] entry",
            spec.path
        ));
    }

    let mut parsed = Vec::with_capacity(entries.len());
    let mut ids = BTreeSet::new();
    for (index, raw) in entries.iter().enumerate() {
        let entry_no = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("{} entry index overflow", spec.path))?;
        let field = |key: &str| -> Result<String> {
            top_level_quoted_value(raw, key).ok_or_else(|| {
                anyhow!(
                    "{} entry {entry_no} is missing required field `{key}`",
                    spec.path
                )
            })
        };
        let id = field("id")?;
        if !ids.insert(id.clone()) {
            return Err(anyhow!("{} duplicates allow entry id `{id}`", spec.path));
        }
        let owner = field("owner")?;
        let surface = field("surface")?;
        let behavior = field("behavior")?;
        let reason = field("reason")?;
        let covered_by = string_array_after_root(raw, "covered_by").unwrap_or_default();
        if covered_by.is_empty() {
            return Err(anyhow!(
                "{} entry {id} must set non-empty `covered_by`",
                spec.path
            ));
        }

        let mut has_locator = false;
        for key in spec.required_locator {
            if !string_array_after_root(raw, key)
                .unwrap_or_default()
                .is_empty()
            {
                has_locator = true;
            }
        }
        if !has_locator {
            return Err(anyhow!(
                "{} entry {id} must set at least one of: {}",
                spec.path,
                spec.required_locator.join(", ")
            ));
        }
        if spec.policy == "generated-allowlist"
            && string_array_after_root(raw, "generated_by")
                .unwrap_or_default()
                .is_empty()
        {
            return Err(anyhow!(
                "{} entry {id} must set non-empty `generated_by`",
                spec.path
            ));
        }

        if companion_entry_has_broad_path_glob(raw)
            && top_level_quoted_value(raw, "broad_glob_reason").is_none()
        {
            return Err(anyhow!(
                "{} entry {id} uses a broad path glob and must set `broad_glob_reason`",
                spec.path
            ));
        }

        for key in ["review_after", "expires"] {
            if let Some(value) = top_level_quoted_value(raw, key) {
                parse_ci_date(&value, &format!("{} entry {id} {key}", spec.path))?;
            }
        }

        parsed.push(CompanionPolicyEntry {
            id,
            owner,
            surface,
            behavior,
            reason,
            covered_by,
        });
    }

    Ok(parsed)
}

fn companion_entry_has_broad_path_glob(raw: &str) -> bool {
    string_array_after_root(raw, "paths")
        .unwrap_or_default()
        .iter()
        .any(|path| path.contains('*'))
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

const EVIDENCE_PARITY_MANIFEST_PATH: &str = "policy/evidence-parity.toml";

const EVIDENCE_PARITY_REQUIRED_SURFACES: &[&str] =
    &["rust", "cli", "rest", "grpc", "python", "typescript"];

const EVIDENCE_PARITY_REQUIRED_CONTRACTS: &[&str] = &[
    "parse-write",
    "validate",
    "normalize",
    "ack",
    "profile-lint-explain-test",
    "redaction-quarantine",
    "bundle-replay",
    "corpus-summary-fingerprint-diff",
    "safe-error-shape",
    "schema-version-behavior",
    "phi-sentinel-behavior",
];

const EVIDENCE_PARITY_ALLOWED_CONTRACT_STATUS: &[&str] = &["partially-proven", "gap-recorded"];
const EVIDENCE_PARITY_ALLOWED_RUST_STATES: &[&str] = &["stable", "surface-specific-tests"];
const EVIDENCE_PARITY_ALLOWED_CLI_STATES: &[&str] =
    &["stable", "stable-where-exposed", "surface-specific-tests"];
const EVIDENCE_PARITY_ALLOWED_REST_STATES: &[&str] = &[
    "stable",
    "stable-where-exposed",
    "parse-stable-write-scoped-to-exposed-endpoints",
    "surface-specific-tests",
];
const EVIDENCE_PARITY_ALLOWED_GRPC_STATES: &[&str] = &[
    "stable",
    "stable-for-implemented-rpcs",
    "stable-for-profile-rpcs",
    "stable-for-validate-redacted",
    "stable-for-configured-root-rpcs",
    "stable-for-inline-messages",
    "stable-for-implemented-v2-rpcs",
    "parse-stable-write-scoped-to-exposed-rpcs",
    "required-for-evidence-rpcs",
    "surface-specific-tests",
];
const EVIDENCE_PARITY_ALLOWED_PYTHON_STATES: &[&str] = &[
    "local-wheel-only",
    "local-wheel-specific-tests",
    "redaction-local-wheel-only-quarantine-not-claimed",
    "required-for-claimed-artifacts",
];

const FIRST_USE_RECEIPT_ROOT: &str = "target/hl7v2-receipt";
const FIRST_10_MINUTES_GUIDE_ROOT: &str = "target/hl7v2-first-10-minutes";
const FIRST_USE_BY_SURFACE_GUIDE_ROOT: &str = "target/hl7v2-first-use-by-surface";
const VENDOR_UPGRADE_DIFF_GUIDE_ROOT: &str = "target/hl7v2-vendor-upgrade-diff";
const OPERATOR_ERROR_GUIDANCE_ROOT: &str = "target/hl7v2-operator-error-guidance";
const FIRST_USE_REDACTION_POLICY: &str = r#"[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.8"
action = "retain"
reason = "administrative sex is required to reproduce validation"
"#;
const SAFE_SUPPORT_BUNDLE_GUIDE_ROOT: &str = "target/hl7v2-safe-support-bundle";
const EVIDENCE_ARTIFACTS_GUIDE_ROOT: &str = "target/hl7v2-evidence-artifacts";
const SIDECAR_GUIDE_ROOT: &str = "target/hl7v2-sidecar";
fn sidecar_guide_config(port: u16) -> String {
    format!(
        r#"[server]
host = "127.0.0.1"
port = {port}
bundle_output_root = "target/hl7v2-sidecar/bundles"

[ack]
mode = "original"
accept_on = "valid"
reject_on = ["parse_error", "validation_error"]
include_error_text = true

[quarantine]
enabled = true
path = "target/hl7v2-sidecar/quarantine"
write_redacted = true
write_report = true
write_bundle = true
"#
    )
}

const SAFE_SUPPORT_BUNDLE_REDACTION_POLICY: &str = r#"[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier is needed for correlation without raw MRN"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name is not needed for support analysis"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth is not needed for support analysis"

[[rules]]
path = "PID.8"
action = "retain"
reason = "administrative sex is required to reproduce the validation issue"
"#;
const GUIDE_PHI_SENTINELS: &[&str] = &["123456^^^HOSP^MR", "123456", "19800101"];

const PYTHON_LOCAL_WHEEL_PROOF_DEFAULT_ROOT: &str = "target/hl7v2-python-local-wheel-proof";
const PYTHON_PUBLIC_REGISTRY_PROOF_DEFAULT_ROOT: &str = "target/hl7v2-python-public-registry-proof";
const PYTHON_LOCAL_WHEEL_MATURIN_REQUIREMENT: &str = "maturin==1.13.1";

fn python_local_wheel_proof(
    root: Option<PathBuf>,
    python: &str,
    rust_toolchain: &str,
    keep_existing: bool,
) -> Result<()> {
    println!("Checking Python local-wheel proof...");

    let metadata = MetadataCommand::new().no_deps().exec()?;
    let workspace_root = metadata.workspace_root.into_std_path_buf();
    let root = prepare_python_local_wheel_root(root, keep_existing, &workspace_root)?;
    let venv = root.join("venv");
    let dist = root.join("dist");
    let cargo_target = root.join("cargo-target");

    recreate_dir(&venv)?;
    recreate_dir(&dist)?;
    recreate_dir(&cargo_target)?;

    let venv_arg = path_to_arg(&venv)?;
    println!("Creating proof virtualenv at {}", venv.display());
    run_program_with_env_in_dir(
        python,
        &["-m", "venv", &venv_arg],
        &[],
        Some(&workspace_root),
    )?;

    let venv_python = python_executable_in_venv(&venv);
    println!("Installing maturin into proof virtualenv...");
    run_command_with_env_in_dir(
        &venv_python,
        &[
            "-m",
            "pip",
            "install",
            "--upgrade",
            "pip",
            PYTHON_LOCAL_WHEEL_MATURIN_REQUIREMENT,
        ],
        &[],
        Some(&workspace_root),
    )?;

    let dist_arg = path_to_arg(&dist)?;
    let cargo_target_arg = path_to_arg(&cargo_target)?;
    println!("Building hl7v2 wheel with maturin...");
    run_command_with_env_in_dir(
        &venv_python,
        &[
            "-m",
            "maturin",
            "build",
            "--release",
            "--out",
            &dist_arg,
            "--target-dir",
            &cargo_target_arg,
        ],
        &[
            ("PYO3_USE_ABI3_FORWARD_COMPATIBILITY", "1"),
            ("RUSTUP_TOOLCHAIN", rust_toolchain),
        ],
        Some(&workspace_root),
    )?;

    let wheel = single_wheel_in_dist(&dist)?;
    let wheel_arg = path_to_arg(&wheel)?;
    println!("Installing local wheel {}", wheel.display());
    run_command_with_env_in_dir(
        &venv_python,
        &["-m", "pip", "install", "--force-reinstall", &wheel_arg],
        &[],
        Some(&workspace_root),
    )?;

    println!("Checking import and Python evidence helpers...");
    run_command_with_env_in_dir(
        &venv_python,
        &["-c", "import hl7v2; print(hl7v2.__version__)"],
        &[],
        Some(&workspace_root),
    )?;
    run_command_with_env_in_dir(
        &venv_python,
        &["tests/python_smoke/smoke.py"],
        &[],
        Some(&workspace_root),
    )?;
    run_command_with_env_in_dir(
        &venv_python,
        &["tests/python_smoke/evidence_workflow_guide.py"],
        &[],
        Some(&workspace_root),
    )?;
    run_command_with_env_in_dir(
        &venv_python,
        &["tests/python_smoke/dirty_evidence_workflow.py"],
        &[],
        Some(&workspace_root),
    )?;

    println!(
        "Python local-wheel proof passed at {}. This does not claim TestPyPI or PyPI availability.",
        root.display()
    );
    Ok(())
}

fn python_public_registry_proof(
    root: Option<PathBuf>,
    python: &str,
    index: PythonPackageIndex,
    version: Option<String>,
    keep_existing: bool,
) -> Result<()> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let version = match version {
        Some(version) => version,
        None => hl7v2_python_package_version(&metadata)?,
    };
    let workspace_root = metadata.workspace_root.into_std_path_buf();
    let root = prepare_python_public_registry_proof_root(root, keep_existing, &workspace_root)?;
    let venv = root.join("venv");

    recreate_dir(&venv)?;

    let venv_arg = path_to_arg(&venv)?;
    println!(
        "Creating Python public-registry proof virtualenv at {}",
        venv.display()
    );
    run_program_with_env_in_dir(
        python,
        &["-m", "venv", &venv_arg],
        &[],
        Some(&workspace_root),
    )?;

    let venv_python = python_executable_in_venv(&venv);
    println!("Upgrading pip in proof virtualenv...");
    run_command_with_env_in_dir(
        &venv_python,
        &["-m", "pip", "install", "--upgrade", "pip"],
        &[],
        Some(&workspace_root),
    )?;

    let index_url = python_package_index_url(index);
    let package = format!("hl7v2=={version}");
    println!(
        "Installing public Python package {package} from {}...",
        python_package_index_label(index)
    );
    run_command_with_env_in_dir(
        &venv_python,
        &[
            "-m",
            "pip",
            "install",
            "--index-url",
            index_url,
            "--no-deps",
            "--force-reinstall",
            &package,
        ],
        &[],
        Some(&workspace_root),
    )?;

    println!("Checking import and Python evidence helpers from installed public package...");
    run_command_with_env_in_dir(
        &venv_python,
        &["-c", "import hl7v2; print(hl7v2.__version__)"],
        &[],
        Some(&workspace_root),
    )?;
    run_command_with_env_in_dir(
        &venv_python,
        &["tests/python_smoke/smoke.py"],
        &[],
        Some(&workspace_root),
    )?;
    run_command_with_env_in_dir(
        &venv_python,
        &["tests/python_smoke/evidence_workflow_guide.py"],
        &[],
        Some(&workspace_root),
    )?;
    run_command_with_env_in_dir(
        &venv_python,
        &["tests/python_smoke/dirty_evidence_workflow.py"],
        &[],
        Some(&workspace_root),
    )?;

    println!(
        "Python public-registry proof passed for {} {package} at {}.",
        python_package_index_label(index),
        root.display()
    );
    println!(
        "This proves install-back only for the selected index and does not claim the other index."
    );
    Ok(())
}

fn hl7v2_python_package_version(metadata: &Metadata) -> Result<String> {
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == "hl7v2-python")
        .ok_or_else(|| anyhow!("cargo metadata did not include hl7v2-python"))?;
    Ok(package.version.to_string())
}

fn python_package_index_url(index: PythonPackageIndex) -> &'static str {
    match index {
        PythonPackageIndex::Testpypi => "https://test.pypi.org/simple/",
        PythonPackageIndex::Pypi => "https://pypi.org/simple/",
    }
}

fn python_package_index_label(index: PythonPackageIndex) -> &'static str {
    match index {
        PythonPackageIndex::Testpypi => "TestPyPI",
        PythonPackageIndex::Pypi => "PyPI",
    }
}

fn prepare_python_local_wheel_root(
    root: Option<PathBuf>,
    keep_existing: bool,
    workspace_root: &Path,
) -> Result<PathBuf> {
    let requested = root.unwrap_or_else(|| PathBuf::from(PYTHON_LOCAL_WHEEL_PROOF_DEFAULT_ROOT));
    let absolute = if requested.is_absolute() {
        requested
    } else {
        workspace_root.join(requested)
    };
    let leaf = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Python local-wheel proof root must have a final path component"))?;
    if !(leaf.contains("hl7v2") && leaf.contains("python") && leaf.contains("proof")) {
        return Err(anyhow!(
            "refusing to use scratch root '{}': final component must contain 'hl7v2', 'python', and 'proof'",
            absolute.display()
        ));
    }

    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)?;
    }

    if absolute.exists() && !keep_existing {
        fs::remove_dir_all(&absolute)?;
    }
    fs::create_dir_all(&absolute)?;

    Ok(absolute)
}

fn prepare_python_public_registry_proof_root(
    root: Option<PathBuf>,
    keep_existing: bool,
    workspace_root: &Path,
) -> Result<PathBuf> {
    let requested =
        root.unwrap_or_else(|| PathBuf::from(PYTHON_PUBLIC_REGISTRY_PROOF_DEFAULT_ROOT));
    let absolute = if requested.is_absolute() {
        requested
    } else {
        workspace_root.join(requested)
    };
    let leaf = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            anyhow!("Python public-registry proof root must have a final path component")
        })?;
    for required in ["hl7v2", "python", "registry", "proof"] {
        if !leaf.contains(required) {
            return Err(anyhow!(
                "refusing to use scratch root '{}': final component must contain '{required}'",
                absolute.display()
            ));
        }
    }

    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)?;
    }

    if absolute.exists() && !keep_existing {
        fs::remove_dir_all(&absolute)?;
    }
    fs::create_dir_all(&absolute)?;

    Ok(absolute)
}

fn recreate_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn python_executable_in_venv(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    }
}

fn single_wheel_in_dist(dist: &Path) -> Result<PathBuf> {
    let mut wheels = fs::read_dir(dist)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("whl"))
        .collect::<Vec<_>>();
    wheels.sort();

    match wheels.as_slice() {
        [wheel] => Ok(wheel.clone()),
        [] => Err(anyhow!("no wheel found in {}", dist.display())),
        _ => Err(anyhow!(
            "expected exactly one wheel in {}, found {}",
            dist.display(),
            wheels.len()
        )),
    }
}

fn path_to_arg(path: &Path) -> Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn check_evidence_parity() -> Result<()> {
    println!("🔎 Checking evidence parity manifest...");
    let root = env::current_dir()?;
    let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
    check_evidence_parity_manifest_text(&text)?;
    println!(
        "✅ evidence parity: {} surface(s), {} contract(s), and registry non-claim boundaries checked",
        EVIDENCE_PARITY_REQUIRED_SURFACES.len(),
        EVIDENCE_PARITY_REQUIRED_CONTRACTS.len()
    );
    Ok(())
}

fn check_evidence_parity_acceptance(include_python: bool) -> Result<()> {
    println!("🔎 Checking cross-surface evidence parity acceptance...");
    check_evidence_parity()?;
    check_safe_error_phi_parity(include_python)?;
    check_profile_parity(include_python)?;
    check_schema_version_parity(include_python)?;
    check_dirty_corpus_parity(include_python)?;
    check_bundle_replay_parity(include_python)?;
    println!("✅ Cross-surface evidence parity acceptance checks passed!");
    Ok(())
}

fn check_first_use_guides(include_python: bool, include_public_crates: bool) -> Result<()> {
    println!("ðŸ”Ž Checking executable first-use guides...");
    check_first_10_minutes_guide()?;
    check_full_evidence_receipt_cli_recipe()?;

    println!("Checking Rust user journey acceptance test...");
    run_command(
        "cargo",
        &[
            "test",
            "-p",
            "hl7v2",
            "--test",
            "user_journey",
            "--all-features",
            "--locked",
        ],
    )?;

    println!("Checking CLI support-bundle user journey acceptance test...");
    run_command(
        "cargo",
        &[
            "test",
            "-p",
            "hl7v2-cli",
            "--test",
            "integration_tests",
            "journey_cli_validate_redact_support_bundle_replay_produces_shareable_receipts",
            "--locked",
        ],
    )?;

    if include_python {
        println!("Checking Python local-wheel first-use scripts...");
        run_command("python", &["tests/python_smoke/smoke.py"])?;
        run_command("python", &["tests/python_smoke/evidence_workflow_guide.py"])?;
        run_command("python", &["tests/python_smoke/dirty_evidence_workflow.py"])?;
    } else {
        println!(
            "Python local-wheel first-use scripts skipped; pass --include-python after installing the hl7v2 wheel."
        );
    }

    if include_public_crates {
        let version = workspace_crate_version("hl7v2")?;
        println!("Checking public crates.io first-use install-back for v{version}...");
        run_command(
            "python",
            &[
                "tests/public_crates_smoke/smoke.py",
                "--version",
                version.as_str(),
            ],
        )?;
    } else {
        println!(
            "Public crates.io install-back smoke skipped; pass --include-public-crates when registry proof is needed."
        );
    }

    println!("âœ… Executable first-use guide checks passed!");
    Ok(())
}

fn check_first_10_minutes_guide() -> Result<()> {
    println!("Checking First 10 Minutes guide recipe...");
    let workspace_root = env::current_dir()?;
    let guide_root = workspace_root.join(FIRST_10_MINUTES_GUIDE_ROOT);
    let target_root = workspace_root.join("target");
    if !guide_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare First 10 Minutes guide output outside target/: {}",
            guide_root.display()
        ));
    }
    if guide_root.exists() {
        fs::remove_dir_all(&guide_root)?;
    }

    let reports = guide_root.join("reports");
    let fixtures = guide_root.join("fixtures");
    let valid_fixtures = fixtures.join("valid");
    let invalid_fixtures = fixtures.join("invalid");
    fs::create_dir_all(&reports)?;
    fs::create_dir_all(&valid_fixtures)?;
    fs::create_dir_all(&invalid_fixtures)?;

    let profile = workspace_root.join("profiles/generic.yaml");
    let valid_message = workspace_root.join("test_data/valid_message.hl7");
    let invalid_message = workspace_root.join("test_data/invalid_message.hl7");
    let sample = guide_root.join("sample.hl7");
    let policy = guide_root.join("safe-analysis.toml");
    let bundle = guide_root.join("issue-bundle");

    ensure_existing_file(&profile)?;
    ensure_existing_file(&valid_message)?;
    ensure_existing_file(&invalid_message)?;

    fs::copy(&valid_message, valid_fixtures.join("valid_message.hl7"))?;
    fs::copy(
        &invalid_message,
        invalid_fixtures.join("invalid_message.hl7"),
    )?;
    fs::write(&policy, FIRST_USE_REDACTION_POLICY)?;

    let doctor = run_cli_guide_command_capture(
        "First 10 Minutes doctor",
        vec![
            "doctor".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let doctor_json: serde_json::Value = serde_json::from_str(&doctor)?;
    ensure_json_has_key(&doctor_json, "version", "First 10 Minutes doctor")?;
    ensure_json_has_key(&doctor_json, "checks", "First 10 Minutes doctor")?;

    run_cli_guide_command(
        "First 10 Minutes sample generation",
        vec![
            "sample".to_string(),
            "--type".to_string(),
            "ADT_A01".to_string(),
            "--output".to_string(),
            path_to_arg(&sample)?,
        ],
    )?;
    ensure_existing_file(&sample)?;

    let validate_sample = run_cli_guide_command_capture(
        "First 10 Minutes sample validation",
        vec![
            "validate-sample".to_string(),
            "--type".to_string(),
            "ADT_A01".to_string(),
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--schema-version".to_string(),
            "2".to_string(),
        ],
    )?;
    let validate_sample_json: serde_json::Value = serde_json::from_str(&validate_sample)?;
    ensure_json_path_string(
        &validate_sample_json,
        &["schema_version"],
        "2",
        "First 10 Minutes sample validation",
    )?;
    ensure_json_path_string(
        &validate_sample_json,
        &["tool_name"],
        "hl7v2-cli",
        "First 10 Minutes sample validation",
    )?;
    ensure_json_path_bool(
        &validate_sample_json,
        &["valid"],
        true,
        "First 10 Minutes sample validation",
    )?;
    ensure_json_path_string(
        &validate_sample_json,
        &["message_type"],
        "ADT^A01",
        "First 10 Minutes sample validation",
    )?;

    let profile_lint = run_cli_guide_command_capture(
        "First 10 Minutes profile lint",
        vec![
            "profile".to_string(),
            "lint".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
        ],
    )?;
    let lint_json: serde_json::Value = serde_json::from_str(&profile_lint)?;
    ensure_json_path_bool(
        &lint_json,
        &["valid"],
        true,
        "First 10 Minutes profile lint",
    )?;
    ensure_json_path_u64(
        &lint_json,
        &["error_count"],
        0,
        "First 10 Minutes profile lint",
    )?;
    ensure_json_path_u64(
        &lint_json,
        &["warning_count"],
        0,
        "First 10 Minutes profile lint",
    )?;

    let profile_explain = run_cli_guide_command_capture(
        "First 10 Minutes profile explain",
        vec![
            "profile".to_string(),
            "explain".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let explain_json: serde_json::Value = serde_json::from_str(&profile_explain)?;
    ensure_json_path_string(
        &explain_json,
        &["message_structure"],
        "GENERIC",
        "First 10 Minutes profile explain",
    )?;
    ensure_json_path_u64(
        &explain_json,
        &["summary", "required_field_count"],
        1,
        "First 10 Minutes profile explain",
    )?;
    ensure_json_path_u64(
        &explain_json,
        &["summary", "value_set_count"],
        1,
        "First 10 Minutes profile explain",
    )?;
    ensure_json_object_array_contains_fields(
        &explain_json,
        &["required_fields"],
        &[("path", "PID.3")],
        "First 10 Minutes profile explain",
    )?;

    let valid_validation = run_cli_guide_command_capture(
        "First 10 Minutes valid message validation",
        vec![
            "val".to_string(),
            path_to_arg(&valid_message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
        ],
    )?;
    let valid_validation_json: serde_json::Value = serde_json::from_str(&valid_validation)?;
    ensure_json_path_bool(
        &valid_validation_json,
        &["valid"],
        true,
        "First 10 Minutes valid message validation",
    )?;
    ensure_json_path_string(
        &valid_validation_json,
        &["message_type"],
        "ADT^A01",
        "First 10 Minutes valid message validation",
    )?;
    ensure_json_path_u64(
        &valid_validation_json,
        &["issue_count"],
        0,
        "First 10 Minutes valid message validation",
    )?;

    let invalid_validation = run_cli_guide_command_capture_allow_codes(
        "First 10 Minutes invalid message validation",
        vec![
            "val".to_string(),
            path_to_arg(&invalid_message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
        ],
        &[1],
    )?;
    let invalid_validation_json: serde_json::Value = serde_json::from_str(&invalid_validation)?;
    ensure_json_path_bool(
        &invalid_validation_json,
        &["valid"],
        false,
        "First 10 Minutes invalid message validation",
    )?;
    ensure_json_path_u64(
        &invalid_validation_json,
        &["issue_count"],
        1,
        "First 10 Minutes invalid message validation",
    )?;
    ensure_json_object_array_contains_fields(
        &invalid_validation_json,
        &["issues"],
        &[("code", "value_not_in_set"), ("path", "PID.8")],
        "First 10 Minutes invalid message validation",
    )?;

    let profile_test = run_cli_guide_command_capture(
        "First 10 Minutes profile fixture test",
        vec![
            "profile".to_string(),
            "test".to_string(),
            path_to_arg(&profile)?,
            path_to_arg(&fixtures)?,
            "--report".to_string(),
            "json".to_string(),
        ],
    )?;
    let profile_test_json: serde_json::Value = serde_json::from_str(&profile_test)?;
    ensure_json_path_bool(
        &profile_test_json,
        &["valid"],
        true,
        "First 10 Minutes profile fixture test",
    )?;
    ensure_json_path_u64(
        &profile_test_json,
        &["case_count"],
        2,
        "First 10 Minutes profile fixture test",
    )?;
    ensure_json_path_u64(
        &profile_test_json,
        &["passed_count"],
        2,
        "First 10 Minutes profile fixture test",
    )?;
    ensure_json_path_u64(
        &profile_test_json,
        &["failed_count"],
        0,
        "First 10 Minutes profile fixture test",
    )?;

    let summary = run_cli_guide_command_capture(
        "First 10 Minutes corpus summary",
        vec![
            "corpus".to_string(),
            "summarize".to_string(),
            path_to_arg(&fixtures)?,
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let summary_json: serde_json::Value = serde_json::from_str(&summary)?;
    ensure_json_path_u64(
        &summary_json,
        &["file_count"],
        2,
        "First 10 Minutes corpus summary",
    )?;
    ensure_json_path_u64(
        &summary_json,
        &["message_count"],
        2,
        "First 10 Minutes corpus summary",
    )?;
    ensure_json_path_u64(
        &summary_json,
        &["parse_error_count"],
        0,
        "First 10 Minutes corpus summary",
    )?;
    ensure_json_object_array_contains_string_u64_field(
        &summary_json,
        &["message_types"],
        "value",
        "ADT^A01^ADT_A01",
        "count",
        2,
        "First 10 Minutes corpus summary",
    )?;

    let fingerprint = run_cli_guide_command_capture(
        "First 10 Minutes corpus fingerprint",
        vec![
            "corpus".to_string(),
            "fingerprint".to_string(),
            path_to_arg(&fixtures)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let fingerprint_json: serde_json::Value = serde_json::from_str(&fingerprint)?;
    ensure_json_path_string(
        &fingerprint_json,
        &["fingerprint_version"],
        "1",
        "First 10 Minutes corpus fingerprint",
    )?;
    ensure_json_path_u64(
        &fingerprint_json,
        &["message_count"],
        2,
        "First 10 Minutes corpus fingerprint",
    )?;
    ensure_json_path_u64(
        &fingerprint_json,
        &["parse_error_count"],
        0,
        "First 10 Minutes corpus fingerprint",
    )?;
    ensure_json_object_array_contains_string_u64_field(
        &fingerprint_json,
        &["validation_issue_code_counts"],
        "value",
        "value_not_in_set",
        "count",
        1,
        "First 10 Minutes corpus fingerprint",
    )?;

    let diff = run_cli_guide_command_capture(
        "First 10 Minutes corpus diff",
        vec![
            "corpus".to_string(),
            "diff".to_string(),
            path_to_arg(&valid_fixtures)?,
            path_to_arg(&invalid_fixtures)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let diff_json: serde_json::Value = serde_json::from_str(&diff)?;
    ensure_json_path_string(
        &diff_json,
        &["diff_version"],
        "1",
        "First 10 Minutes corpus diff",
    )?;
    ensure_json_path_u64(
        &diff_json,
        &["parse_error_count", "delta"],
        0,
        "First 10 Minutes corpus diff",
    )?;
    ensure_json_object_array_contains_string_i64_field(
        &diff_json,
        &["field_presence"],
        "path",
        "PID.5",
        "message_count_delta",
        -1,
        "First 10 Minutes corpus diff",
    )?;
    ensure_json_object_array_contains_string_i64_field(
        &diff_json,
        &["validation_issue_code_counts"],
        "value",
        "value_not_in_set",
        "delta",
        1,
        "First 10 Minutes corpus diff",
    )?;

    let bundle_summary = run_cli_guide_command_capture(
        "First 10 Minutes support bundle",
        vec![
            "support-bundle".to_string(),
            path_to_arg(&valid_message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--redact-policy".to_string(),
            path_to_arg(&policy)?,
            "--out".to_string(),
            path_to_arg(&bundle)?,
        ],
    )?;
    let bundle_summary_json: serde_json::Value = serde_json::from_str(&bundle_summary)?;
    ensure_json_path_string(
        &bundle_summary_json,
        &["bundle_version"],
        "1",
        "First 10 Minutes support bundle",
    )?;
    ensure_json_path_bool(
        &bundle_summary_json,
        &["validation_valid"],
        true,
        "First 10 Minutes support bundle",
    )?;
    ensure_json_path_bool(
        &bundle_summary_json,
        &["redaction_phi_removed"],
        true,
        "First 10 Minutes support bundle",
    )?;
    for artifact in [
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "profile.yaml",
        "redaction-receipt.json",
        "environment.json",
        "replay.sh",
        "replay.ps1",
        "README.md",
        "SAFE-SHARING.md",
        "manifest.json",
    ] {
        ensure_json_array_contains_string(
            &bundle_summary_json,
            &["artifacts"],
            artifact,
            "First 10 Minutes support bundle",
        )?;
        let artifact_path = bundle.join(artifact);
        ensure_existing_file(&artifact_path)?;
        ensure_file_lacks_phi_sentinels(&artifact_path)?;
    }

    let replay = run_cli_guide_command_capture(
        "First 10 Minutes replay",
        vec![
            "replay".to_string(),
            path_to_arg(&bundle)?,
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let replay_json: serde_json::Value = serde_json::from_str(&replay)?;
    ensure_json_path_string(
        &replay_json,
        &["replay_version"],
        "1",
        "First 10 Minutes replay",
    )?;
    ensure_json_path_bool(
        &replay_json,
        &["reproduced"],
        true,
        "First 10 Minutes replay",
    )?;
    for check_name in ["manifest-hashes", "report-match", "environment-match"] {
        ensure_json_object_array_contains_fields(
            &replay_json,
            &["checks"],
            &[("name", check_name), ("status", "pass")],
            "First 10 Minutes replay",
        )?;
    }

    println!(
        "First 10 Minutes guide recipe wrote {}",
        guide_root.display()
    );
    Ok(())
}

fn check_first_use_by_surface_guide() -> Result<()> {
    println!("Checking First Use By Surface guide recipe...");
    let workspace_root = env::current_dir()?;
    let guide_root = workspace_root.join(FIRST_USE_BY_SURFACE_GUIDE_ROOT);
    let target_root = workspace_root.join("target");
    if !guide_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare first-use-by-surface output outside target/: {}",
            guide_root.display()
        ));
    }
    if guide_root.exists() {
        fs::remove_dir_all(&guide_root)?;
    }

    let reports = guide_root.join("reports");
    fs::create_dir_all(&reports)?;

    println!("Checking First Use By Surface Rust route...");
    run_command(
        "cargo",
        &[
            "test",
            "-p",
            "hl7v2",
            "--test",
            "user_journey",
            "journey_rust_validate_redact_bundle_replay_produces_shareable_receipts",
            "--all-features",
            "--locked",
        ],
    )?;

    let doctor = run_cli_guide_command_capture(
        "First Use By Surface CLI doctor",
        vec![
            "doctor".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let doctor_report = reports.join("cli-doctor.json");
    fs::write(&doctor_report, &doctor)?;
    let doctor_json: serde_json::Value = serde_json::from_str(&doctor)?;
    let doctor_label = path_to_arg(&doctor_report)?;
    ensure_json_has_key(&doctor_json, "version", &doctor_label)?;
    ensure_json_has_key(&doctor_json, "checks", &doctor_label)?;
    ensure_file_lacks_phi_sentinels(&doctor_report)?;

    let profile = workspace_root.join("profiles/generic.yaml");
    let valid_message = workspace_root.join("test_data/valid_message.hl7");
    ensure_existing_file(&profile)?;
    ensure_existing_file(&valid_message)?;

    let profile_lint = reports.join("cli-profile-lint.json");
    run_cli_guide_command(
        "First Use By Surface CLI profile lint",
        vec![
            "profile".to_string(),
            "lint".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&profile_lint)?,
        ],
    )?;
    let lint = read_json_file(&profile_lint)?;
    let lint_label = path_to_arg(&profile_lint)?;
    ensure_json_path_bool(&lint, &["valid"], true, &lint_label)?;
    ensure_json_path_u64(&lint, &["error_count"], 0, &lint_label)?;
    ensure_file_lacks_phi_sentinels(&profile_lint)?;

    let validation_report = reports.join("cli-validation-report.json");
    run_cli_guide_command(
        "First Use By Surface CLI validation report",
        vec![
            "val".to_string(),
            path_to_arg(&valid_message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&validation_report)?,
        ],
    )?;
    let validation = read_json_file(&validation_report)?;
    let validation_label = path_to_arg(&validation_report)?;
    ensure_json_path_bool(&validation, &["valid"], true, &validation_label)?;
    ensure_json_has_key(&validation, "message_type", &validation_label)?;
    ensure_json_path_u64(&validation, &["issue_count"], 0, &validation_label)?;
    ensure_file_lacks_phi_sentinels(&validation_report)?;

    let corpus_summary = reports.join("cli-corpus-summary.json");
    run_cli_guide_command(
        "First Use By Surface CLI corpus summary",
        vec![
            "corpus".to_string(),
            "summarize".to_string(),
            "test_data".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&corpus_summary)?,
        ],
    )?;
    let summary = read_json_file(&corpus_summary)?;
    let summary_label = path_to_arg(&corpus_summary)?;
    ensure_json_path_u64(&summary, &["file_count"], 37, &summary_label)?;
    ensure_json_path_u64(&summary, &["message_count"], 14, &summary_label)?;
    ensure_json_path_u64(&summary, &["parse_error_count"], 23, &summary_label)?;
    ensure_json_has_key(&summary, "message_types", &summary_label)?;
    ensure_file_lacks_phi_sentinels(&corpus_summary)?;

    let server_config = run_command_capture_owned(
        "cargo",
        &[
            "run".to_string(),
            "--quiet".to_string(),
            "-p".to_string(),
            "hl7v2-server".to_string(),
            "--".to_string(),
            "--print-config".to_string(),
        ],
    )?;
    let server_config_report = reports.join("server-print-config.json");
    fs::write(&server_config_report, &server_config)?;
    let server_config_json: serde_json::Value = serde_json::from_str(&server_config)?;
    let server_config_label = path_to_arg(&server_config_report)?;
    ensure_json_path_string(
        &server_config_json,
        &["bind_address"],
        "0.0.0.0:8080",
        &server_config_label,
    )?;
    ensure_json_path_bool(
        &server_config_json,
        &["api_key_configured"],
        false,
        &server_config_label,
    )?;
    ensure_json_path_bool(
        &server_config_json,
        &["bundle_output_root_configured"],
        false,
        &server_config_label,
    )?;
    ensure_json_path_bool(
        &server_config_json,
        &["quarantine", "enabled"],
        false,
        &server_config_label,
    )?;
    ensure_file_lacks_phi_sentinels(&server_config_report)?;

    let proof = reports.join("first-use-by-surface.json");
    let proof_json = serde_json::json!({
        "guide": "docs/guides/first-use-by-surface.md",
        "source_checkout_routes": [
            "rust_user_journey",
            "cli_doctor",
            "cli_profile_lint",
            "cli_validation_report",
            "cli_corpus_summary",
            "server_print_config"
        ],
        "delegated_routes": [
            {
                "surface": "python",
                "proof": "cargo +1.95.0 run -p xtask -- python-local-wheel-proof",
                "reason": "public TestPyPI/PyPI proof remains blocked by issue #563"
            }
        ],
        "non_claims": [
            "no TestPyPI upload",
            "no PyPI upload",
            "no npm package",
            "no new crates.io release"
        ]
    });
    fs::write(&proof, serde_json::to_string_pretty(&proof_json)?)?;
    ensure_file_lacks_phi_sentinels(&proof)?;

    println!(
        "First Use By Surface guide recipe wrote {}",
        guide_root.display()
    );
    Ok(())
}

fn check_vendor_upgrade_diff_guide() -> Result<()> {
    println!("Checking Vendor Upgrade Diff guide recipe...");
    let workspace_root = env::current_dir()?;
    let guide_root = workspace_root.join(VENDOR_UPGRADE_DIFF_GUIDE_ROOT);
    let target_root = workspace_root.join("target");
    if !guide_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare Vendor Upgrade Diff guide output outside target/: {}",
            guide_root.display()
        ));
    }
    if guide_root.exists() {
        fs::remove_dir_all(&guide_root)?;
    }

    let before = guide_root.join("before");
    let after = guide_root.join("after");
    let reports = guide_root.join("reports");
    fs::create_dir_all(&before)?;
    fs::create_dir_all(&after)?;
    fs::create_dir_all(&reports)?;

    let profile = workspace_root.join("profiles/generic.yaml");
    let valid_message = workspace_root.join("test_data/valid_message.hl7");
    let invalid_message = workspace_root.join("test_data/invalid_message.hl7");
    ensure_existing_file(&profile)?;
    ensure_existing_file(&valid_message)?;
    ensure_existing_file(&invalid_message)?;

    fs::copy(&valid_message, before.join("site-a-001.hl7"))?;
    fs::copy(&invalid_message, after.join("site-a-001.hl7"))?;

    let profile_lint = reports.join("profile-lint.json");
    run_cli_guide_command(
        "Vendor Upgrade Diff profile lint",
        vec![
            "profile".to_string(),
            "lint".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&profile_lint)?,
        ],
    )?;
    let lint = read_json_file(&profile_lint)?;
    let lint_label = path_to_arg(&profile_lint)?;
    ensure_json_path_bool(&lint, &["valid"], true, &lint_label)?;
    ensure_json_path_u64(&lint, &["error_count"], 0, &lint_label)?;
    ensure_json_path_u64(&lint, &["warning_count"], 0, &lint_label)?;

    let before_summary = reports.join("before-summary.json");
    run_cli_guide_command(
        "Vendor Upgrade Diff before summary",
        vec![
            "corpus".to_string(),
            "summarize".to_string(),
            path_to_arg(&before)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&before_summary)?,
        ],
    )?;
    let before_summary_json = read_json_file(&before_summary)?;
    let before_summary_label = path_to_arg(&before_summary)?;
    ensure_vendor_upgrade_summary(&before_summary_json, &before_summary_label)?;

    let after_summary = reports.join("after-summary.json");
    run_cli_guide_command(
        "Vendor Upgrade Diff after summary",
        vec![
            "corpus".to_string(),
            "summarize".to_string(),
            path_to_arg(&after)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&after_summary)?,
        ],
    )?;
    let after_summary_json = read_json_file(&after_summary)?;
    let after_summary_label = path_to_arg(&after_summary)?;
    ensure_vendor_upgrade_summary(&after_summary_json, &after_summary_label)?;

    let before_fingerprint = reports.join("before-fingerprint.json");
    run_cli_guide_command(
        "Vendor Upgrade Diff before fingerprint",
        vec![
            "corpus".to_string(),
            "fingerprint".to_string(),
            path_to_arg(&before)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&before_fingerprint)?,
        ],
    )?;
    let before_fingerprint_json = read_json_file(&before_fingerprint)?;
    let before_fingerprint_label = path_to_arg(&before_fingerprint)?;
    ensure_json_path_string(
        &before_fingerprint_json,
        &["fingerprint_version"],
        "1",
        &before_fingerprint_label,
    )?;
    ensure_json_path_u64(
        &before_fingerprint_json,
        &["message_count"],
        1,
        &before_fingerprint_label,
    )?;
    ensure_json_path_u64(
        &before_fingerprint_json,
        &["parse_error_count"],
        0,
        &before_fingerprint_label,
    )?;
    ensure_empty_json_array(
        &before_fingerprint_json,
        &["validation_issue_code_counts"],
        &before_fingerprint_label,
    )?;
    let profile_hash = json_path(
        &before_fingerprint_json,
        &["profile", "sha256"],
        &before_fingerprint_label,
    )?
    .as_str()
    .ok_or_else(|| anyhow!("{before_fingerprint_label} profile.sha256 was not a string"))?
    .to_string();

    let after_fingerprint = reports.join("after-fingerprint.json");
    run_cli_guide_command(
        "Vendor Upgrade Diff after fingerprint",
        vec![
            "corpus".to_string(),
            "fingerprint".to_string(),
            path_to_arg(&after)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&after_fingerprint)?,
        ],
    )?;
    let after_fingerprint_json = read_json_file(&after_fingerprint)?;
    let after_fingerprint_label = path_to_arg(&after_fingerprint)?;
    ensure_json_path_string(
        &after_fingerprint_json,
        &["fingerprint_version"],
        "1",
        &after_fingerprint_label,
    )?;
    ensure_json_path_u64(
        &after_fingerprint_json,
        &["message_count"],
        1,
        &after_fingerprint_label,
    )?;
    ensure_json_path_u64(
        &after_fingerprint_json,
        &["parse_error_count"],
        0,
        &after_fingerprint_label,
    )?;
    ensure_json_object_array_contains_string_u64_field(
        &after_fingerprint_json,
        &["validation_issue_code_counts"],
        "value",
        "value_not_in_set",
        "count",
        1,
        &after_fingerprint_label,
    )?;
    ensure_json_path_string(
        &after_fingerprint_json,
        &["profile", "sha256"],
        profile_hash.as_str(),
        &after_fingerprint_label,
    )?;

    let corpus_diff = reports.join("corpus-diff.json");
    run_cli_guide_command(
        "Vendor Upgrade Diff corpus diff",
        vec![
            "corpus".to_string(),
            "diff".to_string(),
            path_to_arg(&before)?,
            path_to_arg(&after)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&corpus_diff)?,
        ],
    )?;
    let diff = read_json_file(&corpus_diff)?;
    let diff_label = path_to_arg(&corpus_diff)?;
    ensure_json_path_string(&diff, &["diff_version"], "1", &diff_label)?;
    ensure_json_path_u64(&diff, &["parse_error_count", "before"], 0, &diff_label)?;
    ensure_json_path_u64(&diff, &["parse_error_count", "after"], 0, &diff_label)?;
    ensure_json_path_u64(&diff, &["parse_error_count", "delta"], 0, &diff_label)?;
    ensure_json_object_array_contains_string_i64_field(
        &diff,
        &["field_presence"],
        "path",
        "PID.5",
        "message_count_delta",
        -1,
        &diff_label,
    )?;
    ensure_json_object_array_contains_string_i64_field(
        &diff,
        &["validation_issue_code_counts"],
        "value",
        "value_not_in_set",
        "delta",
        1,
        &diff_label,
    )?;
    ensure_file_lacks_phi_sentinels(&corpus_diff)?;

    println!(
        "Vendor Upgrade Diff guide recipe wrote {}",
        guide_root.display()
    );
    Ok(())
}

fn check_operator_error_guidance_guide() -> Result<()> {
    println!("Checking Operator Error Guidance recipe...");
    let workspace_root = env::current_dir()?;
    let guide_root = workspace_root.join(OPERATOR_ERROR_GUIDANCE_ROOT);
    let target_root = workspace_root.join("target");
    if !guide_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare operator error guidance output outside target/: {}",
            guide_root.display()
        ));
    }
    if guide_root.exists() {
        fs::remove_dir_all(&guide_root)?;
    }

    let reports = guide_root.join("reports");
    fs::create_dir_all(&reports)?;

    let server_error_tests: &[(&str, &[&str])] = &[
        (
            "REST parse safe error shape",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "parse_endpoint_test",
                "test_parse_malformed_message_returns_error",
                "--locked",
            ],
        ),
        (
            "REST validate parse safe error shape",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_endpoint_test",
                "test_validate_malformed_message_returns_error",
                "--locked",
            ],
        ),
        (
            "REST profile-load safe error shape",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_endpoint_test",
                "test_validate_invalid_profile_yaml_returns_error",
                "--locked",
            ],
        ),
        (
            "REST unsafe bundle id safe error shape",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "bundle_endpoint_test",
                "test_bundle_endpoint_rejects_unsafe_bundle_id_without_writing",
                "--locked",
            ],
        ),
        (
            "REST missing bundle root safe error shape",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "bundle_endpoint_test",
                "test_bundle_endpoint_fails_closed_without_configured_output_root",
                "--locked",
            ],
        ),
    ];
    for (label, args) in server_error_tests {
        println!("Checking {label}...");
        run_command("cargo", args)?;
    }

    let message = workspace_root.join("test_data/invalid_message.hl7");
    let profile = workspace_root.join("profiles/generic.yaml");
    ensure_existing_file(&message)?;
    ensure_existing_file(&profile)?;

    let cli_validation = reports.join("cli-validation-report.json");
    run_cli_guide_command_allow_codes(
        "operator error CLI validation report",
        vec![
            "val".to_string(),
            path_to_arg(&message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&cli_validation)?,
        ],
        &[0, 1],
    )?;
    let validation = read_json_file(&cli_validation)?;
    let validation_label = path_to_arg(&cli_validation)?;
    ensure_json_path_bool(&validation, &["valid"], false, &validation_label)?;
    ensure_json_path_u64(&validation, &["issue_count"], 1, &validation_label)?;
    ensure_json_object_array_contains_fields(
        &validation,
        &["issues"],
        &[
            ("code", "value_not_in_set"),
            ("path", "PID.8"),
            ("severity", "error"),
        ],
        &validation_label,
    )?;
    ensure_file_lacks_phi_sentinels(&cli_validation)?;

    let summary = reports.join("operator-error-guidance.json");
    let summary_json = serde_json::json!({
        "guide": "docs/guides/operator-error-guidance.md",
        "checks": [
            "rest_parse_safe_error_shape",
            "rest_validate_parse_safe_error_shape",
            "rest_profile_load_safe_error_shape",
            "rest_unsafe_bundle_id_safe_error_shape",
            "rest_missing_bundle_root_safe_error_shape",
            "cli_validation_issue_report"
        ],
        "verified_fields": [
            "code",
            "safe_detail",
            "location",
            "suggested_next_action",
            "valid",
            "issue_count",
            "issues.code",
            "issues.path",
            "issues.severity"
        ],
        "non_claims": [
            "no TestPyPI upload",
            "no PyPI upload",
            "no npm package",
            "no new crates.io release"
        ]
    });
    fs::write(&summary, serde_json::to_string_pretty(&summary_json)?)?;
    ensure_file_lacks_phi_sentinels(&summary)?;

    println!(
        "Operator Error Guidance smoke wrote {}",
        guide_root.display()
    );
    Ok(())
}

fn check_full_evidence_receipt_cli_recipe() -> Result<()> {
    println!("Checking Full Evidence Receipt Path CLI recipe...");
    let workspace_root = env::current_dir()?;
    let receipt_root = workspace_root.join(FIRST_USE_RECEIPT_ROOT);
    let target_root = workspace_root.join("target");
    if !receipt_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare first-use receipt outside target/: {}",
            receipt_root.display()
        ));
    }
    if receipt_root.exists() {
        fs::remove_dir_all(&receipt_root)?;
    }

    let reports = receipt_root.join("reports");
    fs::create_dir_all(&reports)?;

    let policy = receipt_root.join("safe-analysis.toml");
    fs::write(&policy, FIRST_USE_REDACTION_POLICY)?;

    let message = workspace_root.join("test_data/invalid_message.hl7");
    let profile = workspace_root.join("profiles/generic.yaml");
    let validation_report = reports.join("validation-report.json");
    let redaction_preview = reports.join("redaction-preview.json");
    let bundle = receipt_root.join("issue-bundle");
    let bundle_summary = reports.join("bundle-summary.json");
    let replay_report = reports.join("replay-report.json");

    ensure_existing_file(&message)?;
    ensure_existing_file(&profile)?;

    let doctor = run_cli_guide_command_capture(
        "CLI doctor",
        vec![
            "doctor".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let doctor_json: serde_json::Value = serde_json::from_str(&doctor)?;
    ensure_json_has_key(&doctor_json, "version", "doctor output")?;
    ensure_json_has_key(&doctor_json, "checks", "doctor output")?;

    run_cli_guide_command_allow_codes(
        "CLI validation report",
        vec![
            "val".to_string(),
            path_to_arg(&message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&validation_report)?,
        ],
        &[0, 1],
    )?;
    let validation = read_json_file(&validation_report)?;
    ensure_json_has_key(&validation, "valid", &path_to_arg(&validation_report)?)?;
    ensure_json_bool(
        &validation,
        "valid",
        false,
        &path_to_arg(&validation_report)?,
    )?;
    ensure_json_has_key(
        &validation,
        "issue_count",
        &path_to_arg(&validation_report)?,
    )?;

    run_cli_guide_command(
        "CLI redaction preview",
        vec![
            "redact".to_string(),
            path_to_arg(&message)?,
            "--policy".to_string(),
            path_to_arg(&policy)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&redaction_preview)?,
        ],
    )?;
    let redaction = read_json_file(&redaction_preview)?;
    ensure_json_has_key(&redaction, "receipt", &path_to_arg(&redaction_preview)?)?;
    ensure_file_lacks_phi_sentinels(&redaction_preview)?;

    run_cli_guide_command(
        "CLI support bundle",
        vec![
            "support-bundle".to_string(),
            path_to_arg(&message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--redact-policy".to_string(),
            path_to_arg(&policy)?,
            "--out".to_string(),
            path_to_arg(&bundle)?,
            "--output".to_string(),
            path_to_arg(&bundle_summary)?,
        ],
    )?;
    let summary = read_json_file(&bundle_summary)?;
    ensure_json_has_key(&summary, "validation_valid", &path_to_arg(&bundle_summary)?)?;
    ensure_json_has_key(
        &summary,
        "redaction_phi_removed",
        &path_to_arg(&bundle_summary)?,
    )?;

    for artifact in [
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "redaction-receipt.json",
        "environment.json",
        "manifest.json",
        "README.md",
        "SAFE-SHARING.md",
        "replay.ps1",
        "replay.sh",
    ] {
        let path = bundle.join(artifact);
        ensure_existing_file(&path)?;
        ensure_file_lacks_phi_sentinels(&path)?;
    }

    run_cli_guide_command(
        "CLI replay report",
        vec![
            "replay".to_string(),
            path_to_arg(&bundle)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&replay_report)?,
        ],
    )?;
    let replay = read_json_file(&replay_report)?;
    ensure_json_bool(&replay, "reproduced", true, &path_to_arg(&replay_report)?)?;
    ensure_file_lacks_phi_sentinels(&replay_report)?;

    println!(
        "Full Evidence Receipt Path CLI recipe wrote {}",
        receipt_root.display()
    );
    Ok(())
}

fn check_safe_support_bundle_guide() -> Result<()> {
    println!("Checking Safe Support Bundle guide recipe...");
    let workspace_root = env::current_dir()?;
    let guide_root = workspace_root.join(SAFE_SUPPORT_BUNDLE_GUIDE_ROOT);
    let target_root = workspace_root.join("target");
    if !guide_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare safe support bundle outside target/: {}",
            guide_root.display()
        ));
    }
    if guide_root.exists() {
        fs::remove_dir_all(&guide_root)?;
    }

    let reports = guide_root.join("reports");
    fs::create_dir_all(&reports)?;

    let policy = guide_root.join("safe-analysis.toml");
    fs::write(&policy, SAFE_SUPPORT_BUNDLE_REDACTION_POLICY)?;

    let message = workspace_root.join("test_data/invalid_message.hl7");
    let profile = workspace_root.join("profiles/generic.yaml");
    let redaction_preview = reports.join("redaction-preview.json");
    let bundle = guide_root.join("issue-bundle");
    let bundle_summary = reports.join("bundle-summary.json");
    let replay_report = reports.join("replay-report.json");

    ensure_existing_file(&message)?;
    ensure_existing_file(&profile)?;

    run_cli_guide_command(
        "safe support bundle redaction preview",
        vec![
            "redact".to_string(),
            path_to_arg(&message)?,
            "--policy".to_string(),
            path_to_arg(&policy)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&redaction_preview)?,
        ],
    )?;
    let redaction = read_json_file(&redaction_preview)?;
    let redaction_label = path_to_arg(&redaction_preview)?;
    ensure_json_path_bool(
        &redaction,
        &["receipt", "phi_removed"],
        true,
        &redaction_label,
    )?;
    for (path, action) in [
        ("PID.3", "hash"),
        ("PID.5", "drop"),
        ("PID.7", "drop"),
        ("PID.8", "retain"),
    ] {
        ensure_json_object_array_contains_fields(
            &redaction,
            &["receipt", "actions"],
            &[("path", path), ("action", action)],
            &redaction_label,
        )?;
    }
    ensure_file_lacks_phi_sentinels(&redaction_preview)?;

    run_cli_guide_command(
        "safe support bundle creation",
        vec![
            "support-bundle".to_string(),
            path_to_arg(&message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--redact-policy".to_string(),
            path_to_arg(&policy)?,
            "--out".to_string(),
            path_to_arg(&bundle)?,
            "--output".to_string(),
            path_to_arg(&bundle_summary)?,
        ],
    )?;
    let summary = read_json_file(&bundle_summary)?;
    let summary_label = path_to_arg(&bundle_summary)?;
    ensure_json_path_string(&summary, &["bundle_version"], "1", &summary_label)?;
    ensure_json_path_bool(&summary, &["validation_valid"], false, &summary_label)?;
    ensure_json_path_u64(&summary, &["validation_issue_count"], 1, &summary_label)?;
    ensure_json_path_bool(&summary, &["redaction_phi_removed"], true, &summary_label)?;
    for artifact in [
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "profile.yaml",
        "redaction-receipt.json",
        "environment.json",
        "replay.sh",
        "replay.ps1",
        "README.md",
        "SAFE-SHARING.md",
        "manifest.json",
    ] {
        ensure_json_array_contains_string(&summary, &["artifacts"], artifact, &summary_label)?;
        let artifact_path = bundle.join(artifact);
        ensure_existing_file(&artifact_path)?;
        ensure_file_lacks_phi_sentinels(&artifact_path)?;
    }
    ensure_file_lacks_phi_sentinels(&bundle_summary)?;

    let validation_report = bundle.join("validation-report.json");
    let validation = read_json_file(&validation_report)?;
    let validation_label = path_to_arg(&validation_report)?;
    ensure_json_path_bool(&validation, &["valid"], false, &validation_label)?;
    ensure_json_path_u64(&validation, &["issue_count"], 1, &validation_label)?;
    ensure_json_object_array_contains_fields(
        &validation,
        &["issues"],
        &[
            ("code", "value_not_in_set"),
            ("path", "PID.8"),
            ("severity", "error"),
        ],
        &validation_label,
    )?;

    run_cli_guide_command(
        "safe support bundle replay",
        vec![
            "replay".to_string(),
            path_to_arg(&bundle)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&replay_report)?,
        ],
    )?;
    let replay = read_json_file(&replay_report)?;
    let replay_label = path_to_arg(&replay_report)?;
    ensure_json_path_string(&replay, &["replay_version"], "1", &replay_label)?;
    ensure_json_path_string(&replay, &["bundle_version"], "1", &replay_label)?;
    ensure_json_path_bool(&replay, &["reproduced"], true, &replay_label)?;
    ensure_json_path_bool(&replay, &["validation_valid"], false, &replay_label)?;
    ensure_json_path_u64(&replay, &["validation_issue_count"], 1, &replay_label)?;
    for check_name in ["manifest-hashes", "report-match", "environment-match"] {
        ensure_json_object_array_contains_fields(
            &replay,
            &["checks"],
            &[("name", check_name), ("status", "pass")],
            &replay_label,
        )?;
    }
    ensure_file_lacks_phi_sentinels(&replay_report)?;

    println!(
        "Safe Support Bundle guide recipe wrote {}",
        guide_root.display()
    );
    Ok(())
}

fn check_evidence_artifacts_guide() -> Result<()> {
    println!("Checking Evidence Artifacts For Operators guide recipe...");
    let workspace_root = env::current_dir()?;
    let guide_root = workspace_root.join(EVIDENCE_ARTIFACTS_GUIDE_ROOT);
    let target_root = workspace_root.join("target");
    if !guide_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare evidence artifact guide output outside target/: {}",
            guide_root.display()
        ));
    }
    if guide_root.exists() {
        fs::remove_dir_all(&guide_root)?;
    }

    let reports = guide_root.join("reports");
    let fixtures = guide_root.join("profile-fixtures");
    let valid_fixtures = fixtures.join("valid");
    let invalid_fixtures = fixtures.join("invalid");
    fs::create_dir_all(&reports)?;
    fs::create_dir_all(&valid_fixtures)?;
    fs::create_dir_all(&invalid_fixtures)?;

    let policy = guide_root.join("safe-analysis.toml");
    fs::write(&policy, SAFE_SUPPORT_BUNDLE_REDACTION_POLICY)?;

    let profile = workspace_root.join("profiles/generic.yaml");
    let valid_message = workspace_root.join("test_data/valid_message.hl7");
    let invalid_message = workspace_root.join("test_data/invalid_message.hl7");
    let dirty_before = workspace_root.join("test_data/dirty-real-world/before");
    let dirty_after = workspace_root.join("test_data/dirty-real-world/after");
    ensure_existing_file(&profile)?;
    ensure_existing_file(&valid_message)?;
    ensure_existing_file(&invalid_message)?;
    let dirty_before_file_count = count_regular_files(&dirty_before)?;
    let dirty_after_file_count = count_regular_files(&dirty_after)?;

    fs::copy(&valid_message, valid_fixtures.join("valid-message.hl7"))?;
    fs::copy(
        &invalid_message,
        invalid_fixtures.join("invalid-message.hl7"),
    )?;

    let doctor = run_cli_guide_command_capture(
        "operator artifacts doctor report",
        vec![
            "doctor".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ],
    )?;
    let doctor_json: serde_json::Value = serde_json::from_str(&doctor)?;
    ensure_json_has_key(&doctor_json, "version", "operator artifacts doctor report")?;
    ensure_json_has_key(&doctor_json, "checks", "operator artifacts doctor report")?;

    let profile_lint = reports.join("profile-lint.json");
    run_cli_guide_command(
        "operator artifacts profile lint report",
        vec![
            "profile".to_string(),
            "lint".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&profile_lint)?,
        ],
    )?;
    let lint = read_json_file(&profile_lint)?;
    let lint_label = path_to_arg(&profile_lint)?;
    ensure_json_path_bool(&lint, &["valid"], true, &lint_label)?;
    ensure_json_path_u64(&lint, &["error_count"], 0, &lint_label)?;
    ensure_json_path_u64(&lint, &["issue_count"], 0, &lint_label)?;

    let profile_explain = reports.join("profile-explain.json");
    run_cli_guide_command(
        "operator artifacts profile explain report",
        vec![
            "profile".to_string(),
            "explain".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&profile_explain)?,
        ],
    )?;
    let explain = read_json_file(&profile_explain)?;
    let explain_label = path_to_arg(&profile_explain)?;
    ensure_json_path_string(&explain, &["message_structure"], "GENERIC", &explain_label)?;
    ensure_json_path_u64(
        &explain,
        &["summary", "required_field_count"],
        1,
        &explain_label,
    )?;
    ensure_json_object_array_contains_fields(
        &explain,
        &["required_fields"],
        &[("path", "PID.3")],
        &explain_label,
    )?;
    ensure_json_object_array_contains_fields(
        &explain,
        &["value_sets"],
        &[("path", "PID.8"), ("name", "HL70001")],
        &explain_label,
    )?;

    let profile_test = reports.join("profile-test.json");
    run_cli_guide_command(
        "operator artifacts profile test report",
        vec![
            "profile".to_string(),
            "test".to_string(),
            path_to_arg(&profile)?,
            path_to_arg(&fixtures)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&profile_test)?,
        ],
    )?;
    let test_report = read_json_file(&profile_test)?;
    let test_label = path_to_arg(&profile_test)?;
    ensure_json_path_bool(&test_report, &["valid"], true, &test_label)?;
    ensure_json_path_u64(&test_report, &["case_count"], 2, &test_label)?;
    ensure_json_path_u64(&test_report, &["failed_count"], 0, &test_label)?;

    let validation_report = reports.join("validation-report.json");
    run_cli_guide_command_allow_codes(
        "operator artifacts validation report",
        vec![
            "val".to_string(),
            path_to_arg(&invalid_message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--report".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&validation_report)?,
        ],
        &[0, 1],
    )?;
    let validation = read_json_file(&validation_report)?;
    let validation_label = path_to_arg(&validation_report)?;
    ensure_json_path_bool(&validation, &["valid"], false, &validation_label)?;
    ensure_json_path_u64(&validation, &["issue_count"], 1, &validation_label)?;
    ensure_json_object_array_contains_fields(
        &validation,
        &["issues"],
        &[
            ("code", "value_not_in_set"),
            ("path", "PID.8"),
            ("severity", "error"),
        ],
        &validation_label,
    )?;
    ensure_file_lacks_phi_sentinels(&validation_report)?;

    let corpus_summary = reports.join("corpus-summary.json");
    run_cli_guide_command(
        "operator artifacts corpus summary",
        vec![
            "corpus".to_string(),
            "summarize".to_string(),
            path_to_arg(&dirty_before)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&corpus_summary)?,
        ],
    )?;
    let summary = read_json_file(&corpus_summary)?;
    let summary_label = path_to_arg(&corpus_summary)?;
    ensure_json_path_u64(&summary, &["file_count"], 2, &summary_label)?;
    ensure_json_path_u64(&summary, &["parse_error_count"], 2, &summary_label)?;
    ensure_file_lacks_phi_sentinels(&corpus_summary)?;

    let corpus_fingerprint = reports.join("corpus-fingerprint.json");
    run_cli_guide_command(
        "operator artifacts corpus fingerprint",
        vec![
            "corpus".to_string(),
            "fingerprint".to_string(),
            path_to_arg(&dirty_before)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&corpus_fingerprint)?,
        ],
    )?;
    let fingerprint = read_json_file(&corpus_fingerprint)?;
    let fingerprint_label = path_to_arg(&corpus_fingerprint)?;
    ensure_json_path_string(
        &fingerprint,
        &["fingerprint_version"],
        "1",
        &fingerprint_label,
    )?;
    ensure_json_path_u64(&fingerprint, &["file_count"], 2, &fingerprint_label)?;
    ensure_json_path_u64(&fingerprint, &["parse_error_count"], 2, &fingerprint_label)?;
    ensure_json_path_string(
        &fingerprint,
        &["profile", "version"],
        "2.5.1",
        &fingerprint_label,
    )?;
    ensure_file_lacks_phi_sentinels(&corpus_fingerprint)?;

    let corpus_diff = reports.join("corpus-diff.json");
    run_cli_guide_command(
        "operator artifacts corpus diff",
        vec![
            "corpus".to_string(),
            "diff".to_string(),
            path_to_arg(&dirty_before)?,
            path_to_arg(&dirty_after)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&corpus_diff)?,
        ],
    )?;
    let diff = read_json_file(&corpus_diff)?;
    let diff_label = path_to_arg(&corpus_diff)?;
    ensure_json_path_string(&diff, &["diff_version"], "1", &diff_label)?;
    ensure_json_path_u64(
        &diff,
        &["file_count", "before"],
        dirty_before_file_count,
        &diff_label,
    )?;
    ensure_json_path_u64(
        &diff,
        &["file_count", "after"],
        dirty_after_file_count,
        &diff_label,
    )?;
    ensure_json_path_u64(
        &diff,
        &["parse_error_count", "before"],
        dirty_before_file_count,
        &diff_label,
    )?;
    ensure_json_path_u64(
        &diff,
        &["parse_error_count", "after"],
        dirty_after_file_count,
        &diff_label,
    )?;
    ensure_file_lacks_phi_sentinels(&corpus_diff)?;

    let redaction_preview = reports.join("redaction-preview.json");
    run_cli_guide_command(
        "operator artifacts redaction preview",
        vec![
            "redact".to_string(),
            path_to_arg(&invalid_message)?,
            "--policy".to_string(),
            path_to_arg(&policy)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&redaction_preview)?,
        ],
    )?;
    let redaction = read_json_file(&redaction_preview)?;
    let redaction_label = path_to_arg(&redaction_preview)?;
    ensure_json_path_bool(
        &redaction,
        &["receipt", "phi_removed"],
        true,
        &redaction_label,
    )?;
    ensure_json_object_array_contains_fields(
        &redaction,
        &["receipt", "actions"],
        &[("path", "PID.8"), ("action", "retain")],
        &redaction_label,
    )?;
    ensure_file_lacks_phi_sentinels(&redaction_preview)?;

    let bundle = guide_root.join("issue-bundle");
    let bundle_summary = reports.join("bundle-summary.json");
    run_cli_guide_command(
        "operator artifacts support bundle",
        vec![
            "support-bundle".to_string(),
            path_to_arg(&invalid_message)?,
            "--profile".to_string(),
            path_to_arg(&profile)?,
            "--redact-policy".to_string(),
            path_to_arg(&policy)?,
            "--out".to_string(),
            path_to_arg(&bundle)?,
            "--output".to_string(),
            path_to_arg(&bundle_summary)?,
        ],
    )?;
    let bundle_summary_json = read_json_file(&bundle_summary)?;
    let bundle_summary_label = path_to_arg(&bundle_summary)?;
    ensure_json_path_bool(
        &bundle_summary_json,
        &["validation_valid"],
        false,
        &bundle_summary_label,
    )?;
    ensure_json_path_bool(
        &bundle_summary_json,
        &["redaction_phi_removed"],
        true,
        &bundle_summary_label,
    )?;
    for artifact in [
        "message.redacted.hl7",
        "validation-report.json",
        "field-paths.json",
        "redaction-receipt.json",
        "environment.json",
        "manifest.json",
        "README.md",
        "SAFE-SHARING.md",
        "replay.ps1",
        "replay.sh",
    ] {
        ensure_json_array_contains_string(
            &bundle_summary_json,
            &["artifacts"],
            artifact,
            &bundle_summary_label,
        )?;
        let artifact_path = bundle.join(artifact);
        ensure_existing_file(&artifact_path)?;
        ensure_file_lacks_phi_sentinels(&artifact_path)?;
    }
    ensure_file_lacks_phi_sentinels(&bundle_summary)?;

    let manifest = read_json_file(&bundle.join("manifest.json"))?;
    let manifest_label = path_to_arg(&bundle.join("manifest.json"))?;
    ensure_json_path_string(&manifest, &["bundle_version"], "1", &manifest_label)?;
    ensure_json_object_array_contains_fields(
        &manifest,
        &["artifacts"],
        &[
            ("role", "redacted_message"),
            ("path", "message.redacted.hl7"),
        ],
        &manifest_label,
    )?;
    ensure_json_object_array_contains_fields(
        &manifest,
        &["artifacts"],
        &[("role", "replay_shell_script"), ("path", "replay.sh")],
        &manifest_label,
    )?;

    let environment = read_json_file(&bundle.join("environment.json"))?;
    let environment_label = path_to_arg(&bundle.join("environment.json"))?;
    ensure_json_has_key(&environment, "tool_version", &environment_label)?;
    ensure_json_has_key(&environment, "input_sha256", &environment_label)?;
    ensure_json_has_key(&environment, "replay_command", &environment_label)?;

    let field_paths = read_json_file(&bundle.join("field-paths.json"))?;
    let field_paths_label = path_to_arg(&bundle.join("field-paths.json"))?;
    ensure_json_object_array_contains_fields(
        &field_paths,
        &["fields"],
        &[("canonical_path", "PID.8"), ("redaction_action", "retain")],
        &field_paths_label,
    )?;

    let replay_report = reports.join("replay-report.json");
    run_cli_guide_command(
        "operator artifacts replay report",
        vec![
            "replay".to_string(),
            path_to_arg(&bundle)?,
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_to_arg(&replay_report)?,
        ],
    )?;
    let replay = read_json_file(&replay_report)?;
    let replay_label = path_to_arg(&replay_report)?;
    ensure_json_path_bool(&replay, &["reproduced"], true, &replay_label)?;
    for check_name in ["manifest-hashes", "report-match", "environment-match"] {
        ensure_json_object_array_contains_fields(
            &replay,
            &["checks"],
            &[("name", check_name), ("status", "pass")],
            &replay_label,
        )?;
    }
    ensure_file_lacks_phi_sentinels(&replay_report)?;

    println!(
        "Evidence Artifacts For Operators guide recipe wrote {}",
        guide_root.display()
    );
    Ok(())
}

fn check_sidecar_guide() -> Result<()> {
    println!("Checking Deploy Validation Sidecar guide recipe...");
    let workspace_root = env::current_dir()?;
    let sidecar_root = workspace_root.join(SIDECAR_GUIDE_ROOT);
    let target_root = workspace_root.join("target");
    if !sidecar_root.starts_with(&target_root) {
        return Err(anyhow!(
            "refusing to prepare sidecar guide output outside target/: {}",
            sidecar_root.display()
        ));
    }
    if sidecar_root.exists() {
        fs::remove_dir_all(&sidecar_root)?;
    }

    let bundles = sidecar_root.join("bundles");
    let quarantine = sidecar_root.join("quarantine");
    let reports = sidecar_root.join("reports");
    fs::create_dir_all(&bundles)?;
    fs::create_dir_all(&quarantine)?;
    fs::create_dir_all(&reports)?;

    let config = sidecar_root.join("server.toml");
    let policy = sidecar_root.join("safe-analysis.toml");
    let port = allocate_loopback_port()?;
    let bind_addr = format!("127.0.0.1:{port}");
    let server_url = format!("http://{bind_addr}");
    fs::write(&config, sidecar_guide_config(port))?;
    fs::write(&policy, SAFE_SUPPORT_BUNDLE_REDACTION_POLICY)?;

    let profile = workspace_root.join("profiles/generic.yaml");
    ensure_existing_file(&profile)?;

    println!("Building hl7v2-server for sidecar guide smoke...");
    run_command("cargo", &["build", "-p", "hl7v2-server", "--locked"])?;
    let server_bin = built_binary_path(&workspace_root, "hl7v2-server");
    ensure_existing_file(&server_bin)?;

    let config_arg = path_to_arg(&config)?;
    let profile_arg = path_to_arg(&profile)?;
    let sidecar_env = [
        ("HL7V2_CONFIG", config_arg.as_str()),
        ("HL7V2_API_KEY", "dev-secret"),
        ("HL7V2_PROFILE_PATHS", profile_arg.as_str()),
    ];

    let public_config = run_command_capture_with_env(
        &server_bin,
        &["--print-config"],
        &sidecar_env,
        Some(&workspace_root),
    )?;
    if public_config.contains("dev-secret") {
        return Err(anyhow!("sidecar --print-config leaked the API key value"));
    }
    let public_config_json: serde_json::Value = serde_json::from_str(&public_config)?;
    ensure_json_path_string(
        &public_config_json,
        &["bind_address"],
        &bind_addr,
        "sidecar --print-config",
    )?;
    ensure_json_path_bool(
        &public_config_json,
        &["api_key_configured"],
        true,
        "sidecar --print-config",
    )?;
    ensure_json_path_bool(
        &public_config_json,
        &["bundle_output_root_configured"],
        true,
        "sidecar --print-config",
    )?;
    ensure_json_path_bool(
        &public_config_json,
        &["quarantine", "enabled"],
        true,
        "sidecar --print-config",
    )?;
    ensure_json_path_bool(
        &public_config_json,
        &["quarantine", "path_configured"],
        true,
        "sidecar --print-config",
    )?;

    ensure_tcp_port_available(&bind_addr)?;

    println!("Starting hl7v2-server sidecar on {bind_addr}...");
    let stdout = fs::File::create(sidecar_root.join("server.stdout.log"))?;
    let stderr = fs::File::create(sidecar_root.join("server.stderr.log"))?;
    let child = Command::new(&server_bin)
        .current_dir(&workspace_root)
        .env("HL7V2_CONFIG", config_arg.as_str())
        .env("HL7V2_API_KEY", "dev-secret")
        .env("HL7V2_PROFILE_PATHS", profile_arg.as_str())
        .env("RUST_LOG", "hl7v2_server=warn")
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;
    let mut sidecar = ChildGuard::new(child);
    thread::sleep(Duration::from_millis(500));
    sidecar.ensure_running("hl7v2-server sidecar", &sidecar_root)?;

    run_program_with_env_in_dir(
        "python",
        &["tests/server_smoke/smoke.py"],
        &[
            ("HL7V2_SERVER_URL", server_url.as_str()),
            ("HL7V2_API_KEY", "dev-secret"),
            ("HL7V2_SERVER_SMOKE_TIMEOUT", "45"),
        ],
        Some(&workspace_root),
    )?;
    sidecar.ensure_running("hl7v2-server sidecar", &sidecar_root)?;

    run_program_with_env_in_dir(
        "python",
        &["tests/server_smoke/guide_quarantine.py"],
        &[
            ("HL7V2_SERVER_URL", server_url.as_str()),
            ("HL7V2_API_KEY", "dev-secret"),
            ("HL7V2_SIDECAR_GUIDE_ROOT", SIDECAR_GUIDE_ROOT),
            ("HL7V2_SERVER_SMOKE_TIMEOUT", "45"),
        ],
        Some(&workspace_root),
    )?;
    sidecar.ensure_running("hl7v2-server sidecar", &sidecar_root)?;
    sidecar.stop()?;

    println!(
        "Deploy Validation Sidecar guide smoke wrote {}",
        sidecar_root.display()
    );
    Ok(())
}

fn run_cli_guide_command(label: &str, args: Vec<String>) -> Result<()> {
    println!("Checking {label}...");
    run_command_owned("cargo", &cargo_run_hl7v2_cli_args(args))
}

fn run_cli_guide_command_allow_codes(
    label: &str,
    args: Vec<String>,
    allowed_codes: &[i32],
) -> Result<()> {
    println!("Checking {label}...");
    let cargo_args = cargo_run_hl7v2_cli_args(args);
    let output = Command::new("cargo").args(&cargo_args).output()?;
    let code = output.status.code();
    if !matches!(code, Some(code) if allowed_codes.contains(&code)) {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}\nstdout:\n{}\nstderr:\n{}",
            "cargo",
            cargo_args.join(" "),
            code,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if code != Some(0) {
        println!("{label} completed with expected exit code {code:?}; verifying receipt file.");
    }
    Ok(())
}

fn run_cli_guide_command_capture(label: &str, args: Vec<String>) -> Result<String> {
    println!("Checking {label}...");
    run_command_capture_owned("cargo", &cargo_run_hl7v2_cli_args(args))
}

fn run_cli_guide_command_capture_allow_codes(
    label: &str,
    args: Vec<String>,
    allowed_codes: &[i32],
) -> Result<String> {
    println!("Checking {label}...");
    let cargo_args = cargo_run_hl7v2_cli_args(args);
    let output = Command::new("cargo").args(&cargo_args).output()?;
    let code = output.status.code();
    if !matches!(code, Some(code) if allowed_codes.contains(&code)) {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}\nstdout:\n{}\nstderr:\n{}",
            "cargo",
            cargo_args.join(" "),
            code,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if code != Some(0) {
        println!("{label} completed with expected exit code {code:?}; verifying stdout receipt.");
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn cargo_run_hl7v2_cli_args(args: Vec<String>) -> Vec<String> {
    let mut cargo_args = vec![
        "run".to_string(),
        "--quiet".to_string(),
        "-p".to_string(),
        "hl7v2-cli".to_string(),
        "--".to_string(),
    ];
    cargo_args.extend(args);
    cargo_args
}

fn built_binary_path(workspace_root: &Path, name: &str) -> PathBuf {
    let mut path = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    path.push("debug");
    if cfg!(windows) {
        path.push(format!("{name}.exe"));
    } else {
        path.push(name);
    }
    path
}

fn ensure_tcp_port_available(bind_addr: &str) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).map_err(|error| {
        anyhow!("sidecar guide smoke requires {bind_addr}, but it is not available: {error}")
    })?;
    drop(listener);
    Ok(())
}

fn allocate_loopback_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| anyhow!("failed to allocate sidecar guide smoke port: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| anyhow!("failed to read sidecar guide smoke port: {error}"))?
        .port();
    drop(listener);
    Ok(port)
}

fn run_command_capture_with_env(
    cmd: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
    cwd: Option<&Path>,
) -> Result<String> {
    let mut command = Command::new(cmd);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command.output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command '{} {}' failed with exit code: {:?}\nstdout:\n{}\nstderr:\n{}",
            cmd.display(),
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8(output.stdout)?)
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
            child.wait()?;
        }
        Ok(())
    }

    fn ensure_running(&mut self, label: &str, log_root: &Path) -> Result<()> {
        let Some(child) = self.child.as_mut() else {
            return Err(anyhow!("{label} process is no longer tracked"));
        };
        if let Some(status) = child.try_wait()? {
            return Err(anyhow!(
                "{label} exited before the smoke completed with status {status}; stdout:\n{}\nstderr:\n{}",
                read_optional_log(&log_root.join("server.stdout.log"))?,
                read_optional_log(&log_root.join("server.stderr.log"))?
            ));
        }
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if matches!(child.try_wait(), Ok(None)) && child.kill().is_err() {
                return;
            }
            drop(child.wait());
        }
    }
}

fn read_optional_log(path: &Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(content) if content.is_empty() => Ok("<empty>".to_string()),
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok("<missing>".to_string()),
        Err(error) => Err(error.into()),
    }
}

fn workspace_crate_version(crate_name: &str) -> Result<String> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == crate_name)
        .ok_or_else(|| anyhow!("cargo metadata did not include crate `{crate_name}`"))?;
    Ok(package.version.to_string())
}

fn ensure_existing_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(anyhow!("expected file to exist: {}", path.display()));
    }
    Ok(())
}

fn count_regular_files(path: &Path) -> Result<u64> {
    if !path.is_dir() {
        return Err(anyhow!("expected directory to exist: {}", path.display()));
    }

    let mut count = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            count = count
                .checked_add(1)
                .ok_or_else(|| anyhow!("too many files to count under {}", path.display()))?;
        }
    }
    Ok(count)
}

fn read_json_file(path: &Path) -> Result<serde_json::Value> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|error| anyhow!("{} is not valid JSON: {error}", path.display()))
}

fn ensure_json_has_key(value: &serde_json::Value, key: &str, label: &str) -> Result<()> {
    if value.get(key).is_none() {
        return Err(anyhow!("{label} did not include expected JSON key `{key}`"));
    }
    Ok(())
}

fn ensure_json_bool(
    value: &serde_json::Value,
    key: &str,
    expected: bool,
    label: &str,
) -> Result<()> {
    match value.get(key).and_then(serde_json::Value::as_bool) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(anyhow!(
            "{label} JSON key `{key}` was {actual}, expected {expected}"
        )),
        None => Err(anyhow!("{label} did not include boolean JSON key `{key}`")),
    }
}

fn json_path<'a>(
    value: &'a serde_json::Value,
    path: &[&str],
    label: &str,
) -> Result<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(key).ok_or_else(|| {
            anyhow!(
                "{label} did not include expected JSON path `{}`",
                path.join(".")
            )
        })?;
    }
    Ok(current)
}

fn ensure_json_path_bool(
    value: &serde_json::Value,
    path: &[&str],
    expected: bool,
    label: &str,
) -> Result<()> {
    match json_path(value, path, label)?.as_bool() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(anyhow!(
            "{label} JSON path `{}` was {actual}, expected {expected}",
            path.join(".")
        )),
        None => Err(anyhow!(
            "{label} did not include boolean JSON path `{}`",
            path.join(".")
        )),
    }
}

fn ensure_json_path_string(
    value: &serde_json::Value,
    path: &[&str],
    expected: &str,
    label: &str,
) -> Result<()> {
    match json_path(value, path, label)?.as_str() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(anyhow!(
            "{label} JSON path `{}` was `{actual}`, expected `{expected}`",
            path.join(".")
        )),
        None => Err(anyhow!(
            "{label} did not include string JSON path `{}`",
            path.join(".")
        )),
    }
}

fn ensure_json_path_u64(
    value: &serde_json::Value,
    path: &[&str],
    expected: u64,
    label: &str,
) -> Result<()> {
    match json_path(value, path, label)?.as_u64() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(anyhow!(
            "{label} JSON path `{}` was {actual}, expected {expected}",
            path.join(".")
        )),
        None => Err(anyhow!(
            "{label} did not include unsigned integer JSON path `{}`",
            path.join(".")
        )),
    }
}

fn ensure_json_array_contains_string(
    value: &serde_json::Value,
    path: &[&str],
    expected: &str,
    label: &str,
) -> Result<()> {
    let array = json_path(value, path, label)?
        .as_array()
        .ok_or_else(|| anyhow!("{label} JSON path `{}` was not an array", path.join(".")))?;
    if array
        .iter()
        .any(|value| value.as_str().is_some_and(|actual| actual == expected))
    {
        return Ok(());
    }
    Err(anyhow!(
        "{label} JSON array `{}` did not contain `{expected}`",
        path.join(".")
    ))
}

fn ensure_empty_json_array(value: &serde_json::Value, path: &[&str], label: &str) -> Result<()> {
    let array = json_path(value, path, label)?
        .as_array()
        .ok_or_else(|| anyhow!("{label} JSON path `{}` was not an array", path.join(".")))?;
    if array.is_empty() {
        return Ok(());
    }
    Err(anyhow!(
        "{label} JSON array `{}` contained {} item(s), expected empty",
        path.join("."),
        array.len()
    ))
}

fn ensure_vendor_upgrade_summary(summary: &serde_json::Value, label: &str) -> Result<()> {
    ensure_json_path_u64(summary, &["file_count"], 1, label)?;
    ensure_json_path_u64(summary, &["message_count"], 1, label)?;
    ensure_json_path_u64(summary, &["parse_error_count"], 0, label)?;
    ensure_json_object_array_contains_string_u64_field(
        summary,
        &["message_types"],
        "value",
        "ADT^A01^ADT_A01",
        "count",
        1,
        label,
    )
}

fn ensure_json_object_array_contains_fields(
    value: &serde_json::Value,
    path: &[&str],
    fields: &[(&str, &str)],
    label: &str,
) -> Result<()> {
    let array = json_path(value, path, label)?
        .as_array()
        .ok_or_else(|| anyhow!("{label} JSON path `{}` was not an array", path.join(".")))?;
    let found = array.iter().any(|item| {
        fields.iter().all(|(key, expected)| {
            item.get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|actual| actual == *expected)
        })
    });
    if found {
        return Ok(());
    }
    let expected = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ");
    Err(anyhow!(
        "{label} JSON array `{}` did not contain object with {expected}",
        path.join(".")
    ))
}

fn ensure_json_object_array_contains_string_u64_field(
    value: &serde_json::Value,
    path: &[&str],
    string_key: &str,
    string_value: &str,
    u64_key: &str,
    u64_value: u64,
    label: &str,
) -> Result<()> {
    let array = json_path(value, path, label)?
        .as_array()
        .ok_or_else(|| anyhow!("{label} JSON path `{}` was not an array", path.join(".")))?;
    if array.iter().any(|item| {
        item.get(string_key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|actual| actual == string_value)
            && item
                .get(u64_key)
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|actual| actual == u64_value)
    }) {
        return Ok(());
    }
    Err(anyhow!(
        "{label} JSON array `{}` did not contain object with {string_key}={string_value}, {u64_key}={u64_value}",
        path.join(".")
    ))
}

fn ensure_json_object_array_contains_string_i64_field(
    value: &serde_json::Value,
    path: &[&str],
    string_key: &str,
    string_value: &str,
    i64_key: &str,
    i64_value: i64,
    label: &str,
) -> Result<()> {
    let array = json_path(value, path, label)?
        .as_array()
        .ok_or_else(|| anyhow!("{label} JSON path `{}` was not an array", path.join(".")))?;
    if array.iter().any(|item| {
        item.get(string_key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|actual| actual == string_value)
            && item
                .get(i64_key)
                .and_then(serde_json::Value::as_i64)
                .is_some_and(|actual| actual == i64_value)
    }) {
        return Ok(());
    }
    Err(anyhow!(
        "{label} JSON array `{}` did not contain object with {string_key}={string_value}, {i64_key}={i64_value}",
        path.join(".")
    ))
}

fn ensure_file_lacks_phi_sentinels(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)?;
    for sentinel in GUIDE_PHI_SENTINELS {
        if text.contains(sentinel) {
            return Err(anyhow!(
                "{} leaked guide PHI sentinel `{sentinel}`",
                path.display()
            ));
        }
    }
    Ok(())
}

fn check_safe_error_phi_parity(include_python: bool) -> Result<()> {
    println!("🔎 Checking safe-error and PHI parity acceptance...");

    let commands: &[(&str, &[&str])] = &[
        (
            "Rust library safe-error/PHI fixture tests",
            &[
                "test",
                "-p",
                "hl7v2",
                "--test",
                "safe_error_phi_parity",
                "--all-features",
                "--locked",
            ],
        ),
        (
            "CLI parse safe-error fixture",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "test_parse_safe_error_does_not_emit_manifest_phi_sentinels",
                "--locked",
            ],
        ),
        (
            "CLI redaction PHI fixture",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "test_redact_json_does_not_emit_phi_leak_sentinels_or_paths",
                "--locked",
            ],
        ),
        (
            "REST parse safe-error fixture",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "parse_endpoint_test",
                "test_parse_malformed_message_returns_error",
                "--locked",
            ],
        ),
        (
            "REST invalid-profile safe-error fixture",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_endpoint_test",
                "test_validate_invalid_profile_yaml_returns_error",
                "--locked",
            ],
        ),
        (
            "REST validate-redacted PHI fixture",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_redacted_endpoint_test",
                "test_validate_redacted_returns_report_receipt_and_redacted_hl7_without_phi",
                "--locked",
            ],
        ),
        (
            "gRPC parse safe-error fixture",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "test_grpc_parse_invalid_hl7_returns_parse_error",
                "--locked",
            ],
        ),
        (
            "gRPC invalid-profile safe-error fixture",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "test_grpc_validate_invalid_profile_returns_invalid_argument",
                "--locked",
            ],
        ),
        (
            "gRPC validate-redacted PHI fixture",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "test_grpc_validate_redacted_returns_report_receipt_and_redacted_hl7_without_phi",
                "--locked",
            ],
        ),
    ];

    for (label, args) in commands {
        println!("Checking {label}...");
        run_command("cargo", args)?;
    }

    if include_python {
        println!("Checking Python local-wheel smoke...");
        run_command("python", &["tests/python_smoke/smoke.py"])?;
        println!("Checking Python evidence workflow guide...");
        run_command("python", &["tests/python_smoke/evidence_workflow_guide.py"])?;
    } else {
        println!(
            "Python local-wheel smoke skipped; pass --include-python after installing the hl7v2 wheel."
        );
    }

    println!("✅ Safe-error and PHI parity acceptance checks passed!");
    Ok(())
}

fn check_profile_parity(include_python: bool) -> Result<()> {
    println!("ðŸ”Ž Checking profile lint/explain/test parity acceptance...");

    let commands: &[(&str, &[&str])] = &[
        (
            "Rust profile facade evidence behavior",
            &[
                "test",
                "-p",
                "hl7v2",
                "--test",
                "conformance_facade",
                "--all-features",
                "--locked",
                "profile_facade",
            ],
        ),
        (
            "CLI profile lint/explain/test behavior",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "--locked",
                "profile_command",
            ],
        ),
        (
            "REST profile endpoint behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "profile_endpoint_test",
                "--locked",
            ],
        ),
        (
            "gRPC profile RPC behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "--locked",
                "profile",
            ],
        ),
    ];

    for (label, args) in commands {
        println!("Checking {label}...");
        run_command("cargo", args)?;
    }

    if include_python {
        println!("Checking Python local-wheel profile smoke...");
        run_command("python", &["tests/python_smoke/smoke.py"])?;
        println!("Checking Python evidence workflow guide...");
        run_command("python", &["tests/python_smoke/evidence_workflow_guide.py"])?;
    } else {
        println!(
            "Python local-wheel smoke skipped; pass --include-python after installing the hl7v2 wheel."
        );
    }

    println!("âœ… Profile lint/explain/test parity acceptance checks passed!");
    Ok(())
}

fn check_schema_version_parity(include_python: bool) -> Result<()> {
    println!("🔎 Checking schema-version parity acceptance...");

    let commands: &[(&str, &[&str])] = &[
        (
            "Shared schema-version fixture contract",
            &[
                "test",
                "-p",
                "hl7v2-test-utils",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "Rust library schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2",
                "--all-features",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "CLI schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "REST validation v2 schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_endpoint_test",
                "--locked",
                "schema_v2",
            ],
        ),
        (
            "REST validation unsupported schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_endpoint_test",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "REST validate-redacted v2 schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_redacted_endpoint_test",
                "--locked",
                "schema_v2",
            ],
        ),
        (
            "REST validate-redacted unsupported schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "validate_redacted_endpoint_test",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "REST bundle schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "bundle_endpoint_test",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "REST replay schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "replay_endpoint_test",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "REST corpus v2 schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "corpus_endpoint_test",
                "--locked",
                "schema_v2",
            ],
        ),
        (
            "REST corpus unsupported schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "corpus_endpoint_test",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "REST profile v2 schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "profile_endpoint_test",
                "--locked",
                "schema_version_2",
            ],
        ),
        (
            "REST profile unsupported schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "profile_endpoint_test",
                "--locked",
                "schema_versions",
            ],
        ),
        (
            "REST quarantine v2 schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "quarantine_output_hooks_test",
                "--locked",
                "v2_provenance",
            ],
        ),
        (
            "REST quarantine unsupported schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "quarantine_output_hooks_test",
                "--locked",
                "schema_version",
            ],
        ),
        (
            "gRPC v2 schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "--locked",
                "v2",
            ],
        ),
        (
            "gRPC unsupported schema-version behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "--locked",
                "schema_versions",
            ],
        ),
    ];

    for (label, args) in commands {
        println!("Checking {label}...");
        run_command("cargo", args)?;
    }

    println!("Checking evidence fixture schemas...");
    evidence_schema_check()?;

    if include_python {
        println!("Checking Python local-wheel schema-version smoke...");
        run_command("python", &["tests/python_smoke/smoke.py"])?;
        println!("Checking Python evidence workflow guide...");
        run_command("python", &["tests/python_smoke/evidence_workflow_guide.py"])?;
    } else {
        println!(
            "Python local-wheel smoke skipped; pass --include-python after installing the hl7v2 wheel."
        );
    }

    println!("✅ Schema-version parity acceptance checks passed!");
    Ok(())
}

fn check_dirty_corpus_parity(include_python: bool) -> Result<()> {
    println!("🔎 Checking dirty-corpus parity acceptance...");

    let commands: &[(&str, &[&str])] = &[
        (
            "Rust dirty real-world corpus proof",
            &[
                "test",
                "-p",
                "hl7v2",
                "--lib",
                "--all-features",
                "--locked",
                "dirty_real_world",
            ],
        ),
        (
            "CLI dirty-corpus command parity",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "test_corpus_commands_share_dirty_real_world_fixture_categories",
                "--locked",
            ],
        ),
        (
            "CLI dirty evidence workflow parity",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "test_dirty_real_world_validate_redact_bundle_replay_workflow",
                "--locked",
            ],
        ),
        (
            "REST dirty-corpus endpoint parity",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "corpus_endpoint_test",
                "test_corpus_endpoints_share_dirty_real_world_fixture_categories",
                "--locked",
            ],
        ),
        (
            "REST dirty evidence workflow parity",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "replay_endpoint_test",
                "test_rest_dirty_real_world_validate_redact_bundle_replay_workflow",
                "--locked",
            ],
        ),
        (
            "gRPC dirty-corpus RPC parity",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "test_grpc_corpus_commands_share_dirty_real_world_fixture_categories",
                "--locked",
            ],
        ),
        (
            "gRPC dirty evidence workflow parity",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "test_grpc_dirty_real_world_validate_redact_bundle_replay_workflow",
                "--locked",
            ],
        ),
    ];

    for (label, args) in commands {
        println!("Checking {label}...");
        run_command("cargo", args)?;
    }

    if include_python {
        println!("Checking Python local-wheel dirty-corpus smoke...");
        run_command("python", &["tests/python_smoke/smoke.py"])?;
        println!("Checking Python local-wheel dirty evidence workflow...");
        run_command("python", &["tests/python_smoke/dirty_evidence_workflow.py"])?;
    } else {
        println!(
            "Python local-wheel smoke skipped; pass --include-python after installing the hl7v2 wheel."
        );
    }

    println!("✅ Dirty-corpus parity acceptance checks passed!");
    Ok(())
}

fn check_bundle_replay_parity(include_python: bool) -> Result<()> {
    println!("🔎 Checking bundle/replay parity acceptance...");

    let commands: &[(&str, &[&str])] = &[
        (
            "Rust evidence bundle behavior",
            &[
                "test",
                "-p",
                "hl7v2",
                "--lib",
                "--all-features",
                "--locked",
                "bundle_",
            ],
        ),
        (
            "Rust evidence replay behavior",
            &[
                "test",
                "-p",
                "hl7v2",
                "--lib",
                "--all-features",
                "--locked",
                "replay_",
            ],
        ),
        (
            "CLI bundle command behavior",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "bundle_command",
                "--locked",
            ],
        ),
        (
            "CLI replay command behavior",
            &[
                "test",
                "-p",
                "hl7v2-cli",
                "--test",
                "integration_tests",
                "replay_command",
                "--locked",
            ],
        ),
        (
            "REST bundle endpoint behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "bundle_endpoint_test",
                "bundle_endpoint",
                "--locked",
            ],
        ),
        (
            "REST replay endpoint behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "replay_endpoint_test",
                "replay_endpoint",
                "--locked",
            ],
        ),
        (
            "gRPC bundle/replay behavior",
            &[
                "test",
                "-p",
                "hl7v2-server",
                "--test",
                "grpc_contract_tests",
                "evidence_bundle",
                "--locked",
            ],
        ),
    ];

    for (label, args) in commands {
        println!("Checking {label}...");
        run_command("cargo", args)?;
    }

    if include_python {
        println!("Checking Python local-wheel bundle/replay smoke...");
        run_command("python", &["tests/python_smoke/evidence_workflow_guide.py"])?;
    } else {
        println!(
            "Python local-wheel smoke skipped; pass --include-python after installing the hl7v2 wheel."
        );
    }

    println!("✅ Bundle/replay parity acceptance checks passed!");
    Ok(())
}

fn check_evidence_parity_manifest_text(text: &str) -> Result<()> {
    let manifest: toml::Value = toml::from_str(text)
        .map_err(|error| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} is not valid TOML: {error}"))?;

    ensure_top_level_string_value(&manifest, "schema_version", "1.0")?;
    ensure_top_level_string_value(&manifest, "policy", "evidence-parity")?;
    ensure_top_level_string_value(&manifest, "status", "active")?;
    ensure_top_level_array_contains(
        &manifest,
        "non_claims",
        "does not claim TestPyPI, PyPI, npm",
    )?;
    ensure_top_level_array_contains(
        &manifest,
        "non_claims",
        "Python local wheel proof is not public Python registry proof",
    )?;
    ensure_top_level_array_contains(
        &manifest,
        "non_claims",
        "hl7v2-python is binding backend infrastructure",
    )?;
    ensure_top_level_array_contains(&manifest, "non_claims", "TypeScript remains planned")?;
    ensure_top_level_array_contains(
        &manifest,
        "acceptance",
        "cargo run -p xtask -- check-evidence-parity-acceptance",
    )?;

    let surface_table = manifest
        .get("surface")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} [surface] table is missing"))?;
    for surface in EVIDENCE_PARITY_REQUIRED_SURFACES {
        let section = format!("[surface.{surface}]");
        if !surface_table.contains_key(*surface) {
            return Err(anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} missing {section}"));
        }
        ensure_toml_string_non_empty(&manifest, &section, "role", EVIDENCE_PARITY_MANIFEST_PATH)?;
        if *surface != "typescript" {
            ensure_toml_array_non_empty(
                &manifest,
                &section,
                "proof",
                EVIDENCE_PARITY_MANIFEST_PATH,
            )?;
        }
    }

    ensure_pyproject_string_value(
        &manifest,
        "[surface.python]",
        "package",
        "hl7v2",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_string_value(
        &manifest,
        "[surface.python]",
        "backend_crate",
        "hl7v2-python",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_value_contains(
        &manifest,
        "[surface.python]",
        "blocked_by",
        "issues/563",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_array_contains(
        &manifest,
        "[surface.python]",
        "proof",
        "cargo run -p xtask -- python-local-wheel-proof",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_array_contains(
        &manifest,
        "[surface.python]",
        "blocked_registry_proof",
        "cargo run -p xtask -- python-public-registry-proof --index testpypi --version <version>",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_array_contains(
        &manifest,
        "[surface.python]",
        "blocked_registry_proof",
        "cargo run -p xtask -- python-public-registry-proof --index pypi --version <version>",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_string_value(
        &manifest,
        "[surface.typescript]",
        "package",
        "@effortlessmetrics/hl7v2",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_string_value(
        &manifest,
        "[surface.typescript]",
        "tier",
        "planned",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;

    ensure_pyproject_array_contains(
        &manifest,
        "[surface.rest]",
        "proof",
        "cargo test -p hl7v2-server --test parse_endpoint_test",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;
    ensure_pyproject_array_contains(
        &manifest,
        "[surface.rest]",
        "proof",
        "cargo test -p hl7v2-server --test validate_redacted_endpoint_test",
        EVIDENCE_PARITY_MANIFEST_PATH,
    )?;

    let contracts = manifest
        .get("contract")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} [[contract]] array is missing"))?;
    let mut seen = BTreeSet::new();
    for contract in contracts {
        let table = contract.as_table().ok_or_else(|| {
            anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} [[contract]] entries must be tables")
        })?;
        let id = table
            .get("id")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} contract.id is missing"))?;
        if !seen.insert(id.to_string()) {
            return Err(anyhow!(
                "{EVIDENCE_PARITY_MANIFEST_PATH} has duplicate contract id `{id}`"
            ));
        }
        for key in [
            "status",
            "rust",
            "cli",
            "rest",
            "grpc",
            "python",
            "typescript",
        ] {
            let value = table
                .get(key)
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` missing `{key}`")
                })?;
            if key == "python" {
                ensure_python_contract_state_is_not_registry_claim(id, value)?;
            }
            ensure_contract_state_is_allowed(id, key, value)?;
        }
        ensure_contract_text_array_non_empty(table, id, "proof", true)?;
        ensure_contract_text_array_non_empty(table, id, "gaps", false)?;
    }
    for required in EVIDENCE_PARITY_REQUIRED_CONTRACTS {
        if !seen.contains(*required) {
            return Err(anyhow!(
                "{EVIDENCE_PARITY_MANIFEST_PATH} missing required contract `{required}`"
            ));
        }
    }

    ensure_contract_proof_contains(
        contracts,
        "parse-write",
        "cargo test -p hl7v2-server --test parse_endpoint_test",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "redaction-quarantine",
        "cargo test -p hl7v2-server --test validate_redacted_endpoint_test",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "schema-version-behavior",
        "cargo run -p xtask -- check-schema-version-parity",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "schema-version-behavior",
        "cargo test -p hl7v2-cli --test integration_tests test_validate_sample_json_schema_version_two --locked",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "schema-version-behavior",
        "cargo test -p hl7v2-server --test validate_endpoint_test test_validate_report_schema_v2_returns_nested_provenance_report --locked",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "schema-version-behavior",
        "cargo test -p hl7v2-server --test profile_endpoint_test test_profile_lint_schema_version_2_adds_server_provenance --locked",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "schema-version-behavior",
        "cargo test -p hl7v2-server --test grpc_contract_tests test_grpc_validate_separates_errors_from_warnings --locked",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "normalize",
        "cargo test -p hl7v2-server --test http_runtime_contract_test",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "profile-lint-explain-test",
        "cargo run -p xtask -- check-profile-parity",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "profile-lint-explain-test",
        "cargo test -p hl7v2-server --test profile_endpoint_test",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "safe-error-shape",
        "cargo run -p xtask -- check-safe-error-phi-parity",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "phi-sentinel-behavior",
        "cargo run -p xtask -- check-safe-error-phi-parity",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "corpus-summary-fingerprint-diff",
        "cargo run -p xtask -- check-dirty-corpus-parity",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "corpus-summary-fingerprint-diff",
        "cargo test -p hl7v2-cli --test integration_tests test_dirty_real_world_validate_redact_bundle_replay_workflow",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "corpus-summary-fingerprint-diff",
        "cargo test -p hl7v2-server --test replay_endpoint_test test_rest_dirty_real_world_validate_redact_bundle_replay_workflow",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "corpus-summary-fingerprint-diff",
        "cargo test -p hl7v2-server --test grpc_contract_tests test_grpc_dirty_real_world_validate_redact_bundle_replay_workflow",
    )?;
    ensure_contract_proof_contains(
        contracts,
        "bundle-replay",
        "cargo run -p xtask -- check-bundle-replay-parity",
    )?;
    ensure_contract_string_value(
        contracts,
        "corpus-summary-fingerprint-diff",
        "fixture_family",
        "test_data/dirty-real-world/",
    )?;
    ensure_contract_string_value(
        contracts,
        "schema-version-behavior",
        "fixture_family",
        "test_data/evidence/schema-version-parity.json",
    )?;

    Ok(())
}

fn ensure_top_level_string_value(document: &toml::Value, key: &str, expected: &str) -> Result<()> {
    let actual = document
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} {key} must be a string"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} {key} must be `{expected}`, found `{actual}`"
        ))
    }
}

fn ensure_top_level_array_contains(
    document: &toml::Value,
    key: &str,
    expected_substring: &str,
) -> Result<()> {
    let values = document
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} {key} must be an array"))?;
    if values.iter().any(|value| {
        value
            .as_str()
            .is_some_and(|value| value.contains(expected_substring))
    }) {
        Ok(())
    } else {
        Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} {key} must contain text `{expected_substring}`"
        ))
    }
}

fn ensure_toml_array_non_empty(
    document: &toml::Value,
    section: &str,
    key: &str,
    context: &str,
) -> Result<()> {
    let values = pyproject_value(document, section, key, context)?
        .as_array()
        .ok_or_else(|| anyhow!("{context} {section}.{key} must be an array"))?;
    if values.is_empty() {
        return Err(anyhow!("{context} {section}.{key} must not be empty"));
    }
    for value in values {
        let text = value
            .as_str()
            .ok_or_else(|| anyhow!("{context} {section}.{key} entries must be strings"))?;
        if text.trim().is_empty() {
            return Err(anyhow!(
                "{context} {section}.{key} entries must not be empty"
            ));
        }
        if !evidence_parity_proof_reference_is_known(text) {
            return Err(anyhow!(
                "{context} {section}.{key} entry `{text}` must be a known command or approved proof reference"
            ));
        }
    }
    Ok(())
}

fn ensure_toml_string_non_empty(
    document: &toml::Value,
    section: &str,
    key: &str,
    context: &str,
) -> Result<()> {
    let actual = pyproject_value(document, section, key, context)?
        .as_str()
        .ok_or_else(|| anyhow!("{context} {section}.{key} must be a string"))?;
    if actual.trim().is_empty() {
        Err(anyhow!("{context} {section}.{key} must not be empty"))
    } else {
        Ok(())
    }
}

fn ensure_contract_text_array_non_empty(
    contract: &toml::map::Map<String, toml::Value>,
    id: &str,
    key: &str,
    require_proof_reference: bool,
) -> Result<()> {
    let values = contract
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} must be an array")
        })?;
    if values.is_empty() {
        return Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} must not be empty"
        ));
    }
    for value in values {
        let text = value.as_str().ok_or_else(|| {
            anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} entries must be strings")
        })?;
        if text.trim().is_empty() {
            return Err(anyhow!(
                "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} entries must not be empty"
            ));
        }
        if require_proof_reference && !evidence_parity_proof_reference_is_known(text) {
            return Err(anyhow!(
                "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` proof entry `{text}` must be a known command or approved proof reference"
            ));
        }
    }
    Ok(())
}

fn ensure_python_contract_state_is_not_registry_claim(id: &str, value: &str) -> Result<()> {
    let normalized = value.to_ascii_lowercase();
    if normalized.contains("testpypi")
        || normalized.contains("pypi")
        || normalized == "stable"
        || normalized == "released"
    {
        Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` python state `{value}` looks like a public registry claim; use local-wheel-only or required-for-claimed-artifacts until upload/install-back is receipted"
        ))
    } else {
        Ok(())
    }
}

fn ensure_contract_state_is_allowed(id: &str, key: &str, value: &str) -> Result<()> {
    let allowed = match key {
        "status" => EVIDENCE_PARITY_ALLOWED_CONTRACT_STATUS,
        "rust" => EVIDENCE_PARITY_ALLOWED_RUST_STATES,
        "cli" => EVIDENCE_PARITY_ALLOWED_CLI_STATES,
        "rest" => EVIDENCE_PARITY_ALLOWED_REST_STATES,
        "grpc" => EVIDENCE_PARITY_ALLOWED_GRPC_STATES,
        "python" => EVIDENCE_PARITY_ALLOWED_PYTHON_STATES,
        "typescript" => &["planned"],
        _ => return Ok(()),
    };
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} state `{value}` is not in the allowed vocabulary: {}",
            allowed.join(", ")
        ))
    }
}

fn evidence_parity_proof_reference_is_known(value: &str) -> bool {
    value.starts_with("cargo test ")
        || value.starts_with("cargo run ")
        || value.starts_with("python ")
        || value
            == "Surface-specific tests and specs require safe diagnostics without raw PHI echo."
}

fn ensure_contract_proof_contains(
    contracts: &[toml::Value],
    id: &str,
    expected: &str,
) -> Result<()> {
    let contract = contract_table(contracts, id)?;
    let proofs = contract
        .get("proof")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` proof must be an array")
        })?;
    if proofs
        .iter()
        .any(|value| value.as_str().is_some_and(|value| value == expected))
    {
        Ok(())
    } else {
        Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` proof must include `{expected}`"
        ))
    }
}

fn ensure_contract_string_value(
    contracts: &[toml::Value],
    id: &str,
    key: &str,
    expected: &str,
) -> Result<()> {
    let contract = contract_table(contracts, id)?;
    let actual = contract
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} must be a string")
        })?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{EVIDENCE_PARITY_MANIFEST_PATH} contract `{id}` {key} must be `{expected}`, found `{actual}`"
        ))
    }
}

fn contract_table<'a>(
    contracts: &'a [toml::Value],
    id: &str,
) -> Result<&'a toml::map::Map<String, toml::Value>> {
    contracts
        .iter()
        .filter_map(toml::Value::as_table)
        .find(|table| {
            table
                .get("id")
                .and_then(toml::Value::as_str)
                .is_some_and(|actual| actual == id)
        })
        .ok_or_else(|| anyhow!("{EVIDENCE_PARITY_MANIFEST_PATH} missing contract `{id}`"))
}

struct PythonPublishWorkflowPolicy {
    path: &'static str,
    workflow_name: &'static str,
    input_name: &'static str,
    testpypi_proof_input: Option<&'static str>,
    publish_job: &'static str,
    install_job: &'static str,
    environment_name: &'static str,
    artifact_name: &'static str,
    package_index_arg: &'static str,
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
        package_index_arg: "testpypi",
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
        package_index_arg: "pypi",
        publish_repository_url: None,
        publish_step_name: "Publish package distributions to PyPI",
    },
];
const PYTHON_DISTRIBUTION_DESCRIPTION: &str =
    "Python package for HL7v2 parsing, validation, and evidence workflows backed by Rust.";
const HL7V2_PYTHON_CRATE_DESCRIPTION: &str =
    "PyO3 extension crate backing the Python hl7v2 package. Rust users should depend on hl7v2.";
const PYTHON_WHEELS_WORKFLOW_PATH: &str = ".github/workflows/python-wheels.yml";

fn check_python_publish_policy() -> Result<()> {
    println!("🔎 Checking Python publish policy...");
    let root = env::current_dir()?;

    ensure_hl7v2_python_binding_backend_publishable(&root)?;
    check_hl7v2_python_manifest_policy(&root)?;
    check_python_pyproject_policy(&root)?;
    check_python_wheels_workflow(&root)?;
    for policy in PYTHON_PUBLISH_WORKFLOWS {
        check_python_publish_workflow(&root, policy)?;
    }

    println!(
        "✅ python publish policy: pyproject.toml, hl7v2-python metadata, Python Wheels smoke, and {} publish workflow(s) checked; Python distribution is hl7v2 and hl7v2-python is a publishable binding backend crate with separate release receipts required",
        PYTHON_PUBLISH_WORKFLOWS.len()
    );
    Ok(())
}

fn ensure_hl7v2_python_binding_backend_publishable(root: &Path) -> Result<()> {
    let metadata = MetadataCommand::new().current_dir(root).no_deps().exec()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == "hl7v2-python")
        .ok_or_else(|| anyhow!("cargo metadata did not include hl7v2-python"))?;

    if package_is_publishable(package) {
        Ok(())
    } else {
        Err(anyhow!(
            "crates/hl7v2-python/Cargo.toml must be publishable as a governed binding backend crate"
        ))
    }
}

fn check_hl7v2_python_manifest_policy(root: &Path) -> Result<()> {
    let workspace_text = fs::read_to_string(root.join("Cargo.toml"))?;
    let workspace: toml::Value = toml::from_str(&workspace_text)
        .map_err(|error| anyhow!("Cargo.toml is not valid TOML: {error}"))?;
    let workspace_version =
        pyproject_value(&workspace, "[workspace.package]", "version", "Cargo.toml")?
            .as_str()
            .ok_or_else(|| anyhow!("Cargo.toml [workspace.package].version must be a string"))?;

    let manifest_path = root.join("crates/hl7v2-python/Cargo.toml");
    let text = fs::read_to_string(manifest_path)?;
    check_hl7v2_python_manifest_policy_text(&text, workspace_version)
}

fn check_hl7v2_python_manifest_policy_text(text: &str, workspace_version: &str) -> Result<()> {
    let manifest: toml::Value = toml::from_str(text)
        .map_err(|error| anyhow!("crates/hl7v2-python/Cargo.toml is not valid TOML: {error}"))?;

    ensure_pyproject_string_value(
        &manifest,
        "[package]",
        "name",
        "hl7v2-python",
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_pyproject_string_value(
        &manifest,
        "[package]",
        "description",
        HL7V2_PYTHON_CRATE_DESCRIPTION,
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_pyproject_string_value(
        &manifest,
        "[package]",
        "readme",
        "README.md",
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_toml_bool_value(
        &manifest,
        "[package]",
        "publish",
        true,
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_pyproject_string_value(
        &manifest,
        "[lib]",
        "name",
        "hl7v2",
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_pyproject_array_contains(
        &manifest,
        "[lib]",
        "crate-type",
        "cdylib",
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_toml_bool_value(
        &manifest,
        "[lib]",
        "doc",
        false,
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_toml_table_string_value(
        &manifest,
        "[dependencies]",
        "hl7v2",
        "path",
        "../hl7v2",
        "crates/hl7v2-python/Cargo.toml",
    )?;
    ensure_toml_table_string_value(
        &manifest,
        "[dependencies]",
        "hl7v2",
        "version",
        workspace_version,
        "crates/hl7v2-python/Cargo.toml",
    )?;

    Ok(())
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
    ensure_pyproject_string_value(&pyproject, "[project]", "name", "hl7v2", "pyproject.toml")?;
    ensure_pyproject_string_value(
        &pyproject,
        "[project]",
        "description",
        PYTHON_DISTRIBUTION_DESCRIPTION,
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

fn check_python_wheels_workflow(root: &Path) -> Result<()> {
    let text = fs::read_to_string(root.join(PYTHON_WHEELS_WORKFLOW_PATH))?;
    check_python_wheels_workflow_text(&text)
}

fn check_python_wheels_workflow_text(text: &str) -> Result<()> {
    let workflow: serde_yaml::Value = serde_yaml::from_str(text)
        .map_err(|error| anyhow!("{PYTHON_WHEELS_WORKFLOW_PATH} is not valid YAML: {error}"))?;
    let root_map = yaml_mapping(&workflow, PYTHON_WHEELS_WORKFLOW_PATH)?;

    ensure_yaml_string(
        root_map,
        PYTHON_WHEELS_WORKFLOW_PATH,
        "name",
        "Python Wheels",
    )?;
    let permissions = yaml_child_mapping(root_map, PYTHON_WHEELS_WORKFLOW_PATH, "permissions")?;
    ensure_yaml_permission(permissions, PYTHON_WHEELS_WORKFLOW_PATH, "contents", "read")?;
    ensure_yaml_missing(permissions, PYTHON_WHEELS_WORKFLOW_PATH, "id-token")?;

    let jobs = yaml_child_mapping(root_map, PYTHON_WHEELS_WORKFLOW_PATH, "jobs")?;
    let wheel_job = yaml_mapping_child(jobs, PYTHON_WHEELS_WORKFLOW_PATH, "jobs", "wheel-smoke")?;
    let steps = yaml_child_sequence(wheel_job, PYTHON_WHEELS_WORKFLOW_PATH, "steps")?;

    let build = yaml_step_named(steps, PYTHON_WHEELS_WORKFLOW_PATH, "Build wheel")?;
    let build_run = yaml_mapping_string(build, PYTHON_WHEELS_WORKFLOW_PATH, "run")?;
    if !build_run.contains("maturin build --release --out dist") {
        return Err(anyhow!(
            "{PYTHON_WHEELS_WORKFLOW_PATH} Build wheel step must run `maturin build --release --out dist`"
        ));
    }

    let install = yaml_step_named(steps, PYTHON_WHEELS_WORKFLOW_PATH, "Install built wheel")?;
    let install_run = yaml_mapping_string(install, PYTHON_WHEELS_WORKFLOW_PATH, "run")?;
    for expected in ["dist/*.whl", "pip", "install"] {
        if !install_run.contains(expected) {
            return Err(anyhow!(
                "{PYTHON_WHEELS_WORKFLOW_PATH} Install built wheel step must contain `{expected}`"
            ));
        }
    }

    for (step_name, expected) in [
        (
            "Run import smoke test",
            "python tests/python_smoke/smoke.py",
        ),
        (
            "Run Python evidence guide smoke test",
            "python tests/python_smoke/evidence_workflow_guide.py",
        ),
        (
            "Run Python dirty evidence workflow smoke test",
            "python tests/python_smoke/dirty_evidence_workflow.py",
        ),
    ] {
        let step = yaml_step_named(steps, PYTHON_WHEELS_WORKFLOW_PATH, step_name)?;
        let run = yaml_mapping_string(step, PYTHON_WHEELS_WORKFLOW_PATH, "run")?;
        if !run.contains(expected) {
            return Err(anyhow!(
                "{PYTHON_WHEELS_WORKFLOW_PATH} `{step_name}` step must contain `{expected}`"
            ));
        }
    }

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

fn ensure_toml_bool_value(
    document: &toml::Value,
    section: &str,
    key: &str,
    expected: bool,
    context: &str,
) -> Result<()> {
    let actual = pyproject_value(document, section, key, context)?
        .as_bool()
        .ok_or_else(|| anyhow!("{context} {section}.{key} must be a boolean"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} {section}.{key} must be `{expected}`, found `{actual}`"
        ))
    }
}

fn ensure_toml_table_string_value(
    document: &toml::Value,
    section: &str,
    key: &str,
    table_key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    let actual = pyproject_value(document, section, key, context)?
        .as_table()
        .and_then(|table| table.get(table_key))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| anyhow!("{context} {section}.{key}.{table_key} must be a string"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(anyhow!(
            "{context} {section}.{key}.{table_key} must be `{expected}`, found `{actual}`"
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
        "tests/python_smoke/dirty_evidence_workflow.py",
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
    let rust_toolchain = yaml_step_named(steps, policy.path, "Install Rust toolchain")?;
    ensure_yaml_string(
        rust_toolchain,
        policy.path,
        "uses",
        "dtolnay/rust-toolchain@v1",
    )?;
    let rust_toolchain_with = yaml_child_mapping(rust_toolchain, policy.path, "with")?;
    ensure_yaml_string(rust_toolchain_with, policy.path, "toolchain", "1.95.0")?;

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
        "cargo run -p xtask -- python-public-registry-proof",
        "--index",
        policy.package_index_arg,
        "--version \"${PACKAGE_VERSION}\"",
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
    use crate::publish::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn read_policy_workflow_for_mutation(
        root: &Path,
        policy: &PythonPublishWorkflowPolicy,
    ) -> Result<String> {
        let workflow = fs::read_to_string(root.join(policy.path))?;
        Ok(workflow.replace("\r\n", "\n"))
    }

    #[test]
    fn command_exists_reports_present_and_missing_commands() {
        assert!(command_exists("cargo"));
        assert!(!command_exists("__hl7v2_missing_command__"));
    }

    #[test]
    fn command_program_uses_platform_runner_suffix() {
        #[cfg(windows)]
        assert_eq!(command_program("cargo"), "cargo.cmd");
        #[cfg(not(windows))]
        assert_eq!(command_program("cargo"), "cargo");
    }

    #[test]
    fn check_first_use_guides_command_defaults_to_local_only() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-first-use-guides"])?;
        match cli.command {
            Commands::CheckFirstUseGuides {
                include_python,
                include_public_crates,
            } => {
                if include_python || include_public_crates {
                    return Err(anyhow!(
                        "check-first-use-guides should default to local non-registry proof"
                    ));
                }
                Ok(())
            }
            _ => Err(anyhow!("expected check-first-use-guides command")),
        }
    }

    #[test]
    fn check_first_use_guides_accepts_optional_surface_flags() -> Result<()> {
        let cli = Cli::try_parse_from([
            "xtask",
            "check-first-use-guides",
            "--include-python",
            "--include-public-crates",
        ])?;
        match cli.command {
            Commands::CheckFirstUseGuides {
                include_python,
                include_public_crates,
            } => {
                if !(include_python && include_public_crates) {
                    return Err(anyhow!(
                        "check-first-use-guides should preserve optional surface flags"
                    ));
                }
                Ok(())
            }
            _ => Err(anyhow!("expected check-first-use-guides command")),
        }
    }

    #[test]
    fn check_first_10_minutes_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-first-10-minutes-guide"])?;
        match cli.command {
            Commands::CheckFirst10MinutesGuide => Ok(()),
            _ => Err(anyhow!("expected check-first-10-minutes-guide command")),
        }
    }

    #[test]
    fn check_first_use_by_surface_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-first-use-by-surface-guide"])?;
        match cli.command {
            Commands::CheckFirstUseBySurfaceGuide => Ok(()),
            _ => Err(anyhow!("expected check-first-use-by-surface-guide command")),
        }
    }

    #[test]
    fn check_vendor_upgrade_diff_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-vendor-upgrade-diff-guide"])?;
        match cli.command {
            Commands::CheckVendorUpgradeDiffGuide => Ok(()),
            _ => Err(anyhow!("expected check-vendor-upgrade-diff-guide command")),
        }
    }

    #[test]
    fn check_operator_error_guidance_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-operator-error-guidance-guide"])?;
        match cli.command {
            Commands::CheckOperatorErrorGuidanceGuide => Ok(()),
            _ => Err(anyhow!(
                "expected check-operator-error-guidance-guide command"
            )),
        }
    }

    #[test]
    fn check_safe_support_bundle_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-safe-support-bundle-guide"])?;
        match cli.command {
            Commands::CheckSafeSupportBundleGuide => Ok(()),
            _ => Err(anyhow!("expected check-safe-support-bundle-guide command")),
        }
    }

    #[test]
    fn check_evidence_artifacts_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-evidence-artifacts-guide"])?;
        match cli.command {
            Commands::CheckEvidenceArtifactsGuide => Ok(()),
            _ => Err(anyhow!("expected check-evidence-artifacts-guide command")),
        }
    }

    #[test]
    fn check_sidecar_guide_command_parses() -> Result<()> {
        let cli = Cli::try_parse_from(["xtask", "check-sidecar-guide"])?;
        match cli.command {
            Commands::CheckSidecarGuide => Ok(()),
            _ => Err(anyhow!("expected check-sidecar-guide command")),
        }
    }

    #[test]
    fn sidecar_guide_config_uses_selected_port() {
        let config = sidecar_guide_config(49152);
        assert!(config.contains("host = \"127.0.0.1\""));
        assert!(config.contains("port = 49152"));
        assert!(config.contains("bundle_output_root = \"target/hl7v2-sidecar/bundles\""));
    }

    #[test]
    fn ensure_tcp_port_available_rejects_bound_address() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let Err(error) = ensure_tcp_port_available(&addr.to_string()) else {
            return Err(anyhow!("bound port should fail availability check"));
        };
        if !error.to_string().contains("sidecar guide smoke requires") {
            return Err(anyhow!("unexpected availability error: {error}"));
        }
        Ok(())
    }

    #[test]
    fn python_venv_executable_uses_platform_path() {
        let venv = Path::new("proof-venv");

        #[cfg(windows)]
        assert_eq!(
            python_executable_in_venv(venv),
            PathBuf::from("proof-venv")
                .join("Scripts")
                .join("python.exe")
        );
        #[cfg(not(windows))]
        assert_eq!(
            python_executable_in_venv(venv),
            PathBuf::from("proof-venv").join("bin").join("python")
        );
    }

    #[test]
    fn single_wheel_in_dist_requires_exactly_one_wheel() -> Result<()> {
        let root = doc_link_temp_root("python-wheel-proof")?;
        let dist = root.join("dist");
        fs::create_dir_all(&dist)?;

        match single_wheel_in_dist(&dist) {
            Ok(_) => return Err(anyhow!("empty dist should not select a wheel")),
            Err(err) if err.to_string().contains("no wheel found") => {}
            Err(err) => return Err(anyhow!("unexpected empty dist error: {err}")),
        }

        let wheel = dist.join("hl7v2-1.5.0-py3-none-any.whl");
        fs::write(&wheel, [])?;
        let selected = single_wheel_in_dist(&dist)?;
        if selected != wheel {
            return Err(anyhow!(
                "expected {}, selected {}",
                wheel.display(),
                selected.display()
            ));
        }

        fs::write(dist.join("hl7v2-1.5.0-cp314-win_amd64.whl"), [])?;
        match single_wheel_in_dist(&dist) {
            Ok(_) => return Err(anyhow!("multi-wheel dist should be rejected")),
            Err(err) if err.to_string().contains("expected exactly one wheel") => {}
            Err(err) => return Err(anyhow!("unexpected multi-wheel dist error: {err}")),
        }

        remove_doc_link_temp_root(&root)?;
        Ok(())
    }

    #[test]
    fn python_local_wheel_root_resolves_relative_to_workspace_root() -> Result<()> {
        let workspace_root = doc_link_temp_root("python-wheel-proof-workspace")?;
        let requested = PathBuf::from("target/hl7v2-python-proof-test");
        let expected = workspace_root.join(&requested);
        let actual = prepare_python_local_wheel_root(Some(requested), false, &workspace_root)?;

        if actual != expected {
            return Err(anyhow!(
                "expected proof root {}, got {}",
                expected.display(),
                actual.display()
            ));
        }
        if !actual.exists() {
            return Err(anyhow!(
                "proof root should be created at {}",
                actual.display()
            ));
        }

        remove_doc_link_temp_root(&workspace_root)?;
        Ok(())
    }

    #[test]
    fn python_public_registry_proof_command_defaults_to_testpypi() -> Result<()> {
        let cli = Cli::try_parse_from([
            "xtask",
            "python-public-registry-proof",
            "--version",
            "1.5.0",
        ])?;
        match cli.command {
            Commands::PythonPublicRegistryProof { index, version, .. } => {
                if index != PythonPackageIndex::Testpypi {
                    return Err(anyhow!("python public proof should default to TestPyPI"));
                }
                if version.as_deref() != Some("1.5.0") {
                    return Err(anyhow!("python public proof should preserve version"));
                }
                Ok(())
            }
            _ => Err(anyhow!("expected python-public-registry-proof command")),
        }
    }

    #[test]
    fn python_public_registry_proof_accepts_pypi_index() -> Result<()> {
        let cli =
            Cli::try_parse_from(["xtask", "python-public-registry-proof", "--index", "pypi"])?;
        match cli.command {
            Commands::PythonPublicRegistryProof { index, .. } => {
                if index == PythonPackageIndex::Pypi {
                    Ok(())
                } else {
                    Err(anyhow!("python public proof should parse --index pypi"))
                }
            }
            _ => Err(anyhow!("expected python-public-registry-proof command")),
        }
    }

    #[test]
    fn python_public_registry_package_index_urls_are_explicit() {
        assert_eq!(
            python_package_index_url(PythonPackageIndex::Testpypi),
            "https://test.pypi.org/simple/"
        );
        assert_eq!(
            python_package_index_url(PythonPackageIndex::Pypi),
            "https://pypi.org/simple/"
        );
    }

    #[test]
    fn python_public_registry_root_resolves_relative_to_workspace_root() -> Result<()> {
        let workspace_root = doc_link_temp_root("python-registry-proof-workspace")?;
        let requested = PathBuf::from("target/hl7v2-python-registry-proof-test");
        let expected = workspace_root.join(&requested);
        let actual =
            prepare_python_public_registry_proof_root(Some(requested), false, &workspace_root)?;

        if actual != expected {
            return Err(anyhow!(
                "expected proof root {}, got {}",
                expected.display(),
                actual.display()
            ));
        }
        if !actual.exists() {
            return Err(anyhow!(
                "proof root should be created at {}",
                actual.display()
            ));
        }

        remove_doc_link_temp_root(&workspace_root)?;
        Ok(())
    }

    #[test]
    fn python_public_registry_root_requires_registry_marker() -> Result<()> {
        let workspace_root = doc_link_temp_root("python-registry-proof-workspace-bad")?;
        match prepare_python_public_registry_proof_root(
            Some(PathBuf::from("target/hl7v2-python-proof-test")),
            false,
            &workspace_root,
        ) {
            Ok(path) => Err(anyhow!(
                "unexpectedly accepted public registry proof root {}",
                path.display()
            )),
            Err(error) if error.to_string().contains("registry") => {
                remove_doc_link_temp_root(&workspace_root)?;
                Ok(())
            }
            Err(error) => Err(anyhow!("unexpected root validation error: {error}")),
        }
    }

    #[test]
    fn recreate_dir_removes_previous_contents() -> Result<()> {
        let root = doc_link_temp_root("python-wheel-proof-recreate")?;
        let proof_dir = root.join("venv");
        let stale_file = proof_dir.join("stale.txt");
        fs::create_dir_all(&proof_dir)?;
        fs::write(&stale_file, "stale")?;

        recreate_dir(&proof_dir)?;

        if !proof_dir.exists() {
            return Err(anyhow!(
                "proof dir should be recreated at {}",
                proof_dir.display()
            ));
        }
        if stale_file.exists() {
            return Err(anyhow!(
                "recreated proof dir retained stale file {}",
                stale_file.display()
            ));
        }

        remove_doc_link_temp_root(&root)?;
        Ok(())
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
    fn publish_order_surfaces_are_separate() -> Result<()> {
        let primary = publish_order_for_surface(PublishSurface::Primary, None)?;
        let bindings = publish_order_for_surface(PublishSurface::Bindings, None)?;
        let all_publishable = publish_order_for_surface(PublishSurface::AllPublishable, None)?;

        for public_surface in PRIMARY_RUST_PRODUCT_CRATES {
            ensure_contains(&primary, public_surface)?;
            ensure_contains(&all_publishable, public_surface)?;
        }

        ensure_not_contains(&primary, "hl7v2-python")?;
        ensure_contains(&bindings, "hl7v2-python")?;
        ensure_contains(&all_publishable, "hl7v2-python")?;
        Ok(())
    }

    #[test]
    fn publish_order_rejects_unclassified_publishable_workspace_package() -> Result<()> {
        let metadata = MetadataCommand::new().exec()?;
        let mut packages = workspace_member_packages(&metadata);
        let mut unclassified = packages
            .get("hl7v2")
            .ok_or_else(|| anyhow!("hl7v2 should be present in workspace packages"))?
            .clone();
        let unclassified_name = "hl7v2-unclassified-test".to_string();
        unclassified.name = unclassified_name.clone();
        packages.insert(unclassified_name.clone(), unclassified);

        let error = match ensure_publishable_workspace_packages_are_classified(&packages) {
            Ok(()) => {
                return Err(anyhow!(
                    "unclassified publishable package should fail surface classification"
                ));
            }
            Err(error) => error.to_string(),
        };

        if !error
            .contains("publishable workspace package(s) are missing publish surface classification")
            || !error.contains(&unclassified_name)
        {
            return Err(anyhow!("unexpected surface classification error: {error}"));
        }
        Ok(())
    }

    #[test]
    fn binding_backend_dry_run_targets_include_nonpublishable_backend() -> Result<()> {
        let metadata = MetadataCommand::new().exec()?;
        let targets = binding_backend_dry_run_targets(&metadata, None)?;

        let expected = vec![BindingBackendDryRunTarget {
            name: "hl7v2-python".to_string(),
            publishable: true,
        }];
        if targets != expected {
            return Err(anyhow!(
                "binding backend dry-run targets were {targets:?}, expected {expected:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn binding_backend_dry_run_can_resume_from_backend_crate() -> Result<()> {
        let metadata = MetadataCommand::new().exec()?;
        let targets = binding_backend_dry_run_targets(&metadata, Some("hl7v2-python"))?;

        let expected = vec![BindingBackendDryRunTarget {
            name: "hl7v2-python".to_string(),
            publishable: true,
        }];
        if targets != expected {
            return Err(anyhow!(
                "resumed binding backend dry-run targets were {targets:?}, expected {expected:?}"
            ));
        }
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
        let packages =
            publishable_workspace_packages_for_surface(&metadata, PublishSurface::AllPublishable)?;
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

        ensure_hl7v2_python_binding_backend_publishable(&root)?;
        check_hl7v2_python_manifest_policy(&root)?;
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
name = "hl7v2"
dynamic = ["version"]
description = "Python package for HL7v2 parsing, validation, and evidence workflows backed by Rust."
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
    fn hl7v2_python_manifest_policy_accepts_backend_metadata() -> Result<()> {
        let manifest = r#"
[package]
name = "hl7v2-python"
description = "PyO3 extension crate backing the Python hl7v2 package. Rust users should depend on hl7v2."
readme = "README.md"
publish = true

[lib]
name = "hl7v2"
crate-type = ["cdylib"]
doc = false

[dependencies]
hl7v2 = { version = "1.5.0", path = "../hl7v2" }
"#;

        check_hl7v2_python_manifest_policy_text(manifest, "1.5.0")
    }

    #[test]
    fn evidence_parity_policy_covers_checked_in_manifest() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;

        check_evidence_parity_manifest_text(&text)
    }

    #[test]
    fn evidence_parity_policy_rejects_public_python_registry_overclaim() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replacen(
            "python = \"local-wheel-only\"",
            "python = \"PyPI-released\"",
            1,
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject public Python registry overclaims"
            )),
            Err(err)
                if err.to_string().contains("python state")
                    && err.to_string().contains("public registry claim") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_python_local_wheel_proof_command() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text
            .lines()
            .filter(|line| !line.contains("cargo run -p xtask -- python-local-wheel-proof"))
            .collect::<Vec<_>>()
            .join("\n");

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should require the Python local wheel proof command"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("cargo run -p xtask -- python-local-wheel-proof") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_python_public_registry_blocked_proof_command() -> Result<()>
    {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text
            .lines()
            .filter(|line| !line.contains("python-public-registry-proof --index testpypi"))
            .collect::<Vec<_>>()
            .join("\n");

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should require the blocked public Python registry proof command"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("python-public-registry-proof --index testpypi") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_production_python_registry_blocked_proof_command()
    -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text
            .lines()
            .filter(|line| !line.contains("python-public-registry-proof --index pypi"))
            .collect::<Vec<_>>()
            .join("\n");

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should require the blocked production Python registry proof command"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("python-public-registry-proof --index pypi") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_rejects_unknown_contract_state() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replacen("rest = \"stable\"", "rest = \"stable-for-magic\"", 1);

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject unknown contract state vocabulary"
            )),
            Err(err)
                if err.to_string().contains("allowed vocabulary")
                    && err.to_string().contains("stable-for-magic") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_rejects_unknown_proof_references() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replacen(
            "\"cargo test -p hl7v2 --all-features\",",
            "\"not-a-proof-command\",",
            1,
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject unknown proof references"
            )),
            Err(err)
                if err.to_string().contains("proof entry")
                    && err.to_string().contains("not-a-proof-command") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_rest_parse_and_redaction_proofs() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text
            .replace(
                "\"cargo test -p hl7v2-server --test parse_endpoint_test\",",
                "\"cargo test -p hl7v2-server --test missing_parse_endpoint_test\",",
            )
            .replace(
                "\"cargo test -p hl7v2-server --test validate_redacted_endpoint_test\",",
                "\"cargo test -p hl7v2-server --test missing_validate_redacted_endpoint_test\",",
            );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject missing REST parse/redaction proof commands"
            )),
            Err(err)
                if err.to_string().contains("parse_endpoint_test")
                    || err.to_string().contains("validate_redacted_endpoint_test") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_schema_version_fixture() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "fixture_family = \"test_data/evidence/schema-version-parity.json\"",
            "fixture_family = \"test_data/evidence/old-schema-version-fixture.json\"",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing schema-version fixture family"
            )),
            Err(err) if err.to_string().contains("schema-version-parity.json") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_schema_version_proofs() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo test -p hl7v2-cli --test integration_tests test_validate_sample_json_schema_version_two --locked\",",
            "\"cargo test -p hl7v2-cli --test integration_tests test_old_schema_version_two --locked\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject missing schema-version proof commands"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("test_validate_sample_json_schema_version_two") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_schema_version_runner() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo run -p xtask -- check-schema-version-parity\",",
            "\"cargo run -p xtask -- old-schema-version-parity\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing schema-version runner"
            )),
            Err(err) if err.to_string().contains("check-schema-version-parity") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_safe_error_phi_runner() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo run -p xtask -- check-safe-error-phi-parity\",",
            "\"cargo run -p xtask -- old-safe-error-phi-parity\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing safe-error/PHI runner"
            )),
            Err(err) if err.to_string().contains("check-safe-error-phi-parity") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_normalize_http_proof() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo test -p hl7v2-server --test http_runtime_contract_test\",",
            "\"cargo test -p hl7v2-server --test old_runtime_contract_test\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject missing normalize HTTP proof"
            )),
            Err(err) if err.to_string().contains("http_runtime_contract_test") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_profile_runner() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo run -p xtask -- check-profile-parity\",",
            "\"cargo run -p xtask -- old-profile-parity\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing profile runner"
            )),
            Err(err) if err.to_string().contains("check-profile-parity") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_rest_profile_proof() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo test -p hl7v2-server --test profile_endpoint_test\",",
            "\"cargo test -p hl7v2-server --test old_profile_endpoint_test\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing REST profile proof"
            )),
            Err(err) if err.to_string().contains("profile_endpoint_test") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_dirty_corpus_runner() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo run -p xtask -- check-dirty-corpus-parity\",",
            "\"cargo run -p xtask -- old-dirty-corpus-parity\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing dirty-corpus runner"
            )),
            Err(err) if err.to_string().contains("check-dirty-corpus-parity") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_dirty_workflow_proof() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo test -p hl7v2-cli --test integration_tests test_dirty_real_world_validate_redact_bundle_replay_workflow\",",
            "\"cargo test -p hl7v2-cli --test integration_tests test_old_dirty_real_world_workflow\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing dirty workflow proof"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("test_dirty_real_world_validate_redact_bundle_replay_workflow") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_rest_dirty_workflow_proof() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo test -p hl7v2-server --test replay_endpoint_test test_rest_dirty_real_world_validate_redact_bundle_replay_workflow\",",
            "\"cargo test -p hl7v2-server --test replay_endpoint_test test_old_rest_dirty_real_world_workflow\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing REST dirty workflow proof"
            )),
            Err(err)
                if err.to_string().contains(
                    "test_rest_dirty_real_world_validate_redact_bundle_replay_workflow",
                ) =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_grpc_dirty_workflow_proof() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo test -p hl7v2-server --test grpc_contract_tests test_grpc_dirty_real_world_validate_redact_bundle_replay_workflow\",",
            "\"cargo test -p hl7v2-server --test grpc_contract_tests test_old_grpc_dirty_real_world_workflow\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing gRPC dirty workflow proof"
            )),
            Err(err)
                if err.to_string().contains(
                    "test_grpc_dirty_real_world_validate_redact_bundle_replay_workflow",
                ) =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_bundle_replay_runner() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo run -p xtask -- check-bundle-replay-parity\",",
            "\"cargo run -p xtask -- old-bundle-replay-parity\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing bundle/replay runner"
            )),
            Err(err) if err.to_string().contains("check-bundle-replay-parity") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_requires_acceptance_runner() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "\"cargo run -p xtask -- check-evidence-parity-acceptance\",",
            "\"cargo run -p xtask -- old-evidence-parity-acceptance\",",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject a missing acceptance runner"
            )),
            Err(err) if err.to_string().contains("check-evidence-parity-acceptance") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn evidence_parity_policy_rejects_missing_required_contract() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let text = fs::read_to_string(root.join(EVIDENCE_PARITY_MANIFEST_PATH))?;
        let broken = text.replace(
            "id = \"safe-error-shape\"",
            "id = \"safe-error-shape-renamed\"",
        );

        match check_evidence_parity_manifest_text(&broken) {
            Ok(()) => Err(anyhow!(
                "evidence parity policy should reject missing required contracts"
            )),
            Err(err) if err.to_string().contains("safe-error-shape") => Ok(()),
            Err(err) => Err(anyhow!("unexpected evidence parity policy error: {err}")),
        }
    }

    #[test]
    fn hl7v2_python_manifest_policy_rejects_generic_description() -> Result<()> {
        let manifest = r#"
[package]
name = "hl7v2-python"
description = "Python bindings for HL7v2 via PyO3."
readme = "README.md"
publish = false

[lib]
name = "hl7v2"
crate-type = ["cdylib"]
doc = false

[dependencies]
hl7v2 = { version = "1.5.0", path = "../hl7v2" }
"#;

        match check_hl7v2_python_manifest_policy_text(manifest, "1.5.0") {
            Ok(()) => Err(anyhow!(
                "hl7v2-python manifest policy should reject generic binding descriptions"
            )),
            Err(err) if err.to_string().contains("[package].description") => Ok(()),
            Err(err) => Err(anyhow!(
                "unexpected hl7v2-python manifest policy error: {err}"
            )),
        }
    }

    #[test]
    fn python_pyproject_policy_rejects_wrong_maturin_manifest_path() -> Result<()> {
        let pyproject = r#"
[build-system]
requires = ["maturin>=1.13.1,<2"]
build-backend = "maturin"

[project]
name = "hl7v2"
dynamic = ["version"]
description = "Python package for HL7v2 parsing, validation, and evidence workflows backed by Rust."
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
    fn python_publish_policy_rejects_missing_dirty_evidence_smoke() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace("python tests/python_smoke/dirty_evidence_workflow.py", "");

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject workflows without dirty evidence smoke"
            )),
            Err(err)
                if err.to_string().contains("local wheel proof step")
                    && err.to_string().contains("dirty_evidence_workflow.py") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_requires_public_registry_install_back_command() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace(
            "cargo run -p xtask -- python-public-registry-proof",
            "python -m pip install --index-url https://test.pypi.org/simple/",
        );
        if broken == workflow {
            return Err(anyhow!(
                "test setup should remove the public-registry proof command"
            ));
        }

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject install-back jobs that bypass xtask public-registry proof"
            )),
            Err(err) if err.to_string().contains("python-public-registry-proof") => Ok(()),
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_rejects_wrong_public_registry_index() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let policy = PYTHON_PUBLISH_WORKFLOWS
            .first()
            .ok_or_else(|| anyhow!("expected at least one Python publish workflow policy"))?;
        let workflow = read_policy_workflow_for_mutation(&root, policy)?;
        let broken = workflow.replace("--index testpypi", "--index pypi");
        if broken == workflow {
            return Err(anyhow!("test setup should change the install-back index"));
        }

        match check_python_publish_workflow_text(policy, &broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject install-back jobs that use the wrong public index"
            )),
            Err(err) if err.to_string().contains("testpypi") => Ok(()),
            Err(err) => Err(anyhow!("unexpected python publish policy error: {err}")),
        }
    }

    #[test]
    fn python_publish_policy_covers_python_wheels_workflow() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let workflow = fs::read_to_string(root.join(PYTHON_WHEELS_WORKFLOW_PATH))?;

        check_python_wheels_workflow_text(&workflow)
    }

    #[test]
    fn python_publish_policy_rejects_python_wheels_missing_dirty_smoke() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
            .to_path_buf();
        let workflow = fs::read_to_string(root.join(PYTHON_WHEELS_WORKFLOW_PATH))?;
        let broken = workflow.replace(
            "python tests/python_smoke/dirty_evidence_workflow.py",
            "python tests/python_smoke/smoke.py",
        );
        if broken == workflow {
            return Err(anyhow!(
                "test setup should remove the Python Wheels dirty evidence smoke command"
            ));
        }

        match check_python_wheels_workflow_text(&broken) {
            Ok(()) => Err(anyhow!(
                "python publish policy should reject Python Wheels without dirty evidence smoke"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("Run Python dirty evidence workflow smoke test") =>
            {
                Ok(())
            }
            Err(err) => Err(anyhow!("unexpected Python Wheels policy error: {err}")),
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

    #[test]
    fn companion_policy_validates_common_schema() -> Result<()> {
        let spec = CompanionPolicySpec {
            path: "policy/generated-allowlist.toml",
            policy: "generated-allowlist",
            required_locator: &["paths"],
        };
        let text = r#"
schema_version = "1.0"
policy = "generated-allowlist"
owner = "EffortlessMetrics"
status = "active"

[[allow]]
id = "generated-baseline"
owner = "release/ci"
surface = "panic-policy"
behavior = "Generated no-new-debt baseline may be refreshed only by the dedicated baseline command."
paths = ["policy/no-panic-baseline.toml"]
generated_by = ["cargo run -p xtask -- no-panic baseline --reset"]
reason = "The baseline is a generated policy receipt, not hand-written prose."
covered_by = ["cargo run -p xtask -- check-no-panic-family"]
review_after = "2026-06-30"
"#;

        let entries = parse_companion_policy_ledger(&spec, text)?;
        if entries.len() != 1 {
            return Err(anyhow!("expected one companion entry"));
        }
        let first = entries
            .first()
            .ok_or_else(|| anyhow!("expected first companion entry"))?;
        if first.id != "generated-baseline" {
            return Err(anyhow!(
                "expected generated-baseline entry id, found {}",
                first.id
            ));
        }
        Ok(())
    }

    #[test]
    fn companion_policy_rejects_duplicate_ids() -> Result<()> {
        let spec = CompanionPolicySpec {
            path: "policy/process-allowlist.toml",
            policy: "process-allowlist",
            required_locator: &["commands"],
        };
        let text = r#"
schema_version = "1.0"
policy = "process-allowlist"
owner = "EffortlessMetrics"
status = "active"

[[allow]]
id = "cargo"
owner = "release/ci"
surface = "build"
behavior = "Cargo may run repository checks."
commands = ["cargo check"]
reason = "Rust build system."
covered_by = ["cargo check --workspace"]

[[allow]]
id = "cargo"
owner = "release/ci"
surface = "build"
behavior = "Cargo may run repository tests."
commands = ["cargo test"]
reason = "Rust test system."
covered_by = ["cargo test --workspace"]
"#;

        let Err(err) = parse_companion_policy_ledger(&spec, text) else {
            return Err(anyhow!("duplicate companion policy id should fail"));
        };
        if !err.to_string().contains("duplicates allow entry id") {
            return Err(anyhow!("unexpected duplicate-id error: {err}"));
        }
        Ok(())
    }

    #[test]
    fn companion_policy_requires_broad_glob_reason() -> Result<()> {
        let spec = CompanionPolicySpec {
            path: "policy/executable-allowlist.toml",
            policy: "executable-allowlist",
            required_locator: &["paths"],
        };
        let text = r#"
schema_version = "1.0"
policy = "executable-allowlist"
owner = "EffortlessMetrics"
status = "active"

[[allow]]
id = "scripts"
owner = "release/ci"
surface = "developer-tooling"
behavior = "Scripts may execute local validation helpers."
paths = ["scripts/**"]
reason = "Scripts are owned tooling entrypoints."
covered_by = ["cargo run -p xtask -- check-file-policy"]
"#;

        let Err(err) = parse_companion_policy_ledger(&spec, text) else {
            return Err(anyhow!("broad path glob should require a reason"));
        };
        if !err.to_string().contains("broad path glob") {
            return Err(anyhow!("unexpected broad-glob error: {err}"));
        }
        Ok(())
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
            assert_eq!(finding.snippet, "let _ = some.unwrap();");
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

    fn test_no_panic_entry(id: &str, snippet: &str, count: usize) -> NoPanicAllowEntry {
        NoPanicAllowEntry {
            id: id.into(),
            path: "crates/x/src/lib.rs".into(),
            family: "unwrap".into(),
            classification: "test_helper".into(),
            owner: "x".into(),
            explanation: "y".into(),
            expires: "2027-01-01".into(),
            snippet: snippet.into(),
            count,
            selector_kind: "method_call".into(),
            selector_callee: "unwrap".into(),
            selector_container: Some("parse_msh".into()),
        }
    }

    fn test_no_panic_finding(snippet: &str, line: usize) -> PanicFinding {
        PanicFinding {
            path: "crates/x/src/lib.rs".into(),
            family: PanicFamily::Unwrap,
            container: Some("parse_msh".into()),
            snippet: snippet.into(),
            line,
            column: 9,
        }
    }

    fn test_no_panic_baseline_entry(snippet: &str, count: usize) -> NoPanicBaselineEntry {
        NoPanicBaselineEntry {
            path: "crates/x/src/lib.rs".into(),
            family: "unwrap".into(),
            snippet: snippet.into(),
            count,
            selector_kind: "method_call".into(),
            selector_callee: "unwrap".into(),
            selector_container: Some("parse_msh".into()),
            last_seen_line: 10,
            last_seen_column: 9,
        }
    }

    #[test]
    fn allowlist_entry_requires_exact_snippet() {
        let entry = test_no_panic_entry("panic-0001", "let _ = some.unwrap();", 1);
        let finding = test_no_panic_finding("let _ = some.unwrap();", 99);
        assert!(no_panic_entry_matches_finding(&entry, &finding));

        let changed = test_no_panic_finding("let _ = other.unwrap();", 99);
        assert!(!no_panic_entry_matches_finding(&entry, &changed));
    }

    #[test]
    fn allowlist_count_is_consumed_per_occurrence() {
        let entry = test_no_panic_entry("panic-0002", "let _ = some.unwrap();", 1);
        let findings = vec![
            test_no_panic_finding("let _ = some.unwrap();", 10),
            test_no_panic_finding("let _ = some.unwrap();", 20),
        ];

        let unmatched = match_findings_against_allowlist(&findings, &[entry]);
        assert_eq!(unmatched.len(), 1);
        let Some(finding) = unmatched.first() else {
            return;
        };
        assert_eq!(finding.line, 20);
    }

    #[test]
    fn allowlist_does_not_cover_same_file_same_callee_different_snippet() {
        let entry = test_no_panic_entry("panic-0003", "let _ = first.unwrap();", 1);
        let findings = vec![
            test_no_panic_finding("let _ = first.unwrap();", 10),
            test_no_panic_finding("let _ = second.unwrap();", 20),
        ];

        let unmatched = match_findings_against_allowlist(&findings, &[entry]);
        assert_eq!(unmatched.len(), 1);
        let Some(finding) = unmatched.first() else {
            return;
        };
        assert_eq!(finding.snippet, "let _ = second.unwrap();");
    }

    #[test]
    fn duplicate_allowlist_keys_are_rejected() {
        let policy = r#"
schema_version = "0.4"

[[allow]]
id = "panic-0001"
path = "crates/x/src/lib.rs"
family = "unwrap"
snippet = "let _ = some.unwrap();"
count = 1
classification = "test_helper"
owner = "x"
explanation = "y"
expires = "2027-01-01"

[allow.selector]
kind = "method_call"
callee = "unwrap"

[[allow]]
id = "panic-0002"
path = "crates/x/src/lib.rs"
family = "unwrap"
snippet = "let _ = some.unwrap();"
count = 2
classification = "test_helper"
owner = "x"
explanation = "y"
expires = "2027-01-01"

[allow.selector]
kind = "method_call"
callee = "unwrap"
"#;

        let err = match parse_no_panic_allowlist(policy) {
            Ok(entries) => {
                assert!(
                    entries.is_empty(),
                    "duplicate key should fail, got {entries:?}"
                );
                return;
            }
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("duplicates exact identity"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn no_panic_baseline_refresh_refuses_new_debt_without_reset() -> Result<()> {
        let existing = vec![test_no_panic_baseline_entry("let _ = some.unwrap();", 1)];
        let current = vec![
            test_no_panic_baseline_entry("let _ = some.unwrap();", 1),
            test_no_panic_baseline_entry("let _ = other.unwrap();", 1),
        ];

        let result = refresh_no_panic_baseline_entries(&current, &existing, false);
        if result.is_ok() {
            return Err(anyhow!("baseline refresh should reject new debt"));
        }
        Ok(())
    }

    #[test]
    fn no_panic_baseline_refresh_reports_count_delta() -> Result<()> {
        let existing = vec![test_no_panic_baseline_entry("let _ = some.unwrap();", 1)];
        let current = vec![test_no_panic_baseline_entry("let _ = some.unwrap();", 3)];

        let Err(err) = refresh_no_panic_baseline_entries(&current, &existing, false) else {
            return Err(anyhow!("baseline refresh should reject count growth"));
        };
        let message = err.to_string();
        if !message.contains("current=3 baseline=1 delta=2") {
            return Err(anyhow!("missing count delta in error: {message}"));
        }
        Ok(())
    }

    #[test]
    fn no_panic_baseline_refresh_drops_disappeared_entries() -> Result<()> {
        let existing = vec![
            test_no_panic_baseline_entry("let _ = some.unwrap();", 1),
            test_no_panic_baseline_entry("let _ = gone.unwrap();", 1),
        ];
        let current = vec![test_no_panic_baseline_entry("let _ = some.unwrap();", 1)];

        let refreshed = match refresh_no_panic_baseline_entries(&current, &existing, false) {
            Ok(refreshed) => refreshed,
            Err(err) => return Err(anyhow!("baseline refresh failed unexpectedly: {err}")),
        };
        if refreshed.len() != 1 {
            return Err(anyhow!(
                "expected one refreshed baseline entry, got {}",
                refreshed.len()
            ));
        }
        let Some(entry) = refreshed.first() else {
            return Err(anyhow!("expected refreshed entry"));
        };
        if entry.snippet != "let _ = some.unwrap();" {
            return Err(anyhow!("unexpected refreshed snippet: {}", entry.snippet));
        }
        Ok(())
    }

    #[test]
    fn no_panic_blocking_mode_ignores_baseline_entries() -> Result<()> {
        let existing = vec![test_no_panic_baseline_entry("let _ = some.unwrap();", 1)];
        let effective = effective_no_panic_baseline_entries("blocking", &existing);
        if !effective.is_empty() {
            return Err(anyhow!("blocking mode should ignore baseline entries"));
        }

        let Some(message) = no_panic_blocking_mode_message("blocking", existing.len()) else {
            return Err(anyhow!("blocking mode should produce an operator message"));
        };
        if !message.contains("ignoring 1 baseline entr") {
            return Err(anyhow!("unexpected blocking mode message: {message}"));
        }
        Ok(())
    }

    #[test]
    fn no_panic_baseline_parser_accepts_blocking_mode() -> Result<()> {
        let policy = r#"
schema_version = "1.0"
policy = "no-panic-baseline"
mode = "blocking"
"#;
        let entries = parse_no_panic_baseline(policy)?;
        if !entries.is_empty() {
            return Err(anyhow!("empty blocking baseline should parse no entries"));
        }
        Ok(())
    }

    #[test]
    fn no_panic_report_json_limits_stale_baseline_sample() -> Result<()> {
        let stale_baseline = (0..51)
            .map(|index| {
                let entry =
                    test_no_panic_baseline_entry(&format!("let _ = gone{index}.unwrap();"), 1);
                NoPanicBaselineDelta::from_entry(&entry, 1, 0)
            })
            .collect();
        let report = NoPanicReport {
            baseline_mode: "no-new-debt".into(),
            baseline_ignored: false,
            allowlist_entries: 0,
            baseline_entries: 51,
            baseline_occurrences: 51,
            strict_findings: 0,
            advisory_findings: 0,
            new_debt: Vec::new(),
            stale_allowlist: Vec::new(),
            stale_baseline,
        };

        let json = render_no_panic_report_json(&report);
        if !json.contains("\"stale_baseline_entries_truncated\": true") {
            return Err(anyhow!(
                "JSON report should mark stale baseline sample truncated"
            ));
        }
        if !json.contains("gone49") {
            return Err(anyhow!(
                "JSON report should include the fiftieth stale entry"
            ));
        }
        if json.contains("gone50") {
            return Err(anyhow!(
                "JSON report should not include the fifty-first stale entry"
            ));
        }
        Ok(())
    }
}
