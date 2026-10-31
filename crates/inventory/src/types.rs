//! Data structures representing workspace inventories for architectural documentation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Complete inventory of a repository or workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInventory {
    /// Absolute or canonical workspace root directory.
    pub root_dir: PathBuf,
    /// Workspace-level package metadata if defined at root.
    pub workspace_name: Option<String>,
    /// Discovered member crates or packages.
    pub crates: Vec<CrateInventory>,
    /// Discovered CLI command definitions and flags.
    pub cli_commands: Vec<CliCommandInventory>,
    /// Discovered public interfaces, types, and structs.
    pub public_types: Vec<PublicTypeInventory>,
    /// Git metadata (branch, latest tag, commit count, unreleased commits).
    pub git: Option<GitInventory>,
    /// Detected toolchain configurations (`rust-toolchain.toml`, `package.json`, etc.).
    pub toolchain: ToolchainInventory,
}

/// Metadata for an individual crate or package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateInventory {
    /// Name of the package (e.g. `miter-engine`).
    pub name: String,
    /// Package version (e.g. `0.1.0-alpha.2`).
    pub version: String,
    /// Path relative to workspace root.
    pub relative_path: PathBuf,
    /// Short summary or description.
    pub description: Option<String>,
    /// Declared license (e.g. `Apache-2.0`).
    pub license: Option<String>,
    /// Crate dependencies and required version constraints.
    pub dependencies: HashMap<String, String>,
    /// Whether this crate defines a binary entrypoint (`src/main.rs`).
    pub has_binary: bool,
}

/// Discovered CLI commands and arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliCommandInventory {
    /// Top-level binary name or parent command (e.g. `chalkline`).
    pub binary_name: String,
    /// Subcommand path (e.g. `["docs", "check"]`).
    pub subcommand_path: Vec<String>,
    /// Brief description from doc comments or help text.
    pub description: Option<String>,
    /// Discovered flags, options, and positional arguments.
    pub args: Vec<CliArgInventory>,
}

/// Discovered CLI argument or flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliArgInventory {
    /// Long name (e.g. `--output` or `--check`).
    pub long_name: Option<String>,
    /// Short flag (e.g. `-o` or `-c`).
    pub short_flag: Option<char>,
    /// Value name if taking arguments (e.g. `PATH`).
    pub value_name: Option<String>,
    /// Documentation/help text.
    pub help: Option<String>,
    /// Whether the argument is required.
    pub required: bool,
}

/// Publicly exposed interface, struct, or enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicTypeInventory {
    /// Crate or module path (e.g. `miter_engine::inventory`).
    pub module_path: String,
    /// Type identifier name (e.g. `WorkspaceInventory`).
    pub type_name: String,
    /// Kind of type: `struct`, `enum`, `trait`, `interface`.
    pub kind: String,
    /// Public docstring summary.
    pub doc_summary: Option<String>,
}

/// Git metadata for change auditing and changelog generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitInventory {
    /// Active branch name.
    pub current_branch: String,
    /// Latest HEAD commit hash (abbreviated or full).
    pub head_commit: String,
    /// Most recent release tag if any.
    pub latest_tag: Option<String>,
    /// Commits since the latest tag (`<last_tag>..HEAD`).
    pub unreleased_commits: Vec<String>,
}

/// Detected toolchain configurations across the repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolchainInventory {
    /// Rust channel/version (from `rust-toolchain.toml` or `Cargo.toml`).
    pub rust_version: Option<String>,
    /// Rust edition (e.g. `2024`).
    pub rust_edition: Option<String>,
    /// Node version constraint (from `package.json` `engines.node`).
    pub node_version: Option<String>,
    /// Package manager (e.g. `pnpm@9.x`).
    pub package_manager: Option<String>,
}
