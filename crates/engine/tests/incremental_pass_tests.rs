//! A second index pass must re-parse nothing it has already parsed (`GB-302`).
//!
//! `miter index` parsed every file on every run even though the database already held a baseline
//! describing each one. The ledger was written, persisted and never read by the default pass, so the
//! cost of an index was the whole tree - 958 files - and the output said "indexed 958 files", which is
//! indistinguishable from correct work being done.
//!
//! These tests assert the contract the CLI now delegates to: a pass over an unchanged tree parses
//! nothing and removes nothing, and a pass over one edited file parses exactly that file.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use miter_engine::Engine;
use miter_engine::persist::{load_redb, load_redb_universal, save_redb};
use miter_storage::WorkspaceStore;
use tempfile::tempdir;

/// Writes a two-file Rust tree under `root`.
fn tree(root: &Path) {
    std::fs::create_dir_all(root.join("alpha/src")).unwrap();
    std::fs::write(
        root.join("alpha/src/lib.rs"),
        "pub fn alpha_one() -> u8 { 1 }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("alpha/src/two.rs"),
        "pub fn alpha_two() -> u8 { 2 }\n",
    )
    .unwrap();
}

/// Indexes `root` with the full pass, saves it, and returns how many symbols were persisted.
fn full_index(root: &Path, db: &Path) -> usize {
    let mut engine = Engine::new();
    engine.ingest_dir(root).unwrap();
    engine.ingest_polyglot_tree(root).unwrap();
    save_redb(&engine, db).unwrap();
    load_redb(db).unwrap().len()
}

#[test]
fn a_second_pass_over_an_unchanged_tree_parses_nothing() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    tree(root);
    let db = root.join(".gaugeblock").join("miter").join("miter.redb");

    let saved = full_index(root, &db);
    assert!(saved > 0, "the first pass indexes the tree");
    let universal_before = load_redb_universal(&db).unwrap().len();

    let store = WorkspaceStore::open(&db).unwrap();
    let mut engine = Engine::new();
    let stats = miter_engine::ingest_dirty(&mut engine, root, &store).unwrap();

    assert_eq!(
        stats.files_indexed, 0,
        "nothing changed, so nothing is parsed: {stats:?}"
    );
    assert_eq!(
        stats.files_skipped, 2,
        "both files are recognised as unchanged: {stats:?}"
    );
    drop(store);

    assert_eq!(
        load_redb(&db).unwrap().len(),
        saved,
        "skipping a file must not drop the rows it already owns"
    );
    assert_eq!(
        load_redb_universal(&db).unwrap().len(),
        universal_before,
        "the polyglot declarations of a skipped file are not dropped either"
    );
}

#[test]
fn one_edited_file_is_the_only_one_reparsed() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    tree(root);
    let db = root.join(".gaugeblock").join("miter").join("miter.redb");
    full_index(root, &db);

    std::fs::write(
        root.join("alpha/src/two.rs"),
        "pub fn renamed_two() -> u8 { 22 }\n",
    )
    .unwrap();

    let store = WorkspaceStore::open(&db).unwrap();
    let mut engine = Engine::new();
    let stats = miter_engine::ingest_dirty(&mut engine, root, &store).unwrap();

    assert_eq!(stats.files_indexed, 1, "one file changed: {stats:?}");
    assert_eq!(stats.files_skipped, 1, "the other is untouched: {stats:?}");
    drop(store);
    let symbols = load_redb(&db).unwrap();
    assert!(
        symbols.iter().any(|s| s.name.contains("renamed_two")),
        "the edited declaration reached the database"
    );
    assert!(
        symbols.iter().any(|s| s.name.contains("alpha_one")),
        "the untouched file kept its declaration"
    );
}
