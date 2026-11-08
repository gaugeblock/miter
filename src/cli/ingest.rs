//! The `index` command that parses, extracts symbols, and persists an index.
//!
//! Records what a declaration is — its id and its provenance.
//! Universal declarations and cross-file identifier references are also recorded.

use std::path::Path;

use miter_engine::Engine;

use super::{CliError, index_mode, json, store};

/// Populates REFERENCES_TABLE in WorkspaceStore from polyglot extracted references.
fn populate_references(
    db: &Path,
    references: &[(String, Vec<(String, miter_storage::ReferenceEntry)>)],
) {
    if let Ok(ws_store) = miter_storage::WorkspaceStore::open_or_recreate(db) {
        let all_refs = references.iter().flat_map(|(_, refs)| refs.clone());
        let _ = ws_store.replace_all_references(all_refs);
    }
}

/// `miter index <PATH> [--db <DB>] [--json]`.
///
/// Discovers every supported file below `path`, encodes the functions it finds, and
/// persists the whole index along with universal declarations and cross-file references.
///
/// # Errors
///
/// [`CliError::Engine`] when traversal fails, [`CliError::Persist`] when database fails.
pub(crate) fn run_index(
    path: &Path,
    db: &Path,
    dirty: bool,
    full: bool,
    json: bool,
) -> Result<(), CliError> {
    // The baseline decides the mode. A database that already describes this tree is re-read rather
    // than re-parsed; before `GB-302` the default pass parsed all 958 files every time and wrote the
    // same rows back, so the ledger it kept could never make it faster.
    let store = miter_storage::WorkspaceStore::open_or_recreate(db)
        .map_err(|e| CliError::Persist(miter_engine::PersistError::Redb(e.to_string())))?;
    let has_baseline = store
        .list_indexed_files()
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    let pass = index_mode::choose_pass(dirty, full, has_baseline);
    if pass != index_mode::Pass::Full {
        let mode = match pass {
            index_mode::Pass::Dirty => "dirty",
            _ => "incremental",
        };
        let mut engine = Engine::new();
        let stats = miter_engine::ingest_dirty(&mut engine, path, &store)?;
        report_pass(mode, db, &stats, engine.universal_symbols().len(), json);
        return Ok(());
    }
    // The full pass writes through the path-based saver, and `redb` refuses a second handle on the
    // same file, so this one is released before it is reopened by path.
    drop(store);

    let mut engine = Engine::new();
    let (stats, polyglot) = if json {
        let stats = engine.ingest_dir(path)?;
        let polyglot = engine.ingest_polyglot_tree(path)?;
        store::save(&engine, db)?;
        populate_references(db, &polyglot.references);
        (stats, polyglot)
    } else {
        let files = miter_ast::discover_sources(path)
            .map(|files| files.len())
            .unwrap_or_default();
        let bar = indicatif::ProgressBar::new(files as u64).with_style(miter_ast::indexing_style());
        bar.set_draw_target(indicatif::ProgressDrawTarget::stderr());
        bar.set_prefix("[1/2] ");
        let stats = engine.ingest_dir_with(path, &bar)?;

        let all_files = miter_engine::ingest::discover_all_sources(path)
            .map(|f| f.len())
            .unwrap_or(files);
        let poly_bar =
            indicatif::ProgressBar::new(all_files as u64).with_style(miter_ast::indexing_style());
        poly_bar.set_draw_target(indicatif::ProgressDrawTarget::stderr());
        poly_bar.set_prefix("[2/2] ");
        let polyglot = engine.ingest_polyglot_tree_with(path, &poly_bar)?;

        poly_bar.set_prefix("");
        poly_bar.set_message("persisting index & references...");
        store::save(&engine, db)?;
        populate_references(db, &polyglot.references);

        poly_bar.finish_and_clear();
        (stats, polyglot)
    };

    if json {
        println!(
            "{}",
            json::Json::object([
                ("command", json::Json::string("index")),
                ("mode", json::Json::string("full")),
                ("path", json::Json::string(path.display().to_string())),
                ("database", json::Json::string(db.display().to_string())),
                (
                    "files_scanned",
                    json::Json::Unsigned(stats.files_scanned as u64)
                ),
                (
                    "files_indexed",
                    json::Json::Unsigned(stats.files_indexed as u64)
                ),
                ("symbols", json::Json::Unsigned(stats.symbols as u64)),
                (
                    "universal_symbols",
                    json::Json::Unsigned(engine.universal_symbols().len() as u64),
                ),
                ("duplicates", json::Json::Unsigned(stats.duplicates as u64)),
                (
                    "failures",
                    json::Json::array(stats.failures.iter().cloned().map(json::Json::string)),
                ),
            ])
            .render()
        );
    } else {
        println!(
            "indexed {} symbols ({} universal) from {}/{} files into {}",
            stats.symbols,
            engine.universal_symbols().len(),
            // The polyglot pass is the one that reads every format, so its counts are the ones a
            // reader means by "files"; the function-shaped pass sees only the code languages and
            // would report 0/0 for a tree of manifests and stylesheets.
            polyglot.stats.files_indexed,
            polyglot.stats.files_scanned,
            db.display()
        );
        for failure in stats.failures.iter().chain(polyglot.stats.failures.iter()) {
            eprintln!("  skipped: {failure}");
        }
    }
    Ok(())
}

/// Renders the counters of an incremental pass, naming which mode ran.
///
/// The mode is in the output because "indexed 0 symbols" means something different when every file
/// was deliberately skipped: a pass that does not say which one it was makes an unchanged tree and a
/// broken traversal print the same line.
fn report_pass(
    mode: &str,
    db: &Path,
    stats: &miter_engine::store::IncrementalStats,
    universal: usize,
    json: bool,
) {
    if json {
        println!(
            "{}",
            json::Json::object([
                ("command", json::Json::string("index")),
                ("mode", json::Json::string(mode)),
                ("database", json::Json::string(db.display().to_string())),
                (
                    "files_scanned",
                    json::Json::Unsigned(stats.files_scanned as u64)
                ),
                (
                    "files_indexed",
                    json::Json::Unsigned(stats.files_indexed as u64)
                ),
                (
                    "files_skipped",
                    json::Json::Unsigned(stats.files_skipped as u64)
                ),
                ("symbols", json::Json::Unsigned(stats.symbols as u64)),
                ("universal_symbols", json::Json::Unsigned(universal as u64)),
                (
                    "failures",
                    json::Json::array(stats.failures.iter().cloned().map(json::Json::string)),
                ),
            ])
            .render()
        );
        return;
    }
    println!(
        "{mode}-indexed: {} symbols ({} universal) from {} files ({} skipped, {} scanned) into {}",
        stats.symbols,
        universal,
        stats.files_indexed,
        stats.files_skipped,
        stats.files_scanned,
        db.display()
    );
    for failure in &stats.failures {
        eprintln!("  skipped: {failure}");
    }
}
