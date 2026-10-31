//! Tests for the layer model (`GB-202`), split from `layers.rs` to stay inside the file-size law.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::CrateEdge;
use super::allowlist::{AllowlistEntry, LayerAllowlist, load_allowlist};
use super::layers::{
    EdgeKind, Layer, assess_edge, build_layer_report, classify_crate, classify_edge, family,
};

fn edge(from: &str, to: &str) -> CrateEdge {
    CrateEdge {
        from: from.to_string(),
        to: to.to_string(),
        sites: Vec::new(),
    }
}

#[test]
fn test_crates_classify_by_name() {
    assert_eq!(classify_crate("gaugeblock-contracts"), Layer::Foundation);
    assert_eq!(classify_crate("gaugeblock-git"), Layer::Foundation);
    assert_eq!(classify_crate("miter-engine"), Layer::Domain);
    assert_eq!(classify_crate("storypole-core"), Layer::Domain);
    assert_eq!(classify_crate("miter-storage"), Layer::Domain);
    assert_eq!(classify_crate("storypole-cli"), Layer::Integrator);
    assert_eq!(classify_crate("benchwork-mcp"), Layer::Integrator);
    assert_eq!(
        classify_crate("miter"),
        Layer::Integrator,
        "a bare name is the root facade"
    );
}

#[test]
fn test_direction_follows_the_layer_order() {
    assert_eq!(
        classify_edge(Layer::Domain, Layer::Foundation),
        EdgeKind::Downward
    );
    assert_eq!(
        classify_edge(Layer::Domain, Layer::Domain),
        EdgeKind::Lateral
    );
    assert_eq!(
        classify_edge(Layer::Foundation, Layer::Domain),
        EdgeKind::Upward
    );
    assert_eq!(
        classify_edge(Layer::Domain, Layer::Integrator),
        EdgeKind::Upward
    );
}

#[test]
fn test_unallowlisted_lateral_edge_fails_the_gate() {
    let report = build_layer_report(
        &[edge("storypole-core", "chalkline-core")],
        &LayerAllowlist::default(),
    );
    assert!(!report.is_clean(), "an unexplained lateral edge is debt");
    assert_eq!(report.lateral.len(), 1);
    assert_eq!(report.lateral[0].ticket, None);
    assert_eq!(report.unallowlisted_lateral().count(), 1);
}

#[test]
fn test_allowlisting_a_lateral_edge_clears_it_and_keeps_the_ticket() {
    let allowlist = LayerAllowlist {
        lateral: vec![AllowlistEntry {
            from: "storypole-core".to_string(),
            to: "chalkline-core".to_string(),
            ticket: "GB-204".to_string(),
        }],
    };
    let report = build_layer_report(&[edge("storypole-core", "chalkline-core")], &allowlist);
    assert!(report.is_clean());
    assert_eq!(report.lateral[0].ticket.as_deref(), Some("GB-204"));
    assert_eq!(report.unallowlisted_lateral().count(), 0);
}

#[test]
fn test_an_allowlist_entry_cannot_excuse_an_upward_edge() {
    let allowlist = LayerAllowlist {
        lateral: vec![AllowlistEntry {
            from: "gaugeblock-git".to_string(),
            to: "storypole-core".to_string(),
            ticket: "GB-204".to_string(),
        }],
    };
    let report = build_layer_report(&[edge("gaugeblock-git", "storypole-core")], &allowlist);
    assert_eq!(report.upward.len(), 1);
    assert!(!report.is_clean(), "inverted edges are never excused");
}

#[test]
fn test_downward_edges_are_counted_not_flagged() {
    let report = build_layer_report(
        &[
            edge("miter-engine", "gaugeblock-contracts"),
            edge("miter", "miter-engine"),
        ],
        &LayerAllowlist::default(),
    );
    // A facade importing its own engine is Integrator -> Domain, which is downward and fine; the
    // same-family exemption only matters within a layer (`miter-engine` <-> `miter-ast`).
    assert_eq!(report.downward, 2);
    assert_eq!(report.internal, 0);
    assert!(report.is_clean());
    assert_eq!(report.layers[&Layer::Domain], vec!["miter-engine"]);
    assert_eq!(report.layers[&Layer::Integrator], vec!["miter"]);
}

#[test]
fn test_absent_allowlist_file_means_no_exceptions() {
    let missing = std::path::Path::new("/nonexistent-layers-allowlist.toml");
    let allowlist = load_allowlist(missing).unwrap();
    assert!(allowlist.lateral.is_empty());
}

#[test]
fn test_a_malformed_allowlist_is_an_error_rather_than_a_free_pass() {
    let path = std::env::temp_dir().join(format!("layers-bad-{}.toml", std::process::id()));
    std::fs::write(&path, "lateral = not-a-table").unwrap();
    assert!(load_allowlist(&path).is_err());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_family_is_the_leading_segment() {
    assert_eq!(family("miter-engine"), "miter");
    assert_eq!(family("miter"), "miter");
    assert_eq!(family("gaugeblock-contracts"), "gaugeblock");
    assert_eq!(family("joiner-core"), "joiner");
}

#[test]
fn test_same_product_internals_are_not_lateral_debt() {
    assert_eq!(assess_edge("miter-engine", "miter-ast"), EdgeKind::Internal);
    assert_eq!(assess_edge("miter-cli", "miter-mcp"), EdgeKind::Internal);
    assert_eq!(assess_edge("joiner", "joiner-cli"), EdgeKind::Internal);
}

#[test]
fn test_foundation_crates_may_compose() {
    assert_eq!(
        assess_edge("gaugeblock-common", "gaugeblock-contracts"),
        EdgeKind::Internal
    );
}

#[test]
fn test_cross_product_domain_edges_stay_gated() {
    assert_eq!(
        assess_edge("joiner-core", "dado-core"),
        EdgeKind::Lateral,
        "a product importing another product's core is the debt GB-204 burns"
    );
    assert_eq!(assess_edge("storypole-cli", "miter-mcp"), EdgeKind::Lateral);
}

#[test]
fn test_a_domain_crate_importing_an_integrator_is_upward() {
    assert_eq!(
        assess_edge("joiner-core", "joiner-cli"),
        EdgeKind::Upward,
        "a product core must not depend on that product's binary"
    );
}
