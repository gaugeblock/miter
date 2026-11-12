//! The database path for one invocation (`GB-195`: moved out of `mod.rs`, which had reached the band).

/// Resolves the workspace database path for one invocation.
///
/// An explicit `--db` wins. Otherwise the command works in the root of the current directory —
/// the nearest `.gaugeblock/` or `.git/` marker at or above it — and opens
/// `<root>/.gaugeblock/miter/miter.redb` (legacy `<root>/.miter/miter.redb` is still read).
/// There is one root and one database per invocation: nothing is
/// registered, attached or kept alive between runs.
#[must_use]
pub fn resolve_db(explicit: Option<std::path::PathBuf>) -> std::path::PathBuf {
    explicit.unwrap_or_else(|| {
        let start = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        miter_storage::default_db_path(&miter_storage::resolve_workspace_root(&start))
    })
}
