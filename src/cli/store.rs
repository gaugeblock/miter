//! Database open/save helpers.
//!
//! The backend is `redb`. The records are self-describing (`miter_engine::codec`), so an index
//! written by one build is readable by any build that shares the codec version.

use std::path::Path;

use miter_engine::Engine;

use super::CliError;

/// Saves the whole index, creating the file if necessary.
///
/// # Errors
///
/// [`CliError::Persist`] when the backend cannot write the database.
pub(crate) fn save(engine: &Engine, path: &Path) -> Result<(), CliError> {
    engine.save_redb(path)?;
    Ok(())
}

/// Loads an index, reporting a friendly error when the file is missing.
///
/// An *absent* index is built rather than refused (`GB-322`): the harness verification chain begins with
/// `miter contracts`, which needs an index, so a fresh clone could not pass its own gate until someone
/// ran `miter index` by hand - and an instruction inside a verification chain is a step nobody notices
/// is missing. This is the same precedent as `GB-237` for a stale index: a read that needs the index
/// brings it up to date rather than describing the problem to the caller.
///
/// `MITER_NO_AUTO_INDEX` still means "do not write an index", for CI and for gates that must not.
///
/// # Errors
///
/// [`CliError::MissingIndex`] when the file is absent and auto-indexing is off, [`CliError::Persist`]
/// when it cannot be read, built or decoded.
pub(crate) fn load(path: &Path) -> Result<Engine, CliError> {
    if !path.exists() {
        build_if_missing(path)?;
    }
    // Read the mtime *before* opening: redb rewrites the file on open, so a timestamp taken
    // afterwards is always "now" and the check silently passes.
    let indexed_at = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok();
    // Refresh before opening the read handle: the refresh needs the write handle, and redb allows one
    // at a time. Auto-indexing is a policy over a mechanism that now detects staleness correctly
    // (`GB-237`), so read commands answer from a current index instead of warning about an old one.
    if let Some(indexed_at) = indexed_at {
        refresh_if_stale(path, indexed_at);
    }
    Ok(Engine::open_redb(path)?)
}

/// Builds the index when it does not exist yet (`GB-322`).
///
/// Mirrors what `miter index` does without its progress reporting: the passes, the save, and the
/// reference table. The duplication is deliberate - `run_index` owns the CLI's output and job counts,
/// and threading a "quiet" mode through it would put a flag in a command that already has three.
fn build_if_missing(path: &Path) -> Result<(), CliError> {
    if std::env::var_os(NO_AUTO_INDEX_ENV).is_some() {
        return Err(CliError::MissingIndex(path.to_path_buf()));
    }
    // `<root>/.gaugeblock/miter/miter.redb` - the index sits inside the tree it describes.
    let Some(root) = path.parent().and_then(Path::parent).and_then(Path::parent) else {
        return Err(CliError::MissingIndex(path.to_path_buf()));
    };
    let started = std::time::Instant::now();
    let mut engine = Engine::new();
    engine.ingest_dir(root)?;
    let polyglot = engine.ingest_polyglot_tree(root)?;
    save(&engine, path)?;
    if let Ok(store) = miter_storage::WorkspaceStore::open_or_recreate(path) {
        let references = polyglot
            .references
            .iter()
            .flat_map(|(_, refs)| refs.clone());
        let _ = store.replace_all_references(references);
    }
    // Stderr, because stdout belongs to `--json` and this is a statement about work the caller did not
    // ask for and should still know happened.
    eprintln!(
        "miter: no index at {}; built it in {} ms ({} files). Set {NO_AUTO_INDEX_ENV} to forbid this.",
        path.display(),
        started.elapsed().as_millis(),
        polyglot.stats.files_indexed
    );
    Ok(())
}

/// Source files an index is expected to describe.
const SOURCE_EXTENSIONS: [&str; 5] = ["rs", "ts", "tsx", "sql", "py"];

/// Directories whose mtimes say nothing about the code.
const SKIPPED_DIRS: [&str; 6] = [
    "target",
    "node_modules",
    ".git",
    ".gaugeblock",
    "dist",
    "e2e",
];

/// Env var that turns auto-indexing off, for CI and for gates that must not write.
///
/// Spelled `MITER_`, one T, because the tool is `miter`. It read `MITTER_NO_AUTO_INDEX` until
/// 2026-11-07 - a name nobody would guess correctly, and one that a reader who *did* guess correctly
/// would get wrong. See the test below, which exists so the typo cannot come back.
pub(crate) const NO_AUTO_INDEX_ENV: &str = "MITER_NO_AUTO_INDEX";

/// Refreshes the index in place when a source file is newer than it.
///
/// Silent on the happy path and explicit on every other: a refresh says how many files it touched on
/// stderr (stdout belongs to `--json`), and a refresh that cannot take the write lock says so rather
/// than hanging or failing. `redb` is single-writer and an MCP server holds the index for hours, so
/// contention is the normal case rather than an error.
fn refresh_if_stale(db: &Path, indexed_at: std::time::SystemTime) {
    if std::env::var_os(NO_AUTO_INDEX_ENV).is_some() {
        // Opting out of the refresh must not opt out of the warning: the caller has said it wants the
        // index left alone, not that it wants to be told nothing about it.
        warn_if_stale(db, indexed_at);
        return;
    }
    let Some(root) = db.parent().and_then(Path::parent).and_then(Path::parent) else {
        return;
    };
    let Some(newest) = newest_source_mtime(root, indexed_at) else {
        return;
    };
    let Ok(age) = newest.duration_since(indexed_at) else {
        return;
    };
    // The same tolerance the warning used: a file touched seconds ago is mid-edit, and re-indexing on
    // every keystroke would cost more than it saves.
    if age.as_secs() < 60 {
        return;
    }
    let Ok(store) = miter_storage::WorkspaceStore::open(db) else {
        eprintln!(
            "miter: note: index is {} minute(s) stale but is held by another process; answering from it",
            age.as_secs() / 60
        );
        return;
    };
    let started = std::time::Instant::now();
    let mut engine = Engine::new();
    // The two halves report different error types, so they are sequenced rather than chained: the
    // message is the same to a reader, and flattening them into one type would lose which half failed.
    let refreshed = miter_engine::ingest_dirty(&mut engine, root, &store).and_then(|_stats| {
        miter_engine::persist::save_redb_into(&store, &engine)
            .map_err(miter_engine::EngineError::Persist)
    });
    match refreshed {
        Ok(()) => eprintln!(
            "miter: index was {} minute(s) stale; refreshed in {} ms",
            age.as_secs() / 60,
            started.elapsed().as_millis()
        ),
        Err(error) => eprintln!("miter: could not refresh the stale index: {error}"),
    }
}

/// Warns on stderr when the index predates the newest source file it should describe (`GB-237`).
///
/// A stale index fails silently: a lookup for a symbol that exists answers "no declaration named",
/// which is indistinguishable from the symbol never having existed. That cost a session - a contract
/// report was taken against a three-day-old index, its field count was wrong by one, and symbols added
/// hours earlier were invisible. The warning goes to stderr so `--json` stays parseable, and it is a
/// warning rather than an error because a read against a slightly old index is usually what a caller
/// mid-edit wants.
fn warn_if_stale(db: &Path, indexed_at: std::time::SystemTime) {
    // `<root>/.gaugeblock/miter/miter.redb` - the index sits inside the tree it describes.
    let Some(root) = db.parent().and_then(Path::parent).and_then(Path::parent) else {
        return;
    };
    let Some(newest) = newest_source_mtime(root, indexed_at) else {
        return;
    };
    let Ok(age) = newest.duration_since(indexed_at) else {
        return;
    };
    if age.as_secs() < 60 {
        return;
    }
    eprintln!(
        "miter: warning: index is {} minute(s) older than a source file under {}; results may be stale (run `miter index {}`)",
        age.as_secs() / 60,
        root.display(),
        root.display()
    );
}

/// The newest source mtime under `root`, returning early once one is newer than `indexed_at`.
fn newest_source_mtime(
    root: &Path,
    indexed_at: std::time::SystemTime,
) -> Option<std::time::SystemTime> {
    let mut newest = None;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                let name = entry.file_name();
                if !SKIPPED_DIRS.iter().any(|skip| name == *skip) {
                    pending.push(entry.path());
                }
                continue;
            }
            let is_source = entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext));
            if !is_source {
                continue;
            }
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if modified.duration_since(indexed_at).is_ok() {
                return Some(modified);
            }
            if newest.is_none_or(|best| modified.duration_since(best).is_ok()) {
                newest = Some(modified);
            }
        }
    }
    newest
}

#[cfg(test)]
mod tests;
