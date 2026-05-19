//! Command-line interface definitions for the xtask binary.

use crate::publish::PublishSurface;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development automation tasks", long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
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
    /// Print the primary Rust product crates.io publish order
    PublishPlan {
        /// Resume from this crate name
        #[arg(long)]
        from: Option<String>,
        /// Package surface to report
        #[arg(long, value_enum, default_value = "primary")]
        surface: PublishSurface,
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
        /// Package surface to dry-run
        #[arg(long, value_enum, default_value = "primary")]
        surface: PublishSurface,
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
    /// Verify spec index and spec-linked policy/proof references stay in sync
    CheckSpecPolicyLinks,
    /// Verify Python TestPyPI/PyPI release workflow safety controls
    CheckPythonPublishPolicy,
    /// Build, install, and smoke-test the Python hl7v2 wheel in a local venv
    PythonLocalWheelProof {
        /// Scratch root for the wheel, venv, and cargo target artifacts
        #[arg(long)]
        root: Option<std::path::PathBuf>,
        /// Python launcher/interpreter used to create the proof virtualenv
        #[arg(long, default_value = "python")]
        python: String,
        /// Rust toolchain used by maturin when building the wheel
        #[arg(long, default_value = "1.95.0")]
        rust_toolchain: String,
        /// Keep the previous scratch root instead of deleting it first
        #[arg(long)]
        keep_existing: bool,
    },
    /// Install public Python hl7v2 from TestPyPI/PyPI and run smoke proof
    PythonPublicRegistryProof {
        /// Scratch root for the proof virtualenv
        #[arg(long)]
        root: Option<std::path::PathBuf>,
        /// Python launcher/interpreter used to create the proof virtualenv
        #[arg(long, default_value = "python")]
        python: String,
        /// Package index to install from
        #[arg(long, value_enum, default_value = "testpypi")]
        index: PythonPackageIndex,
        /// Public hl7v2 package version to install; defaults to the workspace version
        #[arg(long)]
        version: Option<String>,
        /// Keep the previous scratch root instead of deleting it first
        #[arg(long)]
        keep_existing: bool,
    },
    /// Verify CI lane whitelist: coverage, required fields, expensive-default exceptions
    CheckCiLaneWhitelist,
    /// Verify cross-surface evidence parity manifest state and non-claim boundaries
    CheckEvidenceParity,
    /// Run the local cross-surface evidence parity acceptance suite
    CheckEvidenceParityAcceptance {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
    },
    /// Run executable first-use guide checks for the documented receipt path
    CheckFirstUseGuides {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
        /// Also run public crates.io install-back smoke checks
        #[arg(long)]
        include_public_crates: bool,
    },
    /// Run the executable First 10 Minutes guide smoke
    #[command(name = "check-first-10-minutes-guide")]
    CheckFirst10MinutesGuide,
    /// Run the executable First Use By Surface guide smoke
    CheckFirstUseBySurfaceGuide,
    /// Run the executable Vendor Upgrade Diff guide smoke
    CheckVendorUpgradeDiffGuide,
    /// Run the executable operator error guidance smoke
    CheckOperatorErrorGuidanceGuide,
    /// Run the executable safe support-bundle guide smoke
    CheckSafeSupportBundleGuide,
    /// Run the executable evidence-artifact interpretation guide smoke
    CheckEvidenceArtifactsGuide,
    /// Start a local sidecar and run the executable deployment guide smoke
    CheckSidecarGuide,
    /// Run fixture-backed safe-error and PHI parity acceptance checks
    CheckSafeErrorPhiParity {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
    },
    /// Run profile lint/explain/test parity acceptance checks
    CheckProfileParity {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
    },
    /// Run fixture-backed schema-version parity acceptance checks
    CheckSchemaVersionParity {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
    },
    /// Run fixture-backed dirty-corpus parity acceptance checks
    CheckDirtyCorpusParity {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
    },
    /// Run bundle/replay parity acceptance checks
    CheckBundleReplayParity {
        /// Also run Python local-wheel smoke checks when hl7v2 is installed
        #[arg(long)]
        include_python: bool,
    },
    /// Validate checked-in evidence fixtures against their JSON schemas
    EvidenceSchemaCheck,
    /// Generate repo-scoped public badge endpoint JSON
    Badges {
        /// Verify committed badges are current.
        #[arg(long)]
        check: bool,
    },
    /// Generate the diff-scoped RIPR PR evidence packet
    RiprPr {
        /// Workspace root passed to ripr.
        #[arg(long, default_value = ".")]
        root: String,
        /// Base revision for the PR diff.
        #[arg(long, default_value = "origin/main")]
        base: String,
        /// Head revision for the PR diff.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Verify generated artifacts already exist and are contract-valid.
        #[arg(long)]
        check: bool,
    },
    /// Generate bounded RIPR review guidance without posting to GitHub
    RiprReviewComments {
        /// Workspace root passed to ripr.
        #[arg(long, default_value = ".")]
        root: String,
        /// Base revision for the PR diff.
        #[arg(long, default_value = "origin/main")]
        base: String,
        /// Head revision for the PR diff.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Verify generated artifacts already exist and are contract-valid.
        #[arg(long)]
        check: bool,
    },
    /// Generate a stable Markdown summary from PR evidence artifacts
    RiprPrSummary {
        /// Verify the summary is current.
        #[arg(long)]
        check: bool,
    },
    /// Emit non-blocking GitHub warning annotations from review comments
    RiprAnnotations {
        /// Review comments JSON input.
        #[arg(long, default_value = "target/ripr/review/comments.json")]
        comments: String,
        /// Annotation command output path.
        #[arg(long, default_value = "target/ripr/review/annotations.txt")]
        out: String,
        /// Verify generated annotations are current.
        #[arg(long)]
        check: bool,
    },
    /// Generate impacted evidence and mutation routing receipt
    ImpactedEvidence {
        /// PR evidence JSON input.
        #[arg(long, default_value = "target/ripr/pr/repo-exposure.json")]
        pr_evidence: String,
        /// Add one PR label.
        #[arg(long)]
        label: Vec<String>,
        /// Add comma, semicolon, or newline separated PR labels.
        #[arg(long)]
        labels: Option<String>,
        /// Verify generated impacted evidence is current.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum PythonPackageIndex {
    /// Install from TestPyPI
    Testpypi,
    /// Install from production PyPI
    Pypi,
}

#[derive(Subcommand)]
pub(crate) enum NoPanicAction {
    /// Emit proposed allowlist entries for current findings
    Propose {
        /// Include staged (non-required) crates as well
        #[arg(long)]
        include_staged: bool,
    },
    /// Refresh the no-new-debt baseline
    Baseline {
        /// Absorb all current findings into the baseline
        #[arg(long)]
        reset: bool,
    },
}
