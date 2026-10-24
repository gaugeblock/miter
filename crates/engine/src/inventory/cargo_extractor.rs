//! Cargo workspace and package manifest extractor.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
        return Err(InventoryError::InvalidWorkspace(format!(
            "Missing Cargo.toml at {}",
            root_dir.display()
        )));
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

fn collect_member_crates(
    root: &Path,
    pattern: &str,
    ws_version: Option<&str>,
    ws_license: Option<&str>,
    crates: &mut Vec<CrateInventory>,
) -> Result<(), InventoryError> {
    if pattern.ends_with("/*") {
        let parent = pattern.trim_end_matches("/*");
        let parent_path = root.join(parent);
        if parent_path.is_dir() {
            for entry in fs::read_dir(parent_path)? {
                let entry = entry?;
                let path = entry.path();
                let manifest = path.join("Cargo.toml");
                if manifest.is_file() {
                    let rel = path
                        .strip_prefix(root)
                        .map_err(|e| InventoryError::InvalidWorkspace(e.to_string()))?;
                    if let Ok(c) = read_crate_manifest(root, rel, ws_version, ws_license) {
                        crates.push(c);
                    }
                }
            }
        }
    } else {
        let path = root.join(pattern);
        let manifest = path.join("Cargo.toml");
        if manifest.is_file() {
            let rel = Path::new(pattern);
            if let Ok(c) = read_crate_manifest(root, rel, ws_version, ws_license) {
                crates.push(c);
            }
        }
    }
    Ok(())
}

fn read_crate_manifest(
    root: &Path,
    rel: &Path,
    ws_version: Option<&str>,
    ws_license: Option<&str>,
) -> Result<CrateInventory, InventoryError> {
    let full_path = root.join(rel).join("Cargo.toml");
    let content = fs::read_to_string(full_path)?;
    let parsed: toml::Value = toml::from_str(&content)?;

    let pkg = parsed
        .get("package")
        .ok_or_else(|| InventoryError::InvalidWorkspace("missing [package]".into()))?;

    Ok(parse_single_crate(
        root,
        rel,
        pkg,
        parsed.get("dependencies"),
        ws_version,
        ws_license,
    ))
}

fn parse_single_crate(
    root: &Path,
    rel: &Path,
    pkg: &toml::Value,
    deps_val: Option<&toml::Value>,
    ws_version: Option<&str>,
    ws_license: Option<&str>,
) -> CrateInventory {
    let name = pkg
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .or(ws_version)
        .unwrap_or("0.0.0")
        .to_string();

    let description = pkg
        .get("description")
        .and_then(|d| d.as_str())
        .map(String::from);
    let license = pkg
        .get("license")
        .and_then(|l| l.as_str())
        .or(ws_license)
        .map(String::from);

    let mut dependencies = HashMap::new();
    if let Some(deps) = deps_val.and_then(|d| d.as_table()) {
        for (dep_name, dep_info) in deps {
            let ver = if let Some(s) = dep_info.as_str() {
                s.to_string()
            } else if let Some(v) = dep_info.get("version").and_then(|v| v.as_str()) {
                v.to_string()
            } else {
                "workspace".to_string()
            };
            dependencies.insert(dep_name.clone(), ver);
        }
    }

    let has_binary = root.join(rel).join("src").join("main.rs").is_file();

    CrateInventory {
        name,
        version,
        relative_path: PathBuf::from(rel),
        description,
        license,
        dependencies,
        has_binary,
    }
}
