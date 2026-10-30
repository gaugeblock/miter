//! Unit tests for `miter deps` tier 1 (`GB-161`).

use super::{
    CrateEdge, DeclaredCrate, ImportSite, assemble_report, crate_ident, declared_modules,
    import_roots_in_source,
};
use std::collections::BTreeMap;

fn crate_with(name: &str, deps: &[(&str, &str)]) -> DeclaredCrate {
    DeclaredCrate {
        name: name.to_string(),
        dependencies: deps
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
    }
}

fn site(file: &str, line: usize) -> ImportSite {
    ImportSite {
        file: file.to_string(),
        line,
    }
}

#[test]
fn test_crate_ident_normalises_hyphens() {
    assert_eq!(crate_ident("storypole-core"), "storypole_core");
    assert_eq!(crate_ident("serde"), "serde");
}

#[test]
fn test_import_roots_are_first_segments_only() {
    let source = "\
use storypole_core::model::TaskNode;
use gaugeblock_contracts::{SpoolEntry, SyncStatus};
extern crate rustyline;
";
    let roots = import_roots_in_source(source);
    assert_eq!(
        roots,
        vec![
            ("storypole_core".to_string(), 1),
            ("gaugeblock_contracts".to_string(), 2),
            ("rustyline".to_string(), 3),
        ]
    );
}

#[test]
fn test_language_roots_and_comments_are_skipped() {
    let source = "\
// use commented_out::Thing;
use std::collections::HashMap;
use crate::model::Task;
use super::sibling;
use self::inner;
    use std::path::Path;
";
    assert!(
        import_roots_in_source(source).is_empty(),
        "only crate roots count as dependencies"
    );
}

#[test]
fn test_fully_qualified_paths_count_as_use() {
    let source = "\
let payload = serde_json::to_string(&task).expect(\"json\");
tracing::warn!(ticket = %key, \"trace capture failed\");
StorypoleStore::now_ms();
";
    let roots = import_roots_in_source(source);
    assert_eq!(
        roots,
        vec![("serde_json".to_string(), 1), ("tracing".to_string(), 2),],
        "a dependency used without a use statement is still used; a bare type is not a path"
    );
}

#[test]
fn test_pub_use_and_pub_crate_use_are_counted() {
    let source = "\
pub use gaugeblock_contracts::SpoolEntry;
pub(crate) use serde_json::Value;
";
    let roots = import_roots_in_source(source);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].0, "gaugeblock_contracts");
    assert_eq!(roots[1].0, "serde_json");
}

#[test]
fn test_assembly_keeps_workspace_edges_and_ignores_external_imports() {
    let crates = vec![
        crate_with("storypole-core", &[("gaugeblock-contracts", "0.1")]),
        crate_with("gaugeblock-contracts", &[]),
    ];
    let mut imports: BTreeMap<String, BTreeMap<String, Vec<ImportSite>>> = BTreeMap::new();
    imports.insert(
        "storypole-core".to_string(),
        BTreeMap::from([
            (
                "gaugeblock_contracts".to_string(),
                vec![site("storypole/src/telemetry/mod.rs", 14)],
            ),
            ("serde".to_string(), vec![site("storypole/src/lib.rs", 3)]),
        ]),
    );
    imports.insert("gaugeblock-contracts".to_string(), BTreeMap::new());

    let report = assemble_report(&crates, &imports);

    assert_eq!(report.crates_analyzed, 2);
    assert_eq!(
        report.edges,
        vec![CrateEdge {
            from: "storypole-core".to_string(),
            to: "gaugeblock-contracts".to_string(),
            sites: vec![site("storypole/src/telemetry/mod.rs", 14)],
        }],
        "serde is external, so it is not a crate edge"
    );
    assert!(
        report.unused.is_empty(),
        "the declared dependency is imported"
    );
}

#[test]
fn test_declared_but_unimported_dependency_is_reported() {
    let crates = vec![crate_with(
        "joiner-core",
        &[("sha2", "0.10"), ("serde", "1.0")],
    )];
    let mut imports: BTreeMap<String, BTreeMap<String, Vec<ImportSite>>> = BTreeMap::new();
    imports.insert(
        "joiner-core".to_string(),
        BTreeMap::from([("serde".to_string(), vec![site("joiner/src/lib.rs", 1)])]),
    );

    let report = assemble_report(&crates, &imports);

    assert_eq!(report.unused.len(), 1);
    assert_eq!(report.unused[0].crate_name, "joiner-core");
    assert_eq!(report.unused[0].dependency, "sha2");
    assert_eq!(report.unused[0].declared_version, "0.10");
}

#[test]
fn test_a_crate_importing_itself_is_not_an_edge() {
    let crates = vec![crate_with("rabbet-core", &[])];
    let mut imports: BTreeMap<String, BTreeMap<String, Vec<ImportSite>>> = BTreeMap::new();
    imports.insert(
        "rabbet-core".to_string(),
        BTreeMap::from([(
            "rabbet_core".to_string(),
            vec![site("rabbet/src/lib.rs", 2)],
        )]),
    );

    let report = assemble_report(&crates, &imports);
    assert!(report.edges.is_empty(), "a self-import is not a dependency");
}

#[test]
fn test_edges_and_unused_are_sorted_for_stable_output() {
    let crates = vec![
        crate_with("b-core", &[("zeta", "1"), ("alpha", "1")]),
        crate_with("a-core", &[("beta", "1")]),
    ];
    let imports: BTreeMap<String, BTreeMap<String, Vec<ImportSite>>> = BTreeMap::new();

    let report = assemble_report(&crates, &imports);

    assert_eq!(report.unused[0].crate_name, "a-core");
    assert_eq!(report.unused[1].dependency, "alpha");
    assert_eq!(report.unused[2].dependency, "zeta");
}

#[test]
fn test_local_module_names_shadow_a_crate_of_the_same_name() {
    let source = "\
pub mod outfeed;\nmod storypole;\npub(crate) mod sawhorse;\nmod inline { }\npub use outfeed::*;\n";
    let mut modules = declared_modules(source);
    modules.sort();
    assert_eq!(
        modules,
        vec![
            "inline".to_string(),
            "outfeed".to_string(),
            "sawhorse".to_string(),
            "storypole".to_string()
        ],
        "a crate module must be known so its re-export is not read as a dependency"
    );
}

#[test]
fn test_import_looking_content_inside_strings_is_not_a_reference() {
    let source = "\
let fixture = \"pub mod outfeed;\\nuse storypole_core::model::TaskNode;\\n\";\nlet raw = r\"use sawhorse_core::InferenceRequest;\";\nlet hashed = r#\"use chalkline_core::SpecGraph;\"#;\n";
    assert!(
        import_roots_in_source(source).is_empty(),
        "string literals are data, not dependency proof (GB-199)"
    );
    assert!(
        declared_modules(source).is_empty(),
        "a fixture must not register a module either"
    );
}

#[test]
fn test_a_multiline_raw_string_swallows_the_lines_it_covers() {
    let source = "let spec = r#\"\nuse dado_core::Pattern;\n\"#;\nuse batten_core::Grammar;\n";
    assert_eq!(
        import_roots_in_source(source),
        vec![("batten_core".to_string(), 4)],
        "only the line after the raw string closes is real code"
    );
}

#[test]
fn test_comments_hide_imports_but_real_code_after_a_block_comment_still_counts() {
    let source = "\
// use commented::Out;\n/* use block_commented::Out; */ use real_crate::Thing;\n";
    assert_eq!(
        import_roots_in_source(source),
        vec![("real_crate".to_string(), 2)],
        "a trailing block comment must not swallow the statement before it"
    );
}

#[test]
fn test_a_string_continued_with_a_trailing_backslash_does_not_leak_code() {
    // The exact shape that produced the phantom edge: a fixture string continued onto the next
    // physical line with a trailing backslash (GB-199).
    let source = "let source = \"\\\npub mod outfeed;\\nuse storypole_core::x;\\n\";\nuse real_crate::Thing;\n";
    assert_eq!(
        import_roots_in_source(source),
        vec![("real_crate".to_string(), 3)],
        "the continued fixture line is data; only the following statement is code"
    );
    assert!(
        !declared_modules(source).contains(&"outfeed".to_string()),
        "a continued fixture must not register a module"
    );
}
