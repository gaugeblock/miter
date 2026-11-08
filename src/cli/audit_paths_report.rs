//! Reporting for the two layout checks. Split from detection so neither file chases the 250-line limit
//! through formatting changes - `rustfmt` decides how many lines an expression takes, and a file that sits at
//! the boundary gets pushed over by a reformat rather than by a change.

use super::audit_multi_repo::audit_multi_repo;
use super::audit_paths::{PathViolation, audit_layout_adoption};
use std::path::Path;

/// Runs the check and reports it in the caller's format.
///
/// # Errors
///
/// [`CliError::Rejected`] when a crate names the layout without the dependency, [`CliError::Io`] when the
/// walk itself fails.
pub(super) fn run_paths_audit(
    root: &Path,
    format: super::audit::AuditFormat,
) -> Result<(), super::error::CliError> {
    let violations = audit_layout_adoption(root).map_err(super::error::CliError::Io)?;

    match format {
        super::audit::AuditFormat::Json => println!("{}", json(&violations).render()),
        super::audit::AuditFormat::Table => {
            if violations.is_empty() {
                println!(
                    "AUDIT PASSED: every crate naming .gaugeblock depends on gaugeblock-workspace"
                );
            } else {
                println!("Crate names .gaugeblock without gaugeblock-workspace:");
                println!("{:<28} {:<10} MANIFEST", "CRATE", "SITES");
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

    run_multi_repo_audit(root, format)?; // GB-305, the rule's other half

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
    let rows = violations.iter().map(|v| {
        super::json::Json::object([
            ("crate", super::json::Json::string(&v.crate_name)),
            (
                "manifest",
                super::json::Json::string(v.manifest.display().to_string()),
            ),
            ("sites", super::json::Json::Unsigned(v.sites.len() as u64)),
        ])
    });
    super::json::Json::object([("violations", super::json::Json::array(rows))])
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
