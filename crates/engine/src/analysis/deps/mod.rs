//! Crate dependency graph (GB-161 tier 1): declared dependencies versus imported ones.
//!
//! Three questions that a `Cargo.toml` cannot answer on its own: which workspace crates import
//! which other workspace crates, which declared dependencies no source file imports, and where the
//! proof of use lives for a given crate. Tier 2 (extraction simulation, layering fitness, budgets)
//! and tier 3 (feature inventory, per-feature cost) build on this report rather than re-parsing.

mod allowlist;
mod imports;
mod layers;
mod strip;

#[cfg(test)]
mod layers_tests;

#[cfg(test)]
mod tests;

pub use allowlist::{AllowlistEntry, LayerAllowlist, load_allowlist};
pub use imports::{ImportSite, collect_crate_imports, declared_modules, import_roots_in_source};
pub use layers::{
    EdgeKind, Layer, LayerEdge, LayerReport, assess_edge, build_layer_report, classify_crate,
    classify_edge, family,
};

use crate::inventory::{InventoryError, extract_cargo_inventory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Crate names use hyphens; import roots use underscores.
#[must_use]
pub fn crate_ident(name: &str) -> String {
    name.replace('-', "_")
}

/// One workspace crate importing another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateEdge {
    /// Importing crate (package name).
    pub from: String,
    /// Imported crate (package name).
    pub to: String,
    /// Every import site that proves the edge.
    pub sites: Vec<ImportSite>,
}

/// A dependency declared in `Cargo.toml` that no source file of the crate imports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnusedDependency {
    /// The crate carrying the declaration.
    pub crate_name: String,
    /// The declared dependency name.
    pub dependency: String,
    /// Its declared version requirement.
    pub declared_version: String,
}

/// Result of `miter deps` tier 1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateDependencyReport {
    /// Crates evaluated.
    pub crates_analyzed: usize,
    /// Crate-to-crate edges, sorted by importer then dependency.
    pub edges: Vec<CrateEdge>,
    /// Declared dependencies nothing imports.
    pub unused: Vec<UnusedDependency>,
}

/// A crate's declared view, kept separate from the manifest reader so assembly is testable
/// without touching a filesystem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredCrate {
    /// Package name.
    pub name: String,
    /// Declared dependency name -> version requirement.
    pub dependencies: BTreeMap<String, String>,
}

/// Failures from dependency analysis.
#[derive(Debug, thiserror::Error)]
pub enum DepsError {
    /// The workspace manifests could not be read.
    #[error("workspace manifest inventory failed: {0}")]
    Inventory(#[from] InventoryError),

    /// The layer allowlist exists but cannot be read or parsed.
    #[error("layer allowlist is unusable: {0}")]
    Allowlist(String),
}

/// Builds the report from declared crates and the imports collected per crate.
#[must_use]
pub fn assemble_report(
    crates: &[DeclaredCrate],
    imports: &BTreeMap<String, BTreeMap<String, Vec<ImportSite>>>,
) -> CrateDependencyReport {
    let mut known: BTreeMap<String, &str> = BTreeMap::new();
    for krate in crates {
        known.insert(crate_ident(&krate.name), krate.name.as_str());
    }

    let mut edges = Vec::new();
    let mut unused = Vec::new();
    for krate in crates {
        let empty = BTreeMap::new();
        let crate_imports = imports.get(&krate.name).unwrap_or(&empty);

        for (root, sites) in crate_imports {
            if let Some(target) = known.get(root)
                && *target != krate.name.as_str()
            {
                edges.push(CrateEdge {
                    from: krate.name.clone(),
                    to: (*target).to_string(),
                    sites: sites.clone(),
                });
            }
        }

        for (dependency, version) in &krate.dependencies {
            if !crate_imports.contains_key(&crate_ident(dependency)) {
                unused.push(UnusedDependency {
                    crate_name: krate.name.clone(),
                    dependency: dependency.clone(),
                    declared_version: version.clone(),
                });
            }
        }
    }

    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    unused.sort_by(|a, b| {
        a.crate_name
            .cmp(&b.crate_name)
            .then(a.dependency.cmp(&b.dependency))
    });

    CrateDependencyReport {
        crates_analyzed: crates.len(),
        edges,
        unused,
    }
}

/// Analyses every crate in the workspace rooted at `root`.
///
/// # Errors
///
/// [`DepsError::Inventory`] when the workspace manifests cannot be read. Unreadable source files
/// are skipped, so one broken path does not erase the report.
pub fn analyze_dependencies(root: &Path) -> Result<CrateDependencyReport, DepsError> {
    let cargo = extract_cargo_inventory(root)?;

    let declared: Vec<DeclaredCrate> = cargo
        .crates
        .iter()
        .map(|krate| DeclaredCrate {
            name: krate.name.clone(),
            dependencies: krate
                .dependencies
                .iter()
                .map(|(name, version)| (name.clone(), version.clone()))
                .collect(),
        })
        .collect();

    let mut imports = BTreeMap::new();
    for krate in &cargo.crates {
        imports.insert(
            krate.name.clone(),
            collect_crate_imports(root, &krate.relative_path),
        );
    }

    Ok(assemble_report(&declared, &imports))
}
