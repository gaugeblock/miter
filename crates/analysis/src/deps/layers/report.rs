//! The layer report: which edges need attention, and which allowlist entries explain nothing.
//!
//! Split from the classification module when the file passed the 250-line limit (`GB-291`). Classification
//! answers "what layer is this crate"; the report answers "is the graph acceptable", and the second question
//! is the one that keeps growing as exceptions accumulate.

use super::*;

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
    /// Allowlist entries that match no current edge.
    ///
    /// A stale entry is not harmless bookkeeping: it reads as a live exception, so the next person
    /// believes an edge still exists and can waste an afternoon resolving one that was already burned.
    /// Twice in this suite a burned edge left its entry behind, because nothing checked (`GB-248`).
    pub stale: Vec<crate::deps::allowlist::AllowlistEntry>,
}

impl LayerReport {
    /// True when nothing violates the layering and every lateral edge carries a ticket.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.upward.is_empty()
            && self.lateral.iter().all(|edge| edge.allowlisted)
            && self.stale.is_empty()
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

    // An entry is stale when no lateral edge claimed it. Collected from the input allowlist rather than
    // from a set difference over strings, so an entry naming a crate that no longer exists still counts.
    let stale: Vec<_> = allowlist
        .lateral
        .iter()
        .filter(|entry| {
            !lateral
                .iter()
                .any(|edge| edge.from == entry.from && edge.to == entry.to)
        })
        .cloned()
        .collect();

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
        stale,
    }
}
