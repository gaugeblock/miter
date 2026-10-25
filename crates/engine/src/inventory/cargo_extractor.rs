//! Cargo workspace and package manifest extractor.

use std::fs;
use std::path::{Path, PathBuf};

use crate::inventory::cargo_parser::{collect_member_crates, parse_single_crate};
use crate::inventory::error::InventoryError;
use crate::inventory::toolchain_extractor::extract_toolchain;
use crate::inventory::types::{CrateInventory, ToolchainInventory};

/// Discovered cargo manifests and toolchain config.
#[derive(Debug, Clone)]
pub struct CargoExtraction {
    /// Workspace name if package defined at root.
    pub workspace_name: Option<String>,
    /// Discovered crate manifests.
    pub crates: Vec<CrateInventory>,
    /// Discovered toolchain metadata.
    pub toolchain: ToolchainInventory,
}

/// Extracts crate manifests and toolchain metadata from a cargo root.
pub fn extract_cargo_inventory(root_dir: &Path) -> Result<CargoExtraction, InventoryError> {
    let manifest_path = root_dir.join("Cargo.toml");
    if !manifest_path.is_file() {
        return discover_multi_repo_cargo(root_dir);
    }

    let manifest_str = fs::read_to_string(&manifest_path)?;
    let parsed: toml::Value = toml::from_str(&manifest_str)?;

    let mut toolchain = extract_toolchain(root_dir, &parsed);
    let mut crates = Vec::new();
    let mut workspace_name = None;

    let ws_package = parsed.get("workspace").and_then(|w| w.get("package"));
    let ws_version = ws_package
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let ws_license = ws_package
        .and_then(|p| p.get("license"))
        .and_then(|l| l.as_str())
        .map(String::from);

    if let Some(pkg) = parsed.get("package") {
        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        workspace_name = Some(name.clone());
        crates.push(parse_single_crate(
            root_dir,
            Path::new(""),
            pkg,
            parsed.get("dependencies"),
            ws_version.as_deref(),
            ws_license.as_deref(),
        ));
    }

    if let Some(members) = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    {
        for member_val in members {
            if let Some(pattern) = member_val.as_str() {
                if pattern == "." {
                    continue;
                }
                collect_member_crates(
                    root_dir,
                    pattern,
                    ws_version.as_deref(),
                    ws_license.as_deref(),
                    &mut crates,
                )?;
            }
        }
    }

    crates.sort_by(|a, b| a.name.cmp(&b.name));
    crates.dedup_by(|a, b| a.name == b.name);

    if toolchain.rust_edition.is_none() {
        let ed = ws_package
            .and_then(|p| p.get("edition"))
            .and_then(|e| e.as_str())
            .or_else(|| {
                parsed
                    .get("package")
                    .and_then(|p| p.get("edition"))
                    .and_then(|e| e.as_str())
            });
        if let Some(e) = ed {
            toolchain.rust_edition = Some(e.to_string());
        }
    }

    Ok(CargoExtraction {
        workspace_name,
        crates,
        toolchain,
    })
}

fn discover_multi_repo_cargo(root_dir: &Path) -> Result<CargoExtraction, InventoryError> {
    let mut all_crates = Vec::new();
    let mut toolchain = ToolchainInventory::default();

    let entries = fs::read_dir(root_dir).map_err(|_| {
        InventoryError::InvalidWorkspace(format!("Missing Cargo.toml at {}", root_dir.display()))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").is_file() {
            let child_name = entry.file_name();
            let child_rel = PathBuf::from(&child_name);
            if let Ok(sub_ext) = extract_cargo_inventory(&path) {
                if toolchain.rust_edition.is_none() && sub_ext.toolchain.rust_edition.is_some() {
                    toolchain = sub_ext.toolchain;
                }
                for mut cr in sub_ext.crates {
                    cr.relative_path = child_rel.join(&cr.relative_path);
                    all_crates.push(cr);
                }
            }
        }
    }

    if all_crates.is_empty() {
        return Err(InventoryError::InvalidWorkspace(format!(
            "Missing Cargo.toml at {}",
            root_dir.display()
        )));
    }

    all_crates.sort_by(|a, b| a.name.cmp(&b.name));
    all_crates.dedup_by(|a, b| a.name == b.name);

    let workspace_name = root_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string());

    Ok(CargoExtraction {
        workspace_name,
        crates: all_crates,
        toolchain,
    })
}
