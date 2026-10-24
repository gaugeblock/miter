//! CLI commands and flags extractor from Rust source files.

use std::fs;
use std::path::Path;

use crate::inventory::types::{CliArgInventory, CliCommandInventory};

/// Extracts CLI commands and arguments from Rust source files in member crates.
pub fn extract_cli_inventory(
    root_dir: &Path,
    binary_name: &str,
    source_dirs: &[&Path],
) -> Vec<CliCommandInventory> {
    let mut commands = Vec::new();

    for dir in source_dirs {
        let full_dir = root_dir.join(dir);
        if full_dir.is_dir() {
            scan_dir_for_cli(&full_dir, binary_name, &mut commands);
        }
    }

    commands.sort_by(|a, b| a.subcommand_path.cmp(&b.subcommand_path));
    commands.dedup_by(|a, b| a.subcommand_path == b.subcommand_path);
    commands
}

fn scan_dir_for_cli(dir: &Path, binary_name: &str, commands: &mut Vec<CliCommandInventory>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_for_cli(&path, binary_name, commands);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
            && let Ok(content) = fs::read_to_string(&path)
        {
            parse_cli_file(&content, binary_name, commands);
        }
    }
}

fn parse_cli_file(content: &str, binary_name: &str, commands: &mut Vec<CliCommandInventory>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_subcommand_enum = false;
    let mut enum_depth: i32 = 0;
    let mut doc_lines: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            let doc = trimmed.trim_start_matches("///").trim();
            doc_lines.push(doc.to_string());
            continue;
        }

        if trimmed.contains("#[derive(") && trimmed.contains("Subcommand") {
            in_subcommand_enum = true;
            enum_depth = 0;
            doc_lines.clear();
            continue;
        }

        if in_subcommand_enum {
            let open_braces = trimmed.chars().filter(|c| *c == '{').count() as i32;
            let close_braces = trimmed.chars().filter(|c| *c == '}').count() as i32;

            if (enum_depth == 1 || (enum_depth == 0 && open_braces > 0)) && is_enum_variant(trimmed) {
                let variant_name = extract_variant_name(trimmed);
                let kebab_name = to_kebab_case(&variant_name);
                let description = if doc_lines.is_empty() {
                    None
                } else {
                    Some(doc_lines.join(" "))
                };

                let args = extract_variant_args(&lines, i);

                commands.push(CliCommandInventory {
                    binary_name: binary_name.to_string(),
                    subcommand_path: vec![kebab_name],
                    description,
                    args,
                });
                doc_lines.clear();
            }

            enum_depth += open_braces - close_braces;
            if enum_depth <= 0 && (open_braces > 0 || close_braces > 0) {
                in_subcommand_enum = false;
                doc_lines.clear();
            }
        } else if !trimmed.starts_with("#[") {
            doc_lines.clear();
        }
    }
}

fn is_enum_variant(line: &str) -> bool {
    let first = line.split(&[' ', '(', '{', ','][..]).next().unwrap_or("");
    !first.is_empty()
        && first
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        && !line.starts_with("pub enum")
        && !line.starts_with("enum")
}

fn extract_variant_name(line: &str) -> String {
    line.split(&[' ', '(', '{', ','][..])
        .next()
        .unwrap_or("")
        .to_string()
}

fn extract_variant_args(lines: &[&str], start_idx: usize) -> Vec<CliArgInventory> {
    let mut args = Vec::new();
    let mut idx = start_idx;

    // Scan until matching closing brace or parenthesis of variant
    let mut depth = 0;
    while idx < lines.len() {
        let line = lines[idx].trim();
        for ch in line.chars() {
            if ch == '{' || ch == '(' {
                depth += 1;
            } else if ch == '}' || ch == ')' {
                depth -= 1;
            }
        }

        if (line.contains("#[arg(") || line.contains("pub "))
            && let Some(arg) = parse_arg_field(line)
        {
            args.push(arg);
        }

        if depth <= 0 && idx > start_idx {
            break;
        }
        idx += 1;
        if idx > start_idx + 40 {
            break;
        }
    }

    args
}

fn parse_arg_field(line: &str) -> Option<CliArgInventory> {
    if !line.contains(':') && !line.contains("#[arg(") {
        return None;
    }

    let pub_part = line.split("pub ").nth(1)?;
    let field_name = pub_part.split(':').next()?.trim();

    let kebab = to_kebab_case(field_name);
    let is_flag = line.contains("bool");

    Some(CliArgInventory {
        long_name: Some(format!("--{kebab}")),
        short_flag: None,
        value_name: if is_flag {
            None
        } else {
            Some(field_name.to_uppercase())
        },
        help: None,
        required: !line.contains("Option<") && !is_flag,
    })
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            for lc in ch.to_lowercase() {
                result.push(lc);
            }
        } else if ch == '_' {
            result.push('-');
        } else {
            result.push(ch);
        }
    }
    result
}
