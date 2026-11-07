//! Which crates are composition roots, and the classification that consults them.
//!
//! Split out of `layers.rs` when that file crossed the 250-line limit with the orchestrator list in it.

use super::layers::Layer;

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
