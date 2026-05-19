//! Publish planning and crates.io release verification tasks.

mod dry_run;
mod graph;
mod surface;

use crate::escape_toml_basic_string;
use anyhow::{Result, anyhow};
use cargo_metadata::{Metadata, MetadataCommand, Package};
pub(crate) use dry_run::{
    BindingBackendDryRunTarget, binding_backend_dry_run_targets, publish_dry_run,
};
pub(crate) use graph::{
    ensure_publishable_workspace_packages_are_classified, internal_workspace_dependency_closure,
    package_is_publishable, publishable_workspace_packages_for_surface, topological_publish_order,
    workspace_member_packages,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
pub(crate) use surface::PublishSurface;

pub(crate) const PRIMARY_RUST_PRODUCT_CRATES: &[&str] = &["hl7v2", "hl7v2-server", "hl7v2-cli"];
pub(crate) const BINDING_BACKEND_CRATES: &[&str] = &["hl7v2-python"];
pub(crate) const EXCLUDED_PUBLISHABLE_WORKSPACE_PACKAGES: &[&str] = &["xtask", "hl7v2-examples"];

pub(crate) fn publish_plan(from: Option<String>, surface: PublishSurface) -> Result<()> {
    let crates = publish_order_for_surface(surface, from.as_deref())?;
    let metadata = MetadataCommand::new().exec()?;

    if surface == PublishSurface::Primary {
        println!("📋 Primary Rust product crates.io publish order");
    } else {
        println!("{}", surface.publish_plan_heading());
    }
    print_numbered_crates(&crates)?;

    if surface != PublishSurface::Bindings {
        print_binding_backend_status(&metadata)?;
    } else if crates.is_empty() {
        println!("No publishable binding backend crates are currently enabled.");
        print_binding_backend_status(&metadata)?;
    }

    println!();
    if surface == PublishSurface::Primary {
        println!("Execute with:");
        if let Some(start) = crates.first() {
            println!("  cargo run -p xtask -- publish --yes --from {start}");
        } else {
            println!("  cargo run -p xtask -- publish --yes");
        }
    } else {
        println!(
            "Publishing non-primary surfaces requires an explicit release decision and dedicated tooling."
        );
    }

    Ok(())
}

pub(crate) fn publish(
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

fn package_list_crate(crate_name: &str, allow_dirty: bool) -> Result<()> {
    println!("Listing package files for {crate_name}...");

    let mut command = Command::new("cargo");
    command.args(["package", "--list", "-p", crate_name, "--locked"]);
    if allow_dirty {
        command.arg("--allow-dirty");
    }

    let status = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!(
            "Package file listing failed for {} with exit code: {:?}",
            crate_name,
            status.code()
        ));
    }

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

pub(crate) fn publish_order(from: Option<&str>) -> Result<Vec<String>> {
    publish_order_for_surface(PublishSurface::Primary, from)
}

pub(crate) fn publish_order_for_surface(
    surface: PublishSurface,
    from: Option<&str>,
) -> Result<Vec<String>> {
    let metadata = MetadataCommand::new().exec()?;
    let packages = publishable_workspace_packages_for_surface(&metadata, surface)?;
    let ordered = topological_publish_order(&packages)?;

    resume_publish_order(&ordered, from)
}

fn print_numbered_crates(crates: &[String]) -> Result<()> {
    for (index, crate_name) in crates.iter().enumerate() {
        let display_index = index
            .checked_add(1)
            .ok_or_else(|| anyhow!("publish-plan index overflow"))?;
        println!("{display_index:>2}. {crate_name}");
    }
    Ok(())
}

fn print_binding_backend_status(metadata: &Metadata) -> Result<()> {
    println!();
    println!("Binding backend graph:");
    let packages = workspace_member_packages(metadata);
    for crate_name in BINDING_BACKEND_CRATES {
        match packages.get(*crate_name) {
            Some(package) if package_is_publishable(package) => {
                println!(" - {crate_name} (publishable binding backend)");
            }
            Some(_) => {
                println!(" - {crate_name} (publish = false)");
            }
            None => {
                println!(" - {crate_name} (not present)");
            }
        }
    }
    Ok(())
}

pub(crate) fn resume_publish_order(ordered: &[String], from: Option<&str>) -> Result<Vec<String>> {
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
