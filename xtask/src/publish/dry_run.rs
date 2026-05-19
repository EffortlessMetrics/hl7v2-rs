use super::{
    BINDING_BACKEND_CRATES, PublishSurface, package_is_publishable, publish_dry_run_crate,
    publishable_workspace_packages_for_surface, workspace_member_packages, workspace_patch_config,
};
use anyhow::{Result, anyhow};
use cargo_metadata::{Metadata, MetadataCommand};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BindingBackendDryRunTarget {
    pub(crate) name: String,
    pub(crate) publishable: bool,
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
    let ordered = super::topological_publish_order(&packages)?;
    let crates = super::resume_publish_order(&ordered, from.as_deref())?;

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
        super::package_list_crate(&target.name, allow_dirty)?;
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
