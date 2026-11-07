//! `miter deps --layers`: layer classification and the lateral-edge gate (`GB-202`).
//!
//! Tier 1 says who imports whom; this says whether an edge is allowed to exist. The map is a suite
//! property - no single repository can evaluate it - so `--check` is meant for the Tier 3 gate rather
//! than per-repo CI, which is what `GB-200` decided.

use super::error::CliError;
use super::json::Json;
use miter_engine::analysis::{
    Layer, LayerReport, analyze_dependencies, build_layer_report, load_allowlist,
};
use std::path::Path;

/// Where the allowlist lives, relative to the analysed root.
pub const DEFAULT_ALLOWLIST: &str = ".gaugeblock/miter/layers-allowlist.toml";

/// Allowlist maintenance requested on the command line (`GB-248`).
#[derive(Debug, Clone, Copy)]
pub struct AllowlistEdit<'a> {
    /// Delete entries that match no current edge.
    pub prune: bool,
    /// Write an entry for every unallowlisted lateral edge.
    pub adopt: bool,
    /// Ticket recorded on adopted entries.
    pub ticket: Option<&'a str>,
}

/// Runs `miter deps --layers`.
///
/// # Errors
///
/// [`CliError::Usage`] when the workspace or allowlist cannot be read, and [`CliError::Rejected`]
/// when `check` is set and the layering does not hold.
pub fn run_deps_layers(
    root: &Path,
    check: bool,
    allowlist: Option<&Path>,
    edit: AllowlistEdit<'_>,
    format: &str,
) -> Result<(), CliError> {
    let report = analyze_dependencies(root).map_err(|error| CliError::Usage(error.to_string()))?;

    let path = allowlist
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join(DEFAULT_ALLOWLIST));
    // An absent file means "no exceptions declared", which is the state Phase B2 is driving toward;
    // a malformed one is an error so a typo cannot pass the gate.
    let allow = load_allowlist(&path).map_err(|error| CliError::Usage(error.to_string()))?;
    let mut layers = build_layer_report(&report.edges, &allow);

    // Edits are applied before the report is shown, then the allowlist is re-read: reporting a stale entry
    // and then removing it in the same breath would print `violated` for a file that is now clean, which is
    // the kind of message that teaches a reader to distrust the verdict.
    if edit.prune || edit.adopt {
        super::deps_layers_edit::apply_edits(&layers, &path, &edit)?;
        let reloaded = load_allowlist(&path).map_err(|error| CliError::Usage(error.to_string()))?;
        layers = build_layer_report(&report.edges, &reloaded);
    }

    if format.eq_ignore_ascii_case("json") {
        render_json(&layers, &path);
    } else {
        render_table(&layers, &path);
    }

    if check && !layers.is_clean() {
        let missing = layers.unallowlisted_lateral().count();
        return Err(CliError::Rejected(format!(
            "layering violated: {missing} unallowlisted lateral edge(s), {} upward edge(s), {} stale allowlist entr(ies)",
            layers.upward.len(),
            layers.stale.len()
        )));
    }
    Ok(())
}

fn render_table(layers: &LayerReport, path: &Path) {
    println!(
        "Crate layers ({} edges):",
        layers.downward + layers.lateral.len() + layers.upward.len()
    );
    println!("{}", "=".repeat(80));
    for layer in [
        Layer::Foundation,
        Layer::Domain,
        Layer::Orchestrator,
        Layer::Integrator,
    ] {
        let names = layers.layers.get(&layer).cloned().unwrap_or_default();
        println!("{} ({}):", layer.label().to_uppercase(), names.len());
        for name in &names {
            println!("  {name}");
        }
    }

    println!();
    println!(
        "Lateral edges (same layer, need an allowlist entry): {}",
        layers.lateral.len()
    );
    for edge in &layers.lateral {
        match (&edge.ticket, edge.allowlisted) {
            (Some(ticket), _) => println!(
                "  {:<26} -> {:<26} {:>4} sites  allowed ({ticket})",
                edge.from, edge.to, edge.sites
            ),
            (None, _) => println!(
                "  {:<26} -> {:<26} {:>4} sites  UNALLOWLISTED",
                edge.from, edge.to, edge.sites
            ),
        }
    }

    if !layers.upward.is_empty() {
        println!();
        println!("Upward edges (must not exist): {}", layers.upward.len());
        for edge in &layers.upward {
            println!(
                "  {:<26} -> {:<26} {:>4} sites",
                edge.from, edge.to, edge.sites
            );
        }
    }

    if !layers.stale.is_empty() {
        println!();
        println!(
            "Stale allowlist entries (match no edge): {}",
            layers.stale.len()
        );
        for entry in &layers.stale {
            println!("  {} -> {} ({})", entry.from, entry.to, entry.ticket);
        }
    }

    println!();
    println!("Downward edges (unconstrained): {}", layers.downward);
    println!("Allowlist: {}", path.display());
    println!(
        "Verdict: {}",
        if layers.is_clean() {
            "clean"
        } else {
            "violated"
        }
    );
}

fn render_json(layers: &LayerReport, path: &Path) {
    let names = |layer: Layer| {
        Json::array(
            layers
                .layers
                .get(&layer)
                .map(|names| names.as_slice())
                .unwrap_or(&[])
                .iter()
                .map(Json::string),
        )
    };
    let edge_json = |edge: &miter_engine::analysis::LayerEdge| {
        Json::object([
            ("from", Json::string(&edge.from)),
            ("to", Json::string(&edge.to)),
            ("sites", Json::Unsigned(edge.sites as u64)),
            ("allowlisted", Json::Bool(edge.allowlisted)),
            (
                "ticket",
                match &edge.ticket {
                    Some(ticket) => Json::string(ticket),
                    None => Json::Null,
                },
            ),
        ])
    };

    println!(
        "{}",
        Json::object([
            ("clean", Json::Bool(layers.is_clean())),
            ("downward", Json::Unsigned(layers.downward as u64)),
            ("foundation", names(Layer::Foundation)),
            ("domain", names(Layer::Domain)),
            ("integrator", names(Layer::Integrator)),
            ("lateral", Json::array(layers.lateral.iter().map(edge_json)),),
            ("upward", Json::array(layers.upward.iter().map(edge_json))),
            ("allowlist", Json::string(path.display().to_string())),
            (
                "stale",
                Json::array(layers.stale.iter().map(|entry| {
                    Json::object([
                        ("from", Json::string(&entry.from)),
                        ("to", Json::string(&entry.to)),
                        ("ticket", Json::string(&entry.ticket)),
                    ])
                })),
            ),
        ])
        .render()
    );
}
