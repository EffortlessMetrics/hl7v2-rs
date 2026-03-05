use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::process::{Command, Stdio};

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
    Gate,
    /// Fix formatting and common clippy issues
    LintFix,
    /// Setup development environment (git hooks, etc.)
    Setup,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gate => gate()?,
        Commands::LintFix => lint_fix()?,
        Commands::Setup => setup()?,
    }

    Ok(())
}

fn gate() -> Result<()> {
    println!("🚀 Running gate checks...");

    println!("Checking formatting...");
    run_command("cargo", &["fmt", "--all", "--", "--check"])?;

    println!("Running clippy...");
    run_command(
        "cargo",
        &[
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    println!("Running tests...");
    run_command("cargo", &["test", "--all"])?;

    println!("✅ Gate checks passed!");
    Ok(())
}

fn lint_fix() -> Result<()> {
    println!("🛠️  Fixing lints and formatting...");

    println!("Formatting code...");
    run_command("cargo", &["fmt", "--all"])?;

    println!("Applying clippy fixes...");
    run_command(
        "cargo",
        &[
            "clippy",
            "--fix",
            "--allow-dirty",
            "--allow-staged",
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
    println!("⚙️  Setting up development environment...");

    let root = env::current_dir()?;
    let hooks_src = root.join(".githooks");
    let hooks_dst = root.join(".git").join("hooks");

    if !hooks_src.exists() {
        return Err(anyhow!(
            ".githooks directory not found. Please create it first."
        ));
    }

    if !hooks_dst.exists() {
        return Err(anyhow!(
            ".git/hooks directory not found. Is this a git repository?"
        ));
    }

    for entry in fs::read_dir(hooks_src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap();
            let dest = hooks_dst.join(file_name);
            println!("Installing hook: {:?}", file_name);
            fs::copy(&path, &dest)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&dest)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&dest, perms)?;
            }
        }
    }

    println!("✅ Setup complete!");
    Ok(())
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
