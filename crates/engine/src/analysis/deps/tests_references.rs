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

#[test]
fn test_a_char_literal_holding_a_quote_does_not_eat_the_file() {
    // GB-233: a char literal whose value is a double quote. The stripper treated that quote as
    // opening a string and blanked every line after it, so a real dependency further down the file
    // was reported unused - outfeed-core/src/index_poll.rs, where reqwest sat at line 63.
    //
    // A raw string keeps the fixture readable: the awkward literal can appear as itself.
    let source = r#"fn pick(line: &str) -> Option<&str> {
    line.split('"').nth(1)
}

fn fetch(url: &str) {
    let _ = reqwest::blocking::get(url);
}
"#;
    let roots = import_roots_in_source(source);
    assert!(
        roots.iter().any(|(root, _)| root == "reqwest"),
        "a dependency after a quote-holding char literal must still count; got {roots:?}"
    );
}

#[test]
fn test_lifetimes_are_not_char_literals() {
    // The distinction that makes the fix safe: a lifetime must not be consumed as a literal, or the
    // scanner would swallow a character and mis-read the rest of the line.
    let source = r#"fn f<'a>(s: &'a str) -> &'a str { s }
fn g() { serde_json::json!({}); }
"#;
    let roots = import_roots_in_source(source);
    assert!(
        roots.iter().any(|(root, _)| root == "serde_json"),
        "got {roots:?}"
    );
}
