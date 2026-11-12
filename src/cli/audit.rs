//! `miter audit`: Scans workspace source files for line-count threshold violations.

use super::error::CliError;
use super::json::Json;
use miter_engine::analysis::{AuditConfig, AuditSeverity, AuditViolation, WorkspaceAuditor};
use std::path::{Path, PathBuf};

/// Format choice for `miter audit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditFormat {
    /// Human-readable aligned table.
    Table,
    /// Raw JSON document.
    Json,
}

impl AuditFormat {
    /// Parses format string (`table` or `json`).
    #[must_use]
    pub fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("json") {
            Self::Json
        } else {
            Self::Table
        }
    }
}

/// Runs the `miter audit` command.
/// The dispatch entry, so the arm in `mod.rs` stays one line (`GB-195`).
///
/// `mod.rs` reached 255 lines when the ratchet arrived, which is the band's own argument: the file list that
/// grows fastest is the one everybody edits.
pub fn dispatch(
    path: Option<PathBuf>,
    threshold: Option<usize>,
    format: &str,
    check_paths: bool,
    write_size_ratchet: bool,
) -> Result<(), CliError> {
    run_audit(
        path,
        threshold,
        AuditFormat::parse(format),
        check_paths,
        write_size_ratchet,
    )
}

/// Runs the line audit and, with `check_paths`, the policy gates: layout adoption, the multi-repository test
/// requirement, and the 200-250 band ratchet.
///
/// # Errors
///
/// [`CliError::Io`] when the tree cannot be walked or the baseline cannot be written, and
/// [`CliError::Rejected`] when a gate fails.
pub fn run_audit(
    path: Option<PathBuf>,
    explicit_threshold: Option<usize>,
    format: AuditFormat,
    check_paths: bool,
    write_size_ratchet: bool,
) -> Result<(), CliError> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    let config = AuditConfig::from_workspace_with_threshold(&root, explicit_threshold);
    let threshold = config.max_lines;
    let auditor = WorkspaceAuditor::with_config(config);

    let violations = auditor
        .audit_root_with_members(&root)
        .map_err(CliError::Io)?;

    match format {
        AuditFormat::Json => {
            println!("{}", audit_json(&violations, threshold, &root).render());
        }
        AuditFormat::Table => {
            render_audit_table(&violations, threshold, &root);
        }
    }

    // The layout check runs alongside the line check (`GB-304`): `miter audit` is already in every
    // verification chain, so the rule is enforced without anyone remembering to run a second command -
    // which is exactly how five restated paths survived this long.
    if check_paths {
        super::audit_paths_report::run_paths_audit(&root, format)?;
        run_size_ratchet(&root, format, write_size_ratchet)?;
    }

    if violations.is_empty() {
        return Ok(());
    }
    Err(CliError::Rejected(format!(
        "{} file(s) exceed the {}-line threshold",
        violations.len(),
        threshold
    )))
}

fn audit_json(violations: &[AuditViolation], threshold: usize, root: &Path) -> Json {
    let items = violations.iter().map(|v| {
        Json::object([
            ("path", Json::string(v.path.display().to_string())),
            ("lines", Json::Unsigned(v.line_count as u64)),
            ("declarations", Json::Unsigned(v.declaration_count as u64)),
            ("severity", Json::string(v.severity.label())),
        ])
    });

    Json::object([
        ("root", Json::string(root.display().to_string())),
        ("threshold", Json::Unsigned(threshold as u64)),
        ("violation_count", Json::Unsigned(violations.len() as u64)),
        ("violations", Json::array(items)),
    ])
}

fn render_audit_table(violations: &[AuditViolation], threshold: usize, root: &Path) {
    if violations.is_empty() {
        println!(
            "AUDIT PASSED: No files exceed {} lines in {}",
            threshold,
            root.display()
        );
        return;
    }

    println!(
        "AUDIT: Found {} file(s) exceeding threshold ({} lines) in {}:\n",
        violations.len(),
        threshold,
        root.display()
    );

    println!(
        "{:<10} {:>7}  {:>13}  FILE PATH",
        "SEVERITY", "LINES", "DECLARATIONS"
    );
    println!("{:-<10} {:-<7}  {:-<13}  {:-<40}", "", "", "", "");

    for v in violations {
        let sev_tag = match v.severity {
            AuditSeverity::Critical => "! CRITICAL",
            AuditSeverity::Warning => "  WARNING ",
        };
        println!(
            "{:<10} {:>7}  {:>13}  {}",
            sev_tag,
            v.line_count,
            v.declaration_count,
            v.path.display()
        );
    }
}

/// The 200-250 band may not grow (`GB-195`).
///
/// Reuses [`AuditConfig`] at the band threshold rather than walking the tree again: two walkers would be two
/// answers to "which files count", and the suite spent three tickets removing restated logic.
fn run_size_ratchet(
    root: &Path,
    format: AuditFormat,
    write_baseline: bool,
) -> Result<(), CliError> {
    let current = super::audit_size_ratchet::band_files(root);
    if write_baseline {
        super::audit_size_ratchet::write(root, &current).map_err(CliError::Io)?;
        println!(
            "SIZE RATCHET RECORDED: {} file(s) in the {}-line band; growth from here is a failure",
            current.len(),
            super::audit_size_ratchet::BAND
        );
        return Ok(());
    }
    let recorded = super::audit_size_ratchet::load(root);
    if recorded.is_empty() {
        // Adoption, not a pass: say so, because a silent pass over an unrecorded band is how a metric
        // becomes decoration.
        println!(
            "SIZE RATCHET UNRECORDED: {} file(s) in the {}-line band and no baseline; \
             run `miter audit --paths --write-size-ratchet` to record them",
            current.len(),
            super::audit_size_ratchet::BAND
        );
        return Ok(());
    }
    let findings = super::audit_size_ratchet::findings(&recorded, &current);
    if findings.is_empty() {
        println!(
            "AUDIT PASSED: no file in the {}-line band grew ({} recorded)",
            super::audit_size_ratchet::BAND,
            recorded.len()
        );
        return Ok(());
    }
    if !matches!(format, AuditFormat::Json) {
        for finding in &findings {
            println!("  SIZE: {}", finding.describe());
        }
    }
    Err(CliError::Rejected(format!(
        "{} file(s) grew into or within the {}-line band; split them or record the growth deliberately",
        findings.len(),
        super::audit_size_ratchet::BAND
    )))
}
