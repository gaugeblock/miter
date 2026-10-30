//! Import-root collection: which identifiers a crate's source actually references.

use super::strip::{StripState, code_only};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// One place a crate's source imports another crate's root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSite {
    /// File path relative to the workspace root.
    pub file: String,
    /// 1-based line number of the statement.
    pub line: usize,
}

/// Path roots that name something other than a workspace dependency.
const NOT_A_DEPENDENCY: [&str; 8] = [
    "crate",
    "self",
    "super",
    "std",
    "core",
    "alloc",
    "proc_macro",
    "test",
];

/// Extracts `(root, line)` for every reference to a crate root in `source`.
///
/// Two shapes count, because a dependency can be used without ever being imported: a `use` /
/// `extern crate` statement (`use storypole_core::model::TaskNode`) and a fully-qualified path
/// prefix (`serde_json::to_string(..)`, `tracing::warn!(..)`). Only the first segment is returned,
/// because that is what names a crate. Paths that name a language root rather than a crate (`std`,
/// `core`, `crate`, `super`, `self`) are dropped.
#[must_use]
pub fn import_roots_in_source(source: &str) -> Vec<(String, usize)> {
    let mut roots = Vec::new();
    let mut state = StripState::default();
    for (index, raw) in source.lines().enumerate() {
        // Literals and comments are data, not references (GB-199).
        let stripped = code_only(raw, &mut state);
        let line = stripped.trim_start();
        if line.is_empty() {
            continue;
        }
        let mut found: Vec<&str> = Vec::new();

        if let Some(body) = use_body(line) {
            // A `use` statement names its crate in the first segment; every later segment is a
            // module, type or item, so the path scan must not run on this line.
            let root = body
                .trim_start_matches("::")
                .split("::")
                .next()
                .unwrap_or_default()
                .trim()
                .trim_end_matches(';')
                .trim();
            if is_ident(root) && !NOT_A_DEPENDENCY.contains(&root) {
                found.push(root);
            }
        } else {
            for root in path_roots_in_line(line) {
                if !found.contains(&root) {
                    found.push(root);
                }
            }
        }

        for root in found {
            roots.push((root.to_string(), index + 1));
        }
    }
    roots
}

/// Collects every `<ident>::` prefix on a line, which is how a dependency is named when it is used
/// fully qualified instead of imported.
fn path_roots_in_line(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut roots = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let ident = &line[start..index];
            // Crate roots are lowercase snake_case; an upper-case ident before `::` is a type path
            // (`StorypoleStore::now_ms`), which proves use of nothing.
            let looks_like_crate = ident.starts_with(|c: char| c.is_ascii_lowercase() || c == '_');
            if looks_like_crate
                && line[index..].starts_with("::")
                && !NOT_A_DEPENDENCY.contains(&ident)
            {
                roots.push(ident);
            }
        } else {
            index += 1;
        }
    }
    roots
}

/// Returns the part of a statement that follows the `use` / `extern crate` keyword.
fn use_body(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("pub use ") {
        return Some(rest);
    }
    if let Some(rest) = line.strip_prefix("use ") {
        return Some(rest);
    }
    if let Some(rest) = line.strip_prefix("extern crate ") {
        return Some(rest);
    }
    if line.starts_with("pub(") {
        return line.split_once("use ").map(|(_, rest)| rest);
    }
    None
}

/// True when `value` is a single Rust identifier segment.
fn is_ident(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !value.starts_with(|c: char| c.is_ascii_digit())
}

/// Module names the crate declares itself, which shadow an identically named crate.
///
/// `gaugeblock-contracts` has an `outfeed` module and re-exports it with `pub use outfeed::*;`.
/// To a path parser that looks like a dependency on the `outfeed` crate, so the crate's own module
/// names must be subtracted before any root is treated as external.
#[must_use]
pub fn declared_modules(source: &str) -> Vec<String> {
    let mut modules = Vec::new();
    let mut state = StripState::default();
    for raw in source.lines() {
        let stripped = code_only(raw, &mut state);
        let line = stripped.trim_start();
        if line.is_empty() {
            continue;
        }
        let body = line
            .strip_prefix("pub(crate) mod ")
            .or_else(|| line.strip_prefix("pub mod "))
            .or_else(|| line.strip_prefix("mod "));
        let Some(body) = body else {
            continue;
        };
        let name = body
            .trim()
            .trim_end_matches(';')
            .split(['<', '{', ' '])
            .next()
            .unwrap_or_default()
            .trim();
        if is_ident(name) {
            modules.push(name.to_string());
        }
    }
    modules
}

/// Walks a crate's source tree and maps import root -> sites.
///
/// Roots that name one of the crate's own modules are dropped, and unreadable directories or files
/// are skipped rather than failing the run: the report is an inventory, and a partial answer beats
/// no answer when one vendored path is unreadable.
#[must_use]
pub fn collect_crate_imports(
    root: &Path,
    crate_rel: &Path,
    declared_dependencies: &BTreeSet<String>,
) -> BTreeMap<String, Vec<ImportSite>> {
    let mut files: Vec<(String, String)> = Vec::new();
    let mut queue = vec![root.join(crate_rel)];

    while let Some(current) = queue.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name.starts_with('.') || matches!(name, "target" | "node_modules" | "dist" | "build")
            {
                continue;
            }
            if path.is_dir() {
                queue.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            files.push((rel, content));
        }
    }

    let mut own_modules: BTreeSet<String> = BTreeSet::new();
    for (_, content) in &files {
        own_modules.extend(declared_modules(content));
    }

    let mut imports: BTreeMap<String, Vec<ImportSite>> = BTreeMap::new();
    for (rel, content) in &files {
        for (root_ident, line) in import_roots_in_source(content) {
            if super::resolution::masks_a_dependency(
                &root_ident,
                &own_modules,
                declared_dependencies,
            ) {
                continue;
            }
            imports.entry(root_ident).or_default().push(ImportSite {
                file: rel.clone(),
                line,
            });
        }
    }

    imports
}
