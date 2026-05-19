use super::{
    BINDING_BACKEND_CRATES, EXCLUDED_PUBLISHABLE_WORKSPACE_PACKAGES, PRIMARY_RUST_PRODUCT_CRATES,
    PublishSurface,
};
use anyhow::{Result, anyhow};
use cargo_metadata::{DependencyKind, Metadata, Package};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

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

pub(crate) fn topological_publish_order(
    packages: &HashMap<String, Package>,
) -> Result<Vec<String>> {
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
