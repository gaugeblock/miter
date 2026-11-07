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
/// - `outfeed-orchestrator` — the release composition root (`GB-285`, `GB-286`). Cutting a suite release
///   *is* composing: read the backlog for what shipped, synthesize the changelog, verify, tag, record. The
///   crate exists as its own subcrate precisely so that `outfeed-core` need not name any of that, and its
///   edges to `storypole-core` and `sawhorse-core` are the composition itself rather than a leak.
///
/// Deliberately **not** listed, with the reason, because these are the interesting near-misses:
///
/// - `outfeed-core` used to compose storypole and sawhorse; its product is planning and applying a single
///   release, so that composition moved out to a sibling crate rather than being blessed here. Reading
///   a task receipt or synthesizing a changelog is a step inside that, not the point of it.
/// - `chalkline-core` reads storypole and miter-polyglot to audit documentation; documentation is the
///   product, and the reads are incidental.
///
/// Consequence, stated plainly: an edge from an orchestrator to a product crate is [`EdgeKind::Downward`]
/// and is not gated. That is the intended shape — a hub is *supposed* to reach every product — and it
/// is why the remaining allowlist entries are all edges between crates that are not composition roots.
pub const ORCHESTRATORS: &[&str] = &["joiner-core", "benchwork-core", "outfeed-orchestrator"];

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
