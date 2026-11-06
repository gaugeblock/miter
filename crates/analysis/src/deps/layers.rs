//! Layer model for the crate graph (`GB-202`, `GB-204`): foundation / domain / orchestrator /
//! integrator.
//!
//! Tier 1 of `miter deps` answers *who imports whom*. That is not enough to say whether an edge is
//! acceptable, because direction is what matters: a domain crate may lean on a foundation crate, and
//! an integrator may lean on anything, but two domain crates importing each other weld two product
//! surfaces together that were meant to evolve independently. Six such edges exist today, and the
//! point of this module is to make them visible and countable rather than found by hand.
//!
//! The layering is derived from crate names, so it needs no per-repo configuration and cannot drift
//! from the workspace: `gaugeblock-*` is foundation, a bare name is the root facade (an integrator),
//! `-cli`/`-mcp` are integrators, and everything else is a domain crate. `-core`, `-engine`, `-ast`
//! and `-storage` land in the domain layer by falling through, which is intentional: they are all
//! product surfaces.
//!
//! **The one exception is [`ORCHESTRATORS`]**, and it is not derived from names because no name can
//! express it: a composition root and a product core have the same shape. The exception is a named,
//! documented list rather than a heuristic, and `miter deps --layers` prints every crate's layer so
//! that a mistake in it is visible rather than silent.

use super::CrateEdge;
use super::allowlist::LayerAllowlist;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Crates whose *product* is composing other products.
///
/// The criterion is deliberately narrow, because a looser one dissolves the layer: every crate in the
/// suite consumes something. A crate is an orchestrator when composition **is** its product rather
/// than a step inside it.
///
/// - `joiner-core` — the synthesis loop: pattern lookup in `dado`, grammar from `batten`, inference
///   from `sawhorse`, verification through `miter-harness`. Its value is the loop.
/// - `benchwork-core` — the workbench hub: the daemon behind the UI, which drives storypole's backlog,
///   chalkline's docs engine, joiner's synthesis and dado's patterns on the operator's behalf.
///
/// Deliberately **not** listed, with the reason, because these are the interesting near-misses:
///
/// - `outfeed-core` composes storypole and sawhorse, but its product is release and delivery; reading
///   a task receipt or synthesizing a changelog is a step inside that, not the point of it.
/// - `chalkline-core` reads storypole and miter-polyglot to audit documentation; documentation is the
///   product, and the reads are incidental.
///
/// Consequence, stated plainly: an edge from an orchestrator to a product crate is [`EdgeKind::Downward`]
/// and is not gated. That is the intended shape — a hub is *supposed* to reach every product — and it
/// is why the remaining allowlist entries are all edges between crates that are not composition roots.
pub const ORCHESTRATORS: &[&str] = &["joiner-core", "benchwork-core"];

/// Where a crate sits in the suite's dependency layering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Layer {
    /// Shared infrastructure with no product semantics (`gaugeblock-*`).
    Foundation,
    /// A product surface (`<product>-core`, `<product>-engine`, `<product>-ast`, ...).
    Domain,
    /// A composition root: a crate whose product is driving other products ([`ORCHESTRATORS`]).
    Orchestrator,
    /// A wiring layer: binaries, protocol adapters and root facades.
    Integrator,
}

impl Layer {
    /// Ordinal used to compare layers; a lower rank may be depended on freely.
    #[must_use]
    pub fn rank(self) -> u8 {
        match self {
            Layer::Foundation => 0,
            Layer::Domain => 1,
            Layer::Orchestrator => 2,
            Layer::Integrator => 3,
        }
    }

    /// Stable label for reports and JSON.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Layer::Foundation => "foundation",
            Layer::Domain => "domain",
            Layer::Orchestrator => "orchestrator",
            Layer::Integrator => "integrator",
        }
    }
}

/// Classifies a crate by its package name, consulting [`ORCHESTRATORS`] first.
#[must_use]
pub fn classify_crate(name: &str) -> Layer {
    if ORCHESTRATORS.contains(&name) {
        Layer::Orchestrator
    } else if name.starts_with("gaugeblock-") {
        Layer::Foundation
    } else if name.ends_with("-cli") || name.ends_with("-mcp") || !name.contains('-') {
        Layer::Integrator
    } else {
        Layer::Domain
    }
}

/// Direction of an edge relative to the layer order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Toward a lower layer. Always acceptable.
    Downward,
    /// Within one layer. Acceptable only when allowlisted with a ticket.
    Lateral,
    /// Within one layer but structurally expected: one product's own crates, or shared
    /// infrastructure composing. Not debt, so not gated.
    Internal,
    /// Toward a higher layer. Never acceptable: a foundation crate depending on a product surface
    /// cannot be extracted, and a domain crate importing a binary's internals cannot be tested
    /// without it.
    Upward,
}

impl EdgeKind {
    /// Stable label for reports and JSON.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            EdgeKind::Downward => "downward",
            EdgeKind::Lateral => "lateral",
            EdgeKind::Internal => "internal",
            EdgeKind::Upward => "upward",
        }
    }
}

/// Classifies one edge by the layers of its endpoints.
#[must_use]
pub fn classify_edge(from: Layer, to: Layer) -> EdgeKind {
    match from.rank().cmp(&to.rank()) {
        core::cmp::Ordering::Greater => EdgeKind::Downward,
        core::cmp::Ordering::Equal => EdgeKind::Lateral,
        core::cmp::Ordering::Less => EdgeKind::Upward,
    }
}

/// Product family a crate belongs to: the leading segment before the first hyphen.
///
/// `miter`, `miter-engine`, `miter-ast` and `miter-cli` are one family. Crates that exist to serve a
/// single product may depend on each other freely, because how a product splits itself is that
/// product's business rather than cross-suite coupling.
#[must_use]
pub fn family(name: &str) -> &str {
    name.split('-').next().unwrap_or(name)
}

/// Full assessment of an edge: layer direction, with same-layer structural cases excepted.
///
/// Three same-layer shapes are not debt and must not be gated: crates within one product family
/// (`miter-engine -> miter-ast` at 71 sites, `miter-cli -> miter-mcp`), foundation-to-foundation edges
/// (`gaugeblock-common -> gaugeblock-contracts`), and orchestrator-to-orchestrator edges, where one
/// declared composition root drives another (`benchwork-core -> joiner-core`: the hub dispatches the
/// synthesis loop). Everything else keeps its layer verdict, which leaves cross-product domain edges
/// as the debt `GB-204` burns down.
///
/// Note the direction this change makes *stricter*, not looser: before the orchestrator layer existed,
/// a domain crate importing a composition root was merely lateral and therefore allowlistable. It is
/// now upward, and an upward edge is never excused.
#[must_use]
pub fn assess_edge(from: &str, to: &str) -> EdgeKind {
    let (from_layer, to_layer) = (classify_crate(from), classify_crate(to));
    if from_layer == to_layer
        && (family(from) == family(to)
            || from_layer == Layer::Foundation
            || from_layer == Layer::Orchestrator)
    {
        return EdgeKind::Internal;
    }
    classify_edge(from_layer, to_layer)
}

/// An edge that breaks the layering, or a lateral edge explained by the allowlist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerEdge {
    /// Importing crate.
    pub from: String,
    /// Imported crate.
    pub to: String,
    /// How many import sites prove the edge.
    pub sites: usize,
    /// Whether the allowlist covers it.
    pub allowlisted: bool,
    /// Ticket carrying the debt, when allowlisted.
    pub ticket: Option<String>,
}

/// Layer assignment plus every edge that needs attention.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerReport {
    /// Crates per layer, sorted.
    pub layers: BTreeMap<Layer, Vec<String>>,
    /// All lateral edges, allowlisted or not.
    pub lateral: Vec<LayerEdge>,
    /// Edges pointing at a higher layer.
    pub upward: Vec<LayerEdge>,
    /// Count of downward edges, which need no scrutiny.
    pub downward: usize,
    /// Same-layer edges that are structurally expected rather than debt.
    pub internal: usize,
}

impl LayerReport {
    /// True when nothing violates the layering and every lateral edge carries a ticket.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.upward.is_empty() && self.lateral.iter().all(|edge| edge.allowlisted)
    }

    /// Lateral edges with no allowlist entry.
    pub fn unallowlisted_lateral(&self) -> impl Iterator<Item = &LayerEdge> {
        self.lateral.iter().filter(|edge| !edge.allowlisted)
    }
}

/// Builds the layer view from tier-1 edges.
#[must_use]
pub fn build_layer_report(edges: &[CrateEdge], allowlist: &LayerAllowlist) -> LayerReport {
    let mut layers: BTreeMap<Layer, Vec<String>> = BTreeMap::new();
    let mut lateral = Vec::new();
    let mut upward = Vec::new();
    let mut downward = 0;
    let mut internal = 0;

    for edge in edges {
        let from = classify_crate(&edge.from);
        let to = classify_crate(&edge.to);
        layers.entry(from).or_default().push(edge.from.clone());
        layers.entry(to).or_default().push(edge.to.clone());

        match assess_edge(&edge.from, &edge.to) {
            EdgeKind::Downward => downward += 1,
            EdgeKind::Internal => internal += 1,
            EdgeKind::Lateral => {
                let entry = allowlist.entry(&edge.from, &edge.to);
                lateral.push(LayerEdge {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    sites: edge.sites.len(),
                    allowlisted: entry.is_some(),
                    ticket: entry.map(|item| item.ticket.clone()),
                });
            }
            EdgeKind::Upward => upward.push(LayerEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
                sites: edge.sites.len(),
                // An upward edge is never excused: the allowlist is for break-even debt between
                // peers, not for inversion that makes a crate impossible to extract or test.
                allowlisted: false,
                ticket: None,
            }),
        }
    }

    for names in layers.values_mut() {
        names.sort();
        names.dedup();
    }
    lateral.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    upward.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    LayerReport {
        layers,
        lateral,
        upward,
        downward,
        internal,
    }
}
