//! `miter deps`: crate dependency graph, proof of use and declared-but-unused detection.

use super::error::CliError;
use super::json::Json;
use miter_engine::analysis::{CrateDependencyReport, analyze_dependencies};
use std::path::Path;

/// Format choice for `miter deps`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepsFormat {
    /// Human-readable aligned report.
    Table,
    /// Machine-readable JSON output.
    Json,
}

impl DepsFormat {
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

/// Runs the `miter deps` command.
///
/// `why` narrows the report to the crates that import `target`, and `unused_only` drops the edge
/// section entirely, so the three tier-1 questions share one traversal.
pub fn run_deps(
    root: &Path,
    why: Option<&str>,
    unused_only: bool,
    format: DepsFormat,
) -> Result<(), CliError> {
    let report = analyze_dependencies(root).map_err(|error| CliError::Usage(error.to_string()))?;

    match format {
        DepsFormat::Table => render_table(&report, why, unused_only, root),
        DepsFormat::Json => render_json(&report, why, unused_only),
    }

    Ok(())
}

/// Edges to print, honouring `--why`.
fn selected_edges<'a>(
    report: &'a CrateDependencyReport,
    why: Option<&str>,
) -> Vec<&'a miter_engine::analysis::CrateEdge> {
    report
        .edges
        .iter()
        .filter(|edge| match why {
            Some(target) => edge.to.eq_ignore_ascii_case(target),
            None => true,
        })
        .collect()
}

fn render_table(report: &CrateDependencyReport, why: Option<&str>, unused_only: bool, root: &Path) {
    println!(
        "Crate Dependency Graph ({} crates under {}):",
        report.crates_analyzed,
        root.display()
    );
    println!("{}", "=".repeat(80));

    let edges = selected_edges(report, why);
    if !unused_only {
        match why {
            Some(target) => println!("Proof of use for `{target}`:"),
            None => println!("Import edges between workspace crates:"),
        }
        if edges.is_empty() {
            println!("  (none)");
        } else {
            println!("{:<26} {:<26} {:>6}", "CRATE", "IMPORTS", "SITES");
            println!("{}", "-".repeat(80));
            for edge in &edges {
                println!("{:<26} {:<26} {:>6}", edge.from, edge.to, edge.sites.len());
            }
            if why.is_some() {
                println!("\nSites:");
                for edge in &edges {
                    for site in &edge.sites {
                        println!("  {}:{}  <- {}", site.file, site.line, edge.from);
                    }
                }
            }
        }
        println!();
    }

    println!(
        "Declared dependencies no source file imports: {}",
        report.unused.len()
    );
    if !report.unused.is_empty() {
        println!("{:<26} {:<26} {:<10}", "CRATE", "DEPENDENCY", "DECLARED");
        println!("{}", "-".repeat(80));
        for item in &report.unused {
            println!(
                "{:<26} {:<26} {:<10}",
                item.crate_name, item.dependency, item.declared_version
            );
        }
    }
}

fn render_json(report: &CrateDependencyReport, why: Option<&str>, unused_only: bool) {
    let edges = selected_edges(report, why).into_iter().map(|edge| {
        let sites = edge.sites.iter().map(|site| {
            Json::object([
                ("file", Json::string(&site.file)),
                ("line", Json::Unsigned(site.line as u64)),
            ])
        });
        Json::object([
            ("from", Json::string(&edge.from)),
            ("to", Json::string(&edge.to)),
            ("sites", Json::array(sites)),
        ])
    });

    let unused = report.unused.iter().map(|item| {
        Json::object([
            ("crate_name", Json::string(&item.crate_name)),
            ("dependency", Json::string(&item.dependency)),
            ("declared_version", Json::string(&item.declared_version)),
        ])
    });

    let mut fields = vec![
        (
            "crates_analyzed",
            Json::Unsigned(report.crates_analyzed as u64),
        ),
        ("unused_count", Json::Unsigned(report.unused.len() as u64)),
        ("unused", Json::array(unused)),
    ];
    if !unused_only {
        fields.insert(1, ("edges", Json::array(edges)));
    }

    println!("{}", Json::object(fields).render());
}
