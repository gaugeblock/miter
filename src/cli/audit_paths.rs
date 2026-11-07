//! `miter audit --paths`: a crate that names `.gaugeblock` must depend on `gaugeblock-workspace`.
//!
//! `GB-304` found five crates restating the `.gaugeblock/<tool>` layout instead of asking for it. None was
//! wrong at the time; each was a private copy waiting to drift. Repairing five is an afternoon. Preventing
//! the sixth is the point of this check, because the next crate written will reach for `root.join(".gaugeblock")`
//! unless something fails when it does.
//!
//! What counts as a use: a source line that mentions `.gaugeblock` outside a comment. Comments are excluded
//! deliberately - three of the eight original suspects were documentation, and a gate that fires on prose
//! teaches people to ignore it. String literals are *not* excluded, because a restated default path is
//! exactly the defect being caught.

use std::path::{Path, PathBuf};

/// Crates allowed to name the layout without depending on it.
///
/// Short and justified, in the same spirit as the layers allowlist: an entry that cannot be explained in a
/// sentence is one that should not exist.
const ALLOWED: [(&str, &str); 2] = [
    (
        "gaugeblock-workspace",
        "owns the layout; every other crate must ask it",
    ),
    (
        "gaugeblock-git",
        "owns the git process boundary and documents that it does not resolve .gaugeblock",
    ),
];

/// One crate that names the layout without adopting the resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathViolation {
    /// Package name from the manifest.
    pub crate_name: String,
    /// Manifest that should have carried the dependency.
    pub manifest: PathBuf,
    /// `(file, line)` for each non-comment mention.
    pub sites: Vec<(PathBuf, usize)>,
}

/// Directories that cannot contain a workspace crate.
const SKIP_DIRS: [&str; 6] = [
    "target",
    "node_modules",
    ".git",
    ".gaugeblock",
    "dist",
    "web",
];

/// Audits every crate under `root` for layout adoption.
///
/// # Errors
///
/// [`std::io::Error`] only when the walk itself fails; an unreadable manifest is skipped rather than
/// aborting, because one malformed crate must not hide the rest of the report.
pub fn audit_layout_adoption(root: &Path) -> std::io::Result<Vec<PathViolation>> {
    let mut violations = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if !SKIP_DIRS.iter().any(|skip| name == *skip) {
                    pending.push(path);
                }
                continue;
            }
            if path.file_name().is_some_and(|name| name == "Cargo.toml")
                && let Some(violation) = inspect_manifest(&path)
            {
                violations.push(violation);
            }
        }
    }
    violations.sort_by(|a, b| a.crate_name.cmp(&b.crate_name));
    Ok(violations)
}

/// Returns a violation when this crate's `src/` names the layout without the dependency.
fn inspect_manifest(manifest: &Path) -> Option<PathViolation> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let crate_name = package_name(&text)?;
    if ALLOWED.iter().any(|(allowed, _)| *allowed == crate_name) {
        return None;
    }
    if text.contains("gaugeblock-workspace") {
        return None;
    }
    let crate_dir = manifest.parent()?;
    let sites = mentions_in_src(&crate_dir.join("src"));
    if sites.is_empty() {
        return None;
    }
    Some(PathViolation {
        crate_name,
        manifest: manifest.to_path_buf(),
        sites,
    })
}

/// `name = "..."` from the `[package]` table.
fn package_name(manifest: &str) -> Option<String> {
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .find_map(|line| {
            let rest = line.trim().strip_prefix("name")?.trim().strip_prefix('=')?;
            Some(rest.trim().trim_matches('"').to_string())
        })
}

/// Non-comment mentions of the layout under `src`, recursively.
fn mentions_in_src(src: &Path) -> Vec<(PathBuf, usize)> {
    let mut sites = Vec::new();
    let mut pending = vec![src.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                for (index, line) in text.lines().enumerate() {
                    if mentions_layout(line) {
                        sites.push((path.clone(), index + 1));
                    }
                }
            }
        }
    }
    sites
}

/// Whether `line` names the layout and is not a comment.
fn mentions_layout(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
        return false;
    }
    // A trailing comment is stripped rather than the whole line skipped, because `foo(); // .gaugeblock`
    // mentions the layout only in prose.
    let code = match line.find("//") {
        Some(index) => &line[..index],
        None => line,
    };
    code.contains(".gaugeblock")
}


/// Runs the check and reports it in the caller's format.
///
/// # Errors
///
/// [`CliError::Rejected`] when a crate names the layout without the dependency, [`CliError::Io`] when the
/// walk itself fails.
pub fn run_paths_audit(root: &Path, format: super::audit::AuditFormat) -> Result<(), super::error::CliError> {
    let violations =
        audit_layout_adoption(root).map_err(super::error::CliError::Io)?;

    match format {
        super::audit::AuditFormat::Json => println!("{}", json(&violations).render()),
        super::audit::AuditFormat::Table => {
            if violations.is_empty() {
                println!("AUDIT PASSED: every crate naming .gaugeblock depends on gaugeblock-workspace");
            } else {
                println!("Crate names .gaugeblock without gaugeblock-workspace:");
                println!("{:<28} {:<10} {}", "CRATE", "SITES", "MANIFEST");
                println!("{}", "-".repeat(78));
                for violation in &violations {
                    println!(
                        "{:<28} {:<10} {}",
                        violation.crate_name,
                        violation.sites.len(),
                        violation.manifest.display()
                    );
                    for (file, line) in violation.sites.iter().take(3) {
                        println!("    {}:{line}", file.display());
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        return Ok(());
    }
    Err(super::error::CliError::Rejected(format!(
        "{} crate(s) name .gaugeblock without depending on gaugeblock-workspace",
        violations.len()
    )))
}

/// The same report as JSON.
fn json(violations: &[PathViolation]) -> super::json::Json {
    super::json::Json::object([(
        "violations",
        super::json::Json::array(violations.iter().map(|violation| {
            super::json::Json::object([
                ("crate", super::json::Json::string(&violation.crate_name)),
                (
                    "manifest",
                    super::json::Json::string(violation.manifest.display().to_string()),
                ),
                (
                    "sites",
                    super::json::Json::Unsigned(violation.sites.len() as u64),
                ),
            ])
        })),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_mention_is_not_a_use() {
        assert!(!mentions_layout("/// Reads `.gaugeblock/config.toml`."));
        assert!(!mentions_layout("// keep it in .gaugeblock"));
        assert!(!mentions_layout("let _ = 1; // under .gaugeblock"));
    }

    #[test]
    fn a_restated_default_path_is_a_use() {
        assert!(mentions_layout(r#"#[arg(long, default_value = ".gaugeblock/spool/outbox.jsonl")]"#));
        assert!(mentions_layout(r#"let dir = root.join(".gaugeblock");"#));
    }

    #[test]
    fn the_package_name_is_read_from_the_package_table() {
        let manifest = "[workspace]\nmembers = []\n\n[package]\nname = \"dado-core\"\nversion = \"1\"\n";
        assert_eq!(package_name(manifest), Some("dado-core".to_string()));
    }
}
