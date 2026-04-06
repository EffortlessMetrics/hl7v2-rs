//! Workspace dependency alignment tests
//!
//! These tests enforce the EFF-1136 regression boundary only.
//!
//! EFF-1136 is specifically about the tokio dev-dependency in hl7v2-gen.

// Test code uses nested if statements for readability - allow this pattern
#![allow(clippy::collapsible_if)]

use std::fs;
use std::path::Path;

/// Issue-scoped dependency set for EFF-1136.
const ISSUE_SCOPED_DEPS: &[&str] = &["tokio"];

/// Parse a Cargo.toml and return dependencies that have hardcoded versions
/// instead of using workspace = true
fn find_hardcoded_dependencies<P: AsRef<Path>>(cargo_toml_path: P) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read {:?}: {}", cargo_toml_path.as_ref(), e))?;

    let manifest: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {:?}: {}", cargo_toml_path.as_ref(), e))?;

    let mut hardcoded = Vec::new();

    // Check [dependencies] section
    if let Some(deps) = manifest.get("dependencies") {
        if let Some(deps_table) = deps.as_table() {
            for dep_name in ISSUE_SCOPED_DEPS {
                if let Some(dep_value) = deps_table.get(*dep_name) {
                    if is_hardcoded_version(dep_value) {
                        hardcoded.push(dep_name.to_string());
                    }
                }
            }
        }
    }

    // Check [dev-dependencies] section
    if let Some(dev_deps) = manifest.get("dev-dependencies") {
        if let Some(deps_table) = dev_deps.as_table() {
            for dep_name in ISSUE_SCOPED_DEPS {
                if let Some(dep_value) = deps_table.get(*dep_name) {
                    if is_hardcoded_version(dep_value) {
                        hardcoded.push(format!("{} (dev)", dep_name));
                    }
                }
            }
        }
    }

    // Check [target.*.dependencies] sections
    if let Some(target) = manifest.get("target") {
        if let Some(target_table) = target.as_table() {
            for (_, target_deps) in target_table {
                if let Some(deps) = target_deps.get("dependencies") {
                    if let Some(deps_table) = deps.as_table() {
                        for dep_name in ISSUE_SCOPED_DEPS {
                            if let Some(dep_value) = deps_table.get(*dep_name) {
                                if is_hardcoded_version(dep_value) {
                                    hardcoded.push(format!("{} (target)", dep_name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(hardcoded)
}

/// Check if a dependency value has a hardcoded version instead of workspace = true
fn is_hardcoded_version(dep_value: &toml::Value) -> bool {
    match dep_value {
        // Simple version string: tokio = "1.0"
        toml::Value::String(_) => true,
        // Table with version field: tokio = { version = "1.0", ... }
        toml::Value::Table(table) => {
            // If it has workspace = true, it's OK
            if let Some(workspace) = table.get("workspace") {
                if workspace.as_bool() == Some(true) {
                    return false;
                }
            }
            // If it has a version field, it's hardcoded
            if table.contains_key("version") {
                return true;
            }
            // Path-only dependencies are OK (they don't specify versions)
            // Check for git dependencies - those are considered hardcoded too
            if table.contains_key("git") {
                return true;
            }
            false
        }
        _ => false,
    }
}

/// Get all workspace crate Cargo.toml paths
#[allow(dead_code)] // Used by commented-out cross-crate tests
fn get_workspace_cargo_tomls() -> Vec<std::path::PathBuf> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("hl7v2-gen is in a subdirectory")
        .parent()
        .expect("crates is in workspace root");

    let mut paths = Vec::new();

    // Read workspace members from root Cargo.toml
    let root_cargo_toml = workspace_root.join("Cargo.toml");
    if let Ok(content) = fs::read_to_string(&root_cargo_toml) {
        if let Ok(manifest) = toml::from_str::<toml::Value>(&content) {
            if let Some(workspace) = manifest.get("workspace") {
                if let Some(members) = workspace.get("members") {
                    if let Some(members_array) = members.as_array() {
                        for member in members_array {
                            if let Some(member_str) = member.as_str() {
                                let member_path = workspace_root.join(member_str);
                                let cargo_toml = member_path.join("Cargo.toml");
                                if cargo_toml.exists() {
                                    paths.push(cargo_toml);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    paths
}

// =============================================================================
// EFF-1136: Tokio Dependency Alignment Tests
// =============================================================================

#[test]
fn hl7v2_gen_tokio_uses_workspace_version() {
    //! EFF-1136: Verify tokio in hl7v2-gen uses workspace = true
    //!
    //! This test ensures the fix for tokio version drift is in place.
    //! It will FAIL if tokio reverts to hardcoded version like "1.49.0".

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = crate_dir.join("Cargo.toml");

    let hardcoded = find_hardcoded_dependencies(&cargo_toml).expect("Failed to parse Cargo.toml");

    let tokio_hardcoded: Vec<&String> = hardcoded
        .iter()
        .filter(|d| d.starts_with("tokio"))
        .collect();

    assert!(
        tokio_hardcoded.is_empty(),
        "EFF-1136 REGRESSION: tokio in hl7v2-gen must use workspace = true, \
         but found hardcoded version. \
         Violations: {:?}",
        tokio_hardcoded
    );
}

#[test]
fn hl7v2_gen_has_no_hardcoded_tokio_version() {
    //! EFF-1136: Ensure no hardcoded tokio version exists in hl7v2-gen
    //!
    //! Complementary test to verify the tokio dependency alignment.

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = crate_dir.join("Cargo.toml");

    let hardcoded = find_hardcoded_dependencies(&cargo_toml).expect("Failed to parse Cargo.toml");

    // Filter for tokio entries only
    let tokio_violations: Vec<&String> = hardcoded
        .iter()
        .filter(|d| d.starts_with("tokio"))
        .collect();

    assert!(
        tokio_violations.is_empty(),
        "EFF-1136 REGRESSION: Found hardcoded tokio version in hl7v2-gen: {:?}",
        tokio_violations
    );
}

// =============================================================================
// Future: Cross-Crate Dependency Alignment Tests (EFF-1137+)
// =============================================================================
// These tests are commented out pending the broader dependency alignment epic.
// Uncomment and enable when ready to enforce workspace = true across all crates.
//
// #[test]
// fn all_workspace_crates_use_workspace_dependencies() {
//     let cargo_tomls = get_workspace_cargo_tomls();
//     let mut all_violations = Vec::new();
//
//     for cargo_toml in cargo_tomls {
//         match find_hardcoded_dependencies(&cargo_toml) {
//             Ok(violations) => {
//                 if !violations.is_empty() {
//                     let crate_name = cargo_toml
//                         .parent()
//                         .and_then(|p| p.file_name())
//                         .and_then(|n| n.to_str())
//                         .unwrap_or("unknown");
//                     all_violations.push((crate_name.to_string(), violations));
//                 }
//             }
//             Err(e) => {
//                 // Log but don't fail - test data may not be valid TOML
//                 eprintln!("Warning: Could not check {:?}: {}", cargo_toml, e);
//             }
//         }
//     }
//
//     assert!(
//         all_violations.is_empty(),
//         "Found hardcoded dependencies in workspace crates: {:?}",
//         all_violations
//     );
// }
