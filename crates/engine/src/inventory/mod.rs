//! Workspace inventory extraction for AST-grounded architectural documentation.
//!
//! Provides deterministic extraction of:
//! - Cargo workspace members, versions, dependencies
//! - Clap CLI commands, subcommands, and arguments
//! - Public types, interfaces, structs, enums
//! - Git commits, branches, and release tags
//! - Toolchain definitions (`rust-toolchain.toml`, `package.json`)

pub mod cargo_extractor;
pub mod cargo_parser;
pub mod clap_extractor;
pub mod error;
pub mod git_extractor;
pub mod public_types_extractor;
pub mod toolchain_extractor;
pub mod types;

#[cfg(test)]
mod tests;

use std::path::Path;

pub use cargo_extractor::extract_cargo_inventory;
pub use clap_extractor::extract_cli_inventory;
pub use error::InventoryError;
pub use git_extractor::extract_git_inventory;
pub use public_types_extractor::extract_public_types;
pub use types::{
    CliArgInventory, CliCommandInventory, CrateInventory, GitInventory, PublicTypeInventory,
    ToolchainInventory, WorkspaceInventory,
};

/// Inspects a workspace root directory and returns a comprehensive `WorkspaceInventory`.
pub fn inspect_workspace(root: &Path) -> Result<WorkspaceInventory, InventoryError> {
    let cargo = extract_cargo_inventory(root)?;
    let git = extract_git_inventory(root);

    let mut source_dirs = Vec::new();
    let root_src = Path::new("src");
    if root.join(root_src).is_dir() {
        source_dirs.push(root_src);
    }

    for cr in &cargo.crates {
        let crate_src = cr.relative_path.join("src");
        if root.join(&crate_src).is_dir() {
            source_dirs.push(&cr.relative_path);
        }
    }

    let bin_name = cargo
        .workspace_name
        .clone()
        .unwrap_or_else(|| "app".to_string());
    let cli_commands = extract_cli_inventory(root, &bin_name, &source_dirs);
    let public_types = extract_public_types(root, &source_dirs);

    Ok(WorkspaceInventory {
        root_dir: root.to_path_buf(),
        workspace_name: cargo.workspace_name,
        crates: cargo.crates,
        cli_commands,
        public_types,
        git,
        toolchain: cargo.toolchain,
    })
}
