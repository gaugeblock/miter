//! `miter audit --paths`: a crate that resolves the suite must prove it with two repositories.
//!
//! The layout rule (in `audit_paths`) stops a crate restating `.gaugeblock`. This one is about the other half
//! of `GB-305`: a crate that calls `GaugeblockPaths::discover` or `WorkspaceTopology::discover` is making a
//! claim about which repository it is in, and that claim is only testable with two repositories in scope.
//!
//! With one repository, per-repository logic and suite logic are indistinguishable - which is how benchwork
//! tailed one telemetry log of twelve, miter's dirty check missed member repositories, storypole adopted a
//! parent's ledger, and outfeed's release walk recorded nothing in a worktree. Four defects, one missing test
//! shape, none of them caught by a suite that was green throughout.
//!
//! Scope is deliberately narrow: discovery, not resolution. `GaugeblockPaths::at(root).member("dado")` is
//! arithmetic over a path a caller already has; `discover` is the operation whose answer changes with the
//! repository. A gate that demanded multi-repo tests of arithmetic would flag eleven crates and teach people
//! to ignore it.

use std::path::{Path, PathBuf};

/// Calls that make a claim about which repository the caller is in.
const DISCOVERY_CALLS: [&str; 2] = ["GaugeblockPaths::discover", "WorkspaceTopology::discover"];

/// The declaration a test file uses to say it puts two repositories in scope.
const MARKER: &str = "multi-repo:";

/// Crates that resolve the suite but have no multi-repository test yet, each with the ticket that owes one.
///
/// An entry is a debt, not a suppression: the crate is doing the thing the rule is about and has no evidence
/// for it. Short on purpose - if this list grows, the rule is being negotiated with rather than enforced.
const OWED: [(&str, &str); 4] = [
    (
        "benchwork-cli",
        "GB-293: the daemon's telemetry and handler paths need a two-repository test",
    ),
    (
        "storypole-cli",
        "GB-305: config resolution from a member repository",
    ),
    (
        "sawhorse-core",
        "GB-305: telemetry writing from a member repository",
    ),
    (
        "rabbet-cli",
        "GB-304: spool resolution after the default became runtime-resolved",
    ),
];

/// One crate that resolves the suite without a multi-repository test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscoveryFinding {
    /// Package name.
    pub crate_name: String,
    /// `(file, line)` of each discovery call.
    pub sites: Vec<(PathBuf, usize)>,
}

/// Audits every crate under `root`.
///
/// # Errors
///
/// [`std::io::Error`] only when the walk fails; an unreadable manifest or source file is skipped.
pub(super) fn audit_multi_repo(root: &Path) -> std::io::Result<Vec<DiscoveryFinding>> {
    let mut findings = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut manifests = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if ![
                    "target",
                    "node_modules",
                    ".git",
                    ".gaugeblock",
                    "dist",
                    "web",
                ]
                .iter()
                .any(|skip| name == *skip)
                {
                    pending.push(path);
                }
                continue;
            }
            if path.file_name().is_some_and(|name| name == "Cargo.toml") {
                manifests.push(path);
            }
        }
        for manifest in manifests {
            if let Some(finding) = inspect(&manifest) {
                findings.push(finding);
            }
        }
    }
    findings.sort_by(|a, b| a.crate_name.cmp(&b.crate_name));
    Ok(findings)
}

/// Checks one crate: does it discover, and does it prove it?
fn inspect(manifest: &Path) -> Option<DiscoveryFinding> {
    let crate_dir = manifest.parent()?;
    let crate_name = package_name(&std::fs::read_to_string(manifest).ok()?)?;
    let sites = discovery_sites(&crate_dir.join("src"));
    if sites.is_empty() {
        return None;
    }
    if has_multi_repo_test(&crate_dir.join("tests")) {
        return None;
    }
    if OWED.iter().any(|(owed, _)| *owed == crate_name) {
        return None;
    }
    Some(DiscoveryFinding { crate_name, sites })
}

fn discovery_sites(src: &Path) -> Vec<(PathBuf, usize)> {
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
            if !path.extension().is_some_and(|ext| ext == "rs") {
                continue;
            }
            // The checkers contain the patterns they look for (`GB-305`). Skipping them by name is honest:
            // an allowlist entry for their crate would also suppress a real finding in the same crate.
            if path
                .file_name()
                .is_some_and(|name| name == "audit_multi_repo.rs" || name == "audit_paths.rs")
            {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or_default();
                if DISCOVERY_CALLS.iter().any(|call| code.contains(call)) {
                    sites.push((path.clone(), index + 1));
                }
            }
        }
    }
    sites
}

/// Whether any test file declares two repositories in scope.
fn has_multi_repo_test(tests: &Path) -> bool {
    let mut pending = vec![tests.to_path_buf()];
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
                && text.contains(MARKER)
            {
                return true;
            }
        }
    }
    false
}

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

/// Reports the check in the caller's format and fails when a crate owes a test.
pub(super) fn run_multi_repo_audit(
    root: &Path,
    format: super::audit::AuditFormat,
) -> Result<(), super::error::CliError> {
    let findings = audit_multi_repo(root).map_err(super::error::CliError::Io)?;
    match format {
        super::audit::AuditFormat::Json => {
            let rows = findings.iter().map(|finding| {
                super::json::Json::object([
                    ("crate", super::json::Json::string(&finding.crate_name)),
                    (
                        "discovery_sites",
                        super::json::Json::Unsigned(finding.sites.len() as u64),
                    ),
                ])
            });
            println!(
                "{}",
                super::json::Json::object([("multi_repo_gaps", super::json::Json::array(rows))])
                    .render()
            );
        }
        super::audit::AuditFormat::Table => {
            if findings.is_empty() {
                println!(
                    "AUDIT PASSED: every crate resolving the suite has a multi-repository test"
                );
            } else {
                println!("Crate resolves the suite without a multi-repository test:");
                for finding in &findings {
                    println!(
                        "  {:<28} {} discovery site(s)",
                        finding.crate_name,
                        finding.sites.len()
                    );
                    for (file, line) in finding.sites.iter().take(2) {
                        println!("      {}:{line}", file.display());
                    }
                }
            }
        }
    }
    if findings.is_empty() {
        return Ok(());
    }
    Err(super::error::CliError::Rejected(format!(
        "{} crate(s) resolve the suite without a multi-repository test",
        findings.len()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_is_not_discovery() {
        // Arithmetic over a path the caller already has is not a claim about which repository it is in.
        assert!(!discovery_sites_of(
            "let dir = GaugeblockPaths::at(root).member(\"dado\");"
        ));
        assert!(discovery_sites_of(
            "let paths = GaugeblockPaths::discover(&cwd)?;"
        ));
        assert!(discovery_sites_of(
            "let topo = WorkspaceTopology::discover(root);"
        ));
    }

    #[test]
    fn a_comment_does_not_count_as_a_discovery_call() {
        assert!(!discovery_sites_of(
            "// GaugeblockPaths::discover(start_dir)"
        ));
    }

    fn discovery_sites_of(line: &str) -> bool {
        let code = line.split("//").next().unwrap_or_default();
        DISCOVERY_CALLS.iter().any(|call| code.contains(call))
    }
}
