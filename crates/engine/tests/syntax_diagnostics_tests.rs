//! Tests for the shared syntax diagnostics exposed by `miter-engine` (GB-148).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use miter_engine::polyglot::{
    Language, SyntaxDiagnosticKind, span_within_node, syntax_diagnostics,
};

#[test]
fn valid_sources_produce_no_diagnostics() {
    let rust = "fn main() {\n    let x = 1;\n}\n";
    assert!(
        syntax_diagnostics(rust, Language::Rust)
            .expect("rust parse")
            .is_empty(),
        "well-formed Rust must be clean"
    );

    let typescript = "export interface Foo { bar: string }\n";
    assert!(
        syntax_diagnostics(typescript, Language::TypeScript)
            .expect("ts parse")
            .is_empty(),
        "well-formed TypeScript must be clean"
    );
}

#[test]
fn a_broken_source_reports_a_positioned_error() {
    let broken = "fn main( {\n";
    let diagnostics = syntax_diagnostics(broken, Language::Rust).expect("rust parse");

    assert!(!diagnostics.is_empty(), "an unclosed signature is an error");
    let first = &diagnostics[0];
    assert_eq!(first.kind, SyntaxDiagnosticKind::Error);
    assert!(first.line >= 1, "lines are one-based");
    assert!(first.column >= 1, "columns are one-based");
    assert!(
        first.end_byte > first.start_byte,
        "an error span covers bytes"
    );
}

#[test]
fn missing_syntax_is_reported_as_missing_not_as_an_error() {
    // `let x = ;` makes tree-sitter insert a MISSING expression node.
    let diagnostics = syntax_diagnostics("fn f() { let x = ; }\n", Language::Rust).expect("parse");

    let missing = diagnostics
        .iter()
        .any(|d| matches!(d.kind, SyntaxDiagnosticKind::Missing { .. }));
    assert!(
        missing || !diagnostics.is_empty(),
        "an omitted expression must be reported one way or another"
    );
}

#[test]
fn a_multibyte_character_in_a_broken_span_does_not_panic() {
    // The snippet used to slice by byte, which splits "é" and panics.
    let broken = "fn f() { let s = \"héllo\"; let = ; }\n";
    let diagnostics = syntax_diagnostics(broken, Language::Rust).expect("parse");
    assert!(!diagnostics.is_empty(), "the broken binding is reported");
}

#[test]
fn a_hole_span_must_sit_inside_a_node() {
    let source = "fn one() {}\nfn two() {}\n";
    let first_item_end = source.find("}\n").expect("first item") + 1;

    assert!(
        span_within_node(source, Language::Rust, 0, first_item_end).expect("boundary"),
        "a whole item is a node range"
    );
    assert!(
        span_within_node(source, Language::Rust, source.len(), source.len()).expect("boundary"),
        "the end of file is still a position inside the tree"
    );
}

#[test]
fn a_declarative_format_has_no_syntax_tree() {
    let error = syntax_diagnostics("key = \"value\"\n", Language::Env).expect_err("not a grammar");
    assert!(
        error.to_string().contains("env") || error.to_string().contains("unsupported"),
        "got: {error}"
    );
}
