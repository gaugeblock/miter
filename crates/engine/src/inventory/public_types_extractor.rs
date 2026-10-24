//! Public interface and type definition extractor.

use std::fs;
use std::path::Path;

use crate::inventory::types::PublicTypeInventory;

/// Extracts public structs, enums, and traits from Rust source files.
pub fn extract_public_types(root_dir: &Path, source_dirs: &[&Path]) -> Vec<PublicTypeInventory> {
    let mut types = Vec::new();

    for dir in source_dirs {
        let full_dir = root_dir.join(dir);
        if full_dir.is_dir() {
            scan_dir_for_types(&full_dir, &mut types);
        }
    }

    types.sort_by(|a, b| a.type_name.cmp(&b.type_name));
    types.dedup_by(|a, b| a.type_name == b.type_name && a.module_path == b.module_path);
    types
}

fn scan_dir_for_types(dir: &Path, types: &mut Vec<PublicTypeInventory>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_for_types(&path, types);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
            && let Ok(content) = fs::read_to_string(&path)
        {
            parse_types_file(&path, &content, types);
        }
    }
}

fn parse_types_file(path: &Path, content: &str, types: &mut Vec<PublicTypeInventory>) {
    let module_path = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let lines: Vec<&str> = content.lines().collect();
    let mut doc_lines: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            let doc = trimmed.trim_start_matches("///").trim();
            doc_lines.push(doc.to_string());
            continue;
        }

        if trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub trait ")
        {
            let (kind, rest) = if let Some(r) = trimmed.strip_prefix("pub struct ") {
                ("struct", r)
            } else if let Some(r) = trimmed.strip_prefix("pub enum ") {
                ("enum", r)
            } else if let Some(r) = trimmed.strip_prefix("pub trait ") {
                ("trait", r)
            } else {
                continue;
            };

            let type_name = rest
                .split(&['<', '{', '(', ';', ' '][..])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if !type_name.is_empty() {
                let doc_summary = if doc_lines.is_empty() {
                    None
                } else {
                    Some(doc_lines.join(" "))
                };

                types.push(PublicTypeInventory {
                    module_path: module_path.clone(),
                    type_name,
                    kind: kind.to_string(),
                    doc_summary,
                });
            }
            doc_lines.clear();
        } else if !trimmed.starts_with("#[") {
            doc_lines.clear();
        }
    }
}
