//! Toolchain configuration extractor (`rust-toolchain.toml`, `package.json`).

use std::fs;
use std::path::Path;

use crate::inventory::types::ToolchainInventory;

/// Extracts toolchain specifications from `rust-toolchain.toml` and `package.json`.
pub fn extract_toolchain(root: &Path, parsed_cargo: &toml::Value) -> ToolchainInventory {
    let mut toolchain = ToolchainInventory::default();

    let rust_toolchain_path = root.join("rust-toolchain.toml");
    if let Ok(content) = fs::read_to_string(&rust_toolchain_path)
        && let Ok(val) = toml::from_str::<toml::Value>(&content)
        && let Some(channel) = val
            .get("toolchain")
            .and_then(|t| t.get("channel"))
            .and_then(|c| c.as_str())
    {
        toolchain.rust_version = Some(channel.to_string());
    }

    if toolchain.rust_version.is_none()
        && let Some(rv) = parsed_cargo
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("rust-version"))
            .and_then(|v| v.as_str())
    {
        toolchain.rust_version = Some(rv.to_string());
    }

    let pkg_json = root.join("package.json");
    if let Ok(content) = fs::read_to_string(&pkg_json)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
    {
        if let Some(node) = val
            .get("engines")
            .and_then(|e| e.get("node"))
            .and_then(|n| n.as_str())
        {
            toolchain.node_version = Some(node.to_string());
        }
        if let Some(pm) = val.get("packageManager").and_then(|p| p.as_str()) {
            toolchain.package_manager = Some(pm.to_string());
        }
    }

    toolchain
}
