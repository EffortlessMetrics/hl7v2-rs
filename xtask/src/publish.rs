//! Publish planning and crates.io release verification tasks.

use crate::escape_toml_basic_string;
use anyhow::{Result, anyhow};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};
use clap::ValueEnum;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

pub(crate) const PRIMARY_RUST_PRODUCT_CRATES: &[&str] = &["hl7v2", "hl7v2-server", "hl7v2-cli"];
pub(crate) const BINDING_BACKEND_CRATES: &[&str] = &["hl7v2-python"];
pub(crate) const EXCLUDED_PUBLISHABLE_WORKSPACE_PACKAGES: &[&str] = &["xtask", "hl7v2-examples"];

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum PublishSurface {
    /// Primary Rust API/operator crates.
    Primary,
    /// Binding backend crates for foreign-language packages.
    Bindings,
    /// Primary Rust product crates plus publishable binding backend crates.
    AllPublishable,
}

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

pub(crate) fn publish_dry_run(
    from: Option<String>,
    surface: PublishSurface,
    workspace_patches: bool,
    allow_dirty: bool,
) -> Result<()> {
    if surface == PublishSurface::Bindings {
        return binding_backend_dry_run(from.as_deref(), workspace_patches, allow_dirty);
    }

    let metadata = MetadataCommand::new().exec()?;
    let packages = publishable_workspace_packages_for_surface(&metadata, surface)?;
    let ordered = topological_publish_order(&packages)?;
    let crates = resume_publish_order(&ordered, from.as_deref())?;

    println!("🧪 Dry-running {} verification", surface.dry_run_label());
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

fn binding_backend_dry_run(
    from: Option<&str>,
    workspace_patches: bool,
    allow_dirty: bool,
) -> Result<()> {
    let metadata = MetadataCommand::new().exec()?;
    let targets = binding_backend_dry_run_targets(&metadata, from)?;
    if targets.is_empty() {
        println!("No binding backend crates are present in this workspace.");
        return Ok(());
    }

    let packages =
        publishable_workspace_packages_for_surface(&metadata, PublishSurface::AllPublishable)?;

    println!("🧪 Dry-running binding backend crates.io package proof");
    if workspace_patches {
        println!("Using local workspace patches for unpublished internal crates.");
    }

    for target in targets {
        package_list_crate(&target.name, allow_dirty)?;
        if !target.publishable {
            return Err(anyhow!(
                "{} is classified as a binding backend but is not publishable yet (publish = false). Remove publish = false only in a dedicated binding-backend release PR after metadata, dry-run tooling, and release receipts are ready.",
                target.name
            ));
        }

        let config_path = if workspace_patches {
            workspace_patch_config(&target.name, &packages)?
        } else {
            None
        };
        publish_dry_run_crate(&target.name, config_path.as_deref(), allow_dirty)?;
    }

    println!("✅ Binding backend dry-run checks passed!");
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BindingBackendDryRunTarget {
    pub(crate) name: String,
    pub(crate) publishable: bool,
}

pub(crate) fn binding_backend_dry_run_targets(
    metadata: &Metadata,
    from: Option<&str>,
) -> Result<Vec<BindingBackendDryRunTarget>> {
    publishable_workspace_packages_for_surface(metadata, PublishSurface::Bindings)?;

    let packages = workspace_member_packages(metadata);
    let mut targets = Vec::new();
    for crate_name in BINDING_BACKEND_CRATES {
        if let Some(package) = packages.get(*crate_name) {
            targets.push(BindingBackendDryRunTarget {
                name: package.name.to_string(),
                publishable: package_is_publishable(package),
            });
        }
    }

    match from {
        Some(start) => {
            let index = targets
                .iter()
                .position(|target| target.name == start)
                .ok_or_else(|| anyhow!("Unknown binding backend crate '{start}'"))?;
            let resumed = targets.get(index..).ok_or_else(|| {
                anyhow!("resume index for {start} is outside binding backend graph")
            })?;
            Ok(resumed.to_vec())
        }
        None => Ok(targets),
    }
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

impl PublishSurface {
    fn publish_plan_heading(self) -> &'static str {
        match self {
            Self::Primary => "Primary Rust product crates.io publish order",
            Self::Bindings => "Binding backend crates.io publish order",
            Self::AllPublishable => "All publishable crates.io publish order",
        }
    }

    fn dry_run_label(self) -> &'static str {
        match self {
            Self::Primary => "primary Rust product crates.io publish",
            Self::Bindings => "binding backend crates.io package",
            Self::AllPublishable => "all publishable crates.io package",
        }
    }
}

pub(crate) fn publishable_workspace_packages_for_surface(
    metadata: &Metadata,
    surface: PublishSurface,
) -> Result<HashMap<String, Package>> {
    let packages = workspace_member_packages(metadata);
    ensure_publishable_workspace_packages_are_classified(&packages)?;

    let selected: BTreeSet<&str> = match surface {
        PublishSurface::Primary => PRIMARY_RUST_PRODUCT_CRATES.iter().copied().collect(),
        PublishSurface::Bindings => BINDING_BACKEND_CRATES
            .iter()
            .copied()
            .filter(|name| packages.get(*name).is_some_and(package_is_publishable))
            .collect(),
        PublishSurface::AllPublishable => PRIMARY_RUST_PRODUCT_CRATES
            .iter()
            .chain(
                BINDING_BACKEND_CRATES
                    .iter()
                    .filter(|name| packages.get(**name).is_some_and(package_is_publishable)),
            )
            .copied()
            .collect(),
    };

    let mut selected_packages = HashMap::new();
    for package_name in selected {
        let package = packages
            .get(package_name)
            .ok_or_else(|| anyhow!("workspace package {package_name} is missing"))?;
        if !package_is_publishable(package) {
            return Err(anyhow!(
                "workspace package {package_name} is selected for {surface:?} but is not publishable"
            ));
        }
        selected_packages.insert(package.name.to_string(), package.clone());
    }

    Ok(selected_packages)
}

pub(crate) fn ensure_publishable_workspace_packages_are_classified(
    packages: &HashMap<String, Package>,
) -> Result<()> {
    let classified: BTreeSet<&str> = PRIMARY_RUST_PRODUCT_CRATES
        .iter()
        .chain(BINDING_BACKEND_CRATES.iter())
        .copied()
        .collect();
    let unclassified: Vec<_> = packages
        .values()
        .filter(|package| package_is_publishable(package))
        .map(|package| package.name.as_str())
        .filter(|package_name| !classified.contains(package_name))
        .collect();
    if !unclassified.is_empty() {
        return Err(anyhow!(
            "publishable workspace package(s) are missing publish surface classification: {}",
            unclassified.join(", ")
        ));
    }

    Ok(())
}

pub(crate) fn workspace_member_packages(metadata: &Metadata) -> HashMap<String, Package> {
    let workspace_members: HashSet<_> = metadata.workspace_members.iter().cloned().collect();

    metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(&pkg.id))
        .filter(|pkg| {
            !EXCLUDED_PUBLISHABLE_WORKSPACE_PACKAGES
                .iter()
                .any(|excluded| *excluded == pkg.name)
        })
        .cloned()
        .map(|pkg| (pkg.name.to_string(), pkg))
        .collect()
}

pub(crate) fn package_is_publishable(package: &Package) -> bool {
    package
        .publish
        .as_ref()
        .is_none_or(|registries| !registries.is_empty())
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

pub(crate) fn internal_workspace_dependency_closure(
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
