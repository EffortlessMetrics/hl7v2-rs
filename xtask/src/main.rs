use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::collections::HashSet;
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
    Gate {
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gate { changed, only } => gate(changed, only)?,
        Commands::LintFix => lint_fix()?,
        Commands::Setup => setup()?,
        Commands::Audit => audit()?,
        Commands::Outdated => outdated()?,
        Commands::Scaffold { name, description } => scaffold(&name, description)?,
        Commands::Docs { no_open } => docs(no_open)?,
    }

    Ok(())
}

fn gate(changed_only: bool, only: Option<String>) -> Result<()> {
    println!("🚀 Running gate checks...");

    let crates = if changed_only {
        get_changed_crates()?
    } else {
        vec![]
    };

    if changed_only && crates.is_empty() {
        println!("No crates changed. Skipping checks.");
        return Ok(());
    }

    let run_fmt = only.as_deref().map_or(true, |s| s == "fmt");
    let run_clippy = only.as_deref().map_or(true, |s| s == "clippy");
    let run_test = only.as_deref().map_or(true, |s| s == "test");

    if run_fmt {
        println!("Checking formatting...");
        run_command("cargo", &["fmt", "--all", "--", "--check"])?;
    }

    if run_clippy {
        println!("Running clippy...");
        let mut args = vec![
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ];
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

    if run_test {
        println!("Running tests...");
        let mut args = vec!["test", "--all-features"];
        if changed_only {
            for c in &crates {
                args.push("-p");
                args.push(c);
            }
        } else {
            args.push("--workspace");
        }
        
        // Use nextest if available
        if command_exists("cargo-nextest") {
            let mut nextest_args = vec!["nextest", "run", "--all-features"];
            if changed_only {
                for c in &crates {
                    nextest_args.push("-p");
                    nextest_args.push(c);
                }
            } else {
                nextest_args.push("--workspace");
            }
            run_command("cargo", &nextest_args)?;
        } else {
            run_command("cargo", &args)?;
        }
    }

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
            "--workspace",
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

    // Create .githooks if it doesn't exist
    if !hooks_src.exists() {
        fs::create_dir_all(&hooks_src)?;
        println!("Created .githooks directory");
    }

    // Ensure pre-commit hook exists
    let pre_commit_path = hooks_src.join("pre-commit");
    if !pre_commit_path.exists() {
        fs::write(&pre_commit_path, r#"#!/usr/bin/env bash
set -e
echo "Running pre-commit checks..."
cargo run -p xtask -- gate --check
"#)?;
        println!("Created default pre-commit hook");
    }

    if !hooks_dst.exists() {
        println!("Warning: .git/hooks not found. Are you in a git repository?");
    } else {
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
    }

    // Check for required tools
    let tools = ["cargo-deny", "cargo-audit", "cargo-nextest", "just"];
    for tool in tools {
        if !command_exists(tool) {
            println!("Note: '{}' not found. Consider installing it for full DevEx.", tool);
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
    let cargo_toml = format!(r#"[package]
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
"#);
    fs::write(crate_path.join("Cargo.toml"), cargo_toml)?;

    // README.md
    let readme = format!(r#"# {crate_name}

{description}

## Usage

```rust
use {crate_name}::*;
```
"#);
    fs::write(crate_path.join("README.md"), readme)?;

    // CLAUDE.md
    let claude = format!(r#"# {crate_name} Development

## Build & Test

```bash
cargo build -p {crate_name}
cargo test -p {crate_name}
cargo clippy -p {crate_name} -- -D warnings
```
"#);
    fs::write(crate_path.join("CLAUDE.md"), claude)?;

    // src/lib.rs
    fs::write(crate_path.join("src").join("lib.rs"), r#"//! Main library file
    
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
"#)?;

    println!("✅ Crate {} scaffolded successfully!", crate_name);
    println!("Don't forget to run 'cargo build' to update the workspace.");

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

fn command_exists(cmd: &str) -> bool {
    let cmd = if cfg!(windows) {
        format!("{}.exe", cmd)
    } else {
        cmd.to_string()
    };

    Command::new("where")
        .arg(&cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn get_changed_crates() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(&["diff", "--name-only", "HEAD"])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("Failed to run git diff"));
    }

    let files = String::from_utf8(output.stdout)?;
    let mut changed_crates = HashSet::new();

    for line in files.lines() {
        if line.starts_with("crates/") {
            let parts: Vec<&str> = line.split('/').collect();
            if parts.len() > 1 {
                changed_crates.insert(parts[1].to_string());
            }
        }
    }

    Ok(changed_crates.into_iter().collect())
}
