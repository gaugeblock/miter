//! Name-resolution rules: which segment of a path names a crate (`GB-290`).

use super::imports::import_roots_in_source;

#[test]
fn only_the_first_segment_of_a_path_names_a_crate() {
    // `GB-290`: a module named after another product manufactured an edge to that product.
    let roots = import_roots_in_source(
        "fn f() { outfeed_orchestrator::storypole::gather(dir, &basis, false); }",
    );
    let names: Vec<&str> = roots.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["outfeed_orchestrator"]);
}

#[test]
fn a_root_relative_path_still_names_its_crate() {
    let roots = import_roots_in_source("fn f() { let _ = ::serde_json::json!({}); }");
    let names: Vec<&str> = roots.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["serde_json"]);
}

#[test]
fn a_single_segment_call_is_still_counted() {
    let roots = import_roots_in_source(r#"fn f() { tracing::warn!("x"); }"#);
    let names: Vec<&str> = roots.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["tracing"]);
}
