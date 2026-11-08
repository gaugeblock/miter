//! A saved index must remember the files it was built from (`GB-302`, `GB-305`).
//!
//! Before this existed, `save_redb` wrote symbols and references and no per-file baseline, because the only
//! writers of the ledger were the incremental passes themselves - and `save_redb` deletes and rebuilds the
//! database from the engine, so anything an incremental pass wrote was erased by the next full index.
//!
//! The consequence was invisible rather than loud: an index with no baseline cannot detect a change, so
//! `miter index --dirty` reported `0 files scanned, 0 skipped` and read as "nothing to do". Two repositories
//! in scope were required to see it, and nobody had a test with two.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use miter_engine::Engine;
use miter_engine::persist::save_redb;
use miter_storage::{IndexedFileRecord, WorkspaceStore};
use tempfile::tempdir;

/// A record for a file with known size and mtime.
fn record(digest_byte: u8, size: u64, mtime_secs: u64) -> IndexedFileRecord {
    IndexedFileRecord::new(mtime_secs, 0, [digest_byte; 32], size)
}

#[test]
fn a_saved_index_remembers_every_file_it_describes() {
    let tmp = tempdir().expect("tempdir");
    let db = tmp.path().join(".gaugeblock").join("miter").join("miter.redb");

    let mut engine = Engine::new();
    // Two members of one suite, which is the shape the defect needed: one repository hid it.
    engine.record_file("alpha/src/lib.rs".to_string(), record(1, 120, 1_700_000_000));
    engine.record_file("beta/src/main.rs".to_string(), record(2, 240, 1_700_000_100));
    save_redb(&engine, &db).expect("save the index");

    let store = WorkspaceStore::open(&db).expect("reopen the index");
    assert_eq!(store.file_hash_count().expect("hashes"), 2, "both files are recorded");

    let alpha = store
        .get_indexed_file("alpha/src/lib.rs")
        .expect("read alpha")
        .expect("alpha is present");
    assert_eq!(alpha.size_bytes, 120);
    assert_eq!(alpha.mtime_secs, 1_700_000_000);

    let beta = store
        .get_indexed_file("beta/src/main.rs")
        .expect("read beta")
        .expect("beta is present");
    assert_eq!(beta.size_bytes, 240, "the second member is not lost");

    // The digest table is what the polyglot pass will skip on, so it must agree with the record.
    assert_eq!(
        store.file_hash("beta/src/main.rs").expect("read hash"),
        Some([2u8; 32]),
        "the hash and the record describe the same file"
    );
}

#[test]
fn saving_an_engine_with_no_ledger_writes_no_rows_rather_than_stale_ones() {
    let tmp = tempdir().expect("tempdir");
    let db = tmp.path().join("miter.redb");

    let mut first = Engine::new();
    first.record_file("gamma/src/lib.rs".to_string(), record(3, 10, 1));
    save_redb(&first, &db).expect("first save");

    // A second save from an engine that carries no baseline must not resurrect the previous rows: the file is
    // rebuilt from the engine, and a reader must never see a ledger describing files the index does not have.
    save_redb(&Engine::new(), &db).expect("second save");
    let store = WorkspaceStore::open(&db).expect("reopen");
    assert_eq!(store.file_hash_count().expect("hashes"), 0);
}
