//! Tests for how references to dependencies are recognised (GB-232).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::imports::import_roots_in_source;

#[test]
fn test_a_qualified_path_counts_as_a_reference() {
    // GB-232: `regex::escape(id)` with no `use regex` statement is a genuine dependency reference.
    // This shape broke a real purge attempt in batten-core.
    let source = "fn f(idents: &[String]) -> String {
    idents.iter().map(|id| regex::escape(id)).collect()
}
";
    let roots = import_roots_in_source(source);
    assert!(
        roots.iter().any(|(root, _)| root == "regex"),
        "a qualified path must count as use; got {roots:?}"
    );
}

#[test]
fn test_a_local_module_does_not_mask_a_dependency_of_the_same_name() {
    // GB-232: batten-core has `mod regex;` (src/grammar/regex.rs) and also depends on the `regex`
    // crate. Filtering by module name alone made the crate look unused, and acting on that report
    // failed to compile.
    let own: std::collections::BTreeSet<String> =
        ["regex", "model"].iter().map(|s| s.to_string()).collect();
    let deps: std::collections::BTreeSet<String> =
        ["regex"].iter().map(|s| s.to_string()).collect();

    assert!(
        !crate::analysis::deps::resolution::masks_a_dependency("regex", &own, &deps),
        "a declared dependency must not be hidden by a same-named module"
    );
    assert!(
        crate::analysis::deps::resolution::masks_a_dependency("model", &own, &deps),
        "a plain local module still hides, or every module would become a dependency"
    );
    assert!(!crate::analysis::deps::resolution::masks_a_dependency(
        "serde", &own, &deps
    ));
}
