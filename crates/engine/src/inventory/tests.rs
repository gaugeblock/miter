//! Unit tests for workspace inventory extraction.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "miter_inv_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_cargo_extractor_single_crate() {
    let tmp = TempDir::new("cargo");
    let root = tmp.path();

    let cargo_toml = r#"
[package]
name = "sample-crate"
version = "0.2.1"
edition = "2024"
description = "Sample description"
license = "Apache-2.0"

[dependencies]
serde = "1.0"
"#;
    fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();

    let inv = inspect_workspace(root).unwrap();
    assert_eq!(inv.workspace_name.as_deref(), Some("sample-crate"));
    assert_eq!(inv.crates.len(), 1);
    assert_eq!(inv.crates[0].name, "sample-crate");
    assert_eq!(inv.crates[0].version, "0.2.1");
    assert_eq!(inv.crates[0].dependencies.get("serde").unwrap(), "1.0");
    assert_eq!(inv.toolchain.rust_edition.as_deref(), Some("2024"));
}

#[test]
fn test_cli_extractor_subcommands() {
    let tmp = TempDir::new("cli");
    let root = tmp.path();
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    let rust_code = r#"
#[derive(Subcommand)]
pub enum Commands {
    /// Check docs drift
    Check {
        /// Force flag
        #[arg(long)]
        force: bool,
    },
    /// Scaffold missing docs
    Scaffold {
        path: String,
    },
}
"#;
    fs::write(src.join("cli.rs"), rust_code).unwrap();

    let cmds = extract_cli_inventory(root, "mytool", &[Path::new("src")]);
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].subcommand_path, vec!["check"]);
    assert_eq!(cmds[0].description.as_deref(), Some("Check docs drift"));
    assert_eq!(cmds[1].subcommand_path, vec!["scaffold"]);
}

#[test]
fn test_public_types_extractor() {
    let tmp = TempDir::new("types");
    let root = tmp.path();
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    let rust_code = r#"
/// Configuration doc
pub struct Config {
    pub name: String,
}

/// Status enum doc
pub enum Status {
    Active,
    Inactive,
}

pub trait Runner {
    fn run(&self);
}
"#;
    fs::write(src.join("lib.rs"), rust_code).unwrap();

    let types = extract_public_types(root, &[Path::new("src")]);
    assert_eq!(types.len(), 3);
    assert!(types.iter().any(|t| t.type_name == "Config" && t.kind == "struct"));
    assert!(types.iter().any(|t| t.type_name == "Status" && t.kind == "enum"));
    assert!(types.iter().any(|t| t.type_name == "Runner" && t.kind == "trait"));
}
