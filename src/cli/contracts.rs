//! `miter contracts`: Cross-language polyglot schema contract verification.

use super::error::CliError;
use super::json::Json;
use miter_engine::{ContractReport, StorageError, WorkspaceStore};
use std::path::Path;

/// Format choice for `miter contracts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractsFormat {
    /// Human-readable aligned summary table.
    Table,
    /// Machine-readable JSON output.
    Json,
}

impl ContractsFormat {
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

/// Runs the `miter contracts` command.
///
/// `check` turns the report into a verdict. Without it the command describes what it found and succeeds,
/// which is what a person inspecting drift wants; with it, drift is a failure, because a gate that prints
/// thirty-two drift lines and exits 0 is not a gate (`GB-223`).
pub fn run_contracts(
    db: &Path,
    format: ContractsFormat,
    check: bool,
    allowlist_path: &Path,
) -> Result<(), CliError> {
    if !db.exists() {
        return Err(CliError::MissingIndex(db.to_path_buf()));
    }

    let store = WorkspaceStore::open(db)
        .map_err(|e: StorageError| CliError::Usage(format!("failed to open database: {e}")))?;
    let symbols = miter_engine::load_redb_universal_from(&store)
        .map_err(|e| CliError::Usage(format!("failed to load universal symbols: {e}")))?;
    let allow = miter_engine::load_contract_allowlist(allowlist_path).map_err(CliError::Usage)?;
    let report = miter_engine::verify_contracts_with_allowlist(&symbols, &allow);

    match format {
        ContractsFormat::Table => render_table(&report),
        ContractsFormat::Json => render_json(&report),
    }

    verdict(&report, check)
}

/// Whether a report should fail the command.
///
/// Split out because the interesting part is a decision, not printing: the report can be rendered in two
/// formats and the verdict must not depend on which one was chosen.
fn verdict(report: &ContractReport, check: bool) -> Result<(), CliError> {
    if !check || report.drifts.is_empty() {
        return Ok(());
    }
    let fields: usize = report
        .drifts
        .iter()
        .map(|drift| drift.missing_fields.len())
        .sum();
    Err(CliError::Rejected(format!(
        "contract drift: {} entit{} across {fields} field(s)",
        report.drifts.len(),
        if report.drifts.len() == 1 { "y" } else { "ies" }
    )))
}

fn render_table(report: &ContractReport) {
    println!("Polyglot Contract Verification across SQL, Rust, and TypeScript");
    println!("{}", "=".repeat(80));
    println!(
        "Verified Multi-Language Entities: {}",
        report.verified_entities
    );
    println!("Entities with Contract Drift:     {}", report.drifts.len());
    if report.suppressed > 0 {
        println!("Declared non-mirrors (allowed):   {}", report.suppressed);
    }

    if report.drifts.is_empty() {
        println!("\nALL CONTRACTS CONSISTENT: No cross-language schema drifts detected.");
        return;
    }

    println!("\n{:<24} {:<24} {:<30}", "ENTITY", "FIELD", "DRIFT STATUS");
    println!("{}", "-".repeat(80));

    for drift in &report.drifts {
        for missing in &drift.missing_fields {
            let status = format!(
                "in [{}] but missing in [{}]",
                missing.present_in.join(", "),
                missing.missing_in.join(", ")
            );
            println!(
                "{:<24} {:<24} {:<30}",
                drift.entity, missing.field_name, status
            );
        }
    }
}

fn render_json(report: &ContractReport) {
    let drifts_json = report.drifts.iter().map(|d| {
        let missing_json = d.missing_fields.iter().map(|m| {
            let pres = m.present_in.iter().map(|s| Json::string(s.clone()));
            let miss = m.missing_in.iter().map(|s| Json::string(s.clone()));
            Json::object([
                ("field", Json::string(&m.field_name)),
                ("present_in", Json::array(pres)),
                ("missing_in", Json::array(miss)),
            ])
        });
        let srcs = d.sources.iter().map(|s| Json::string(s.clone()));
        Json::object([
            ("entity", Json::string(&d.entity)),
            ("sources", Json::array(srcs)),
            ("missing_fields", Json::array(missing_json)),
        ])
    });

    let doc = Json::object([
        (
            "verified_entities",
            Json::Unsigned(report.verified_entities as u64),
        ),
        ("drift_count", Json::Unsigned(report.drifts.len() as u64)),
        ("drifts", Json::array(drifts_json)),
    ]);

    println!("{}", doc.render());
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use miter_engine::{ContractDrift, MissingFieldDrift};

    fn report(drifts: Vec<ContractDrift>) -> ContractReport {
        ContractReport {
            verified_entities: 3,
            drifts,
            suppressed: 0,
        }
    }

    fn drift(entity: &str, fields: &[&str]) -> ContractDrift {
        ContractDrift {
            entity: entity.to_string(),
            sources: vec!["rust".to_string(), "typescript".to_string()],
            missing_fields: fields
                .iter()
                .map(|field| MissingFieldDrift {
                    field_name: (*field).to_string(),
                    present_in: vec!["rust".to_string()],
                    missing_in: vec!["typescript".to_string()],
                })
                .collect(),
        }
    }

    #[test]
    fn test_drift_is_described_without_check_and_rejected_with_it() {
        let drifted = report(vec![
            drift("chat_message", &["content", "timestamp_ms"]),
            drift("typed_hole", &["kind"]),
        ]);

        // Describing is not failing: a person inspecting drift asks for the report, not a verdict.
        assert!(verdict(&drifted, false).is_ok());

        // With `--check` the same report is a failure, which is what a gate needs.
        let error = verdict(&drifted, true).expect_err("drift must fail the check");
        let message = error.to_string();
        assert!(message.contains("2 entities"), "got {message}");
        assert!(message.contains("3 field(s)"), "got {message}");
    }

    #[test]
    fn test_a_clean_report_passes_the_check() {
        let clean = report(Vec::new());
        assert!(verdict(&clean, true).is_ok());
    }

    #[test]
    fn test_a_single_entity_is_not_pluralised() {
        let one = report(vec![drift("chat_message", &["content"])]);
        let message = verdict(&one, true)
            .expect_err("drift must fail")
            .to_string();
        assert!(message.contains("1 entity"), "got {message}");
    }
}
