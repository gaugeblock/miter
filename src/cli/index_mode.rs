//! Which pass `miter index` runs (`GB-302`).
//!
//! The decision lives apart from the command so it can be asserted. The command itself needs a
//! database, a tree and a parsed argument list, so the only thing a test there could pin down is the
//! absence of a panic - and the defect was a choice, not a crash.

/// The pass `miter index` will execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Re-parse the files git or mtime report as changed (`--dirty`).
    Dirty,
    /// Re-read the stored baseline and re-parse only what differs from it.
    Incremental,
    /// Parse every file and replace the whole index (`--full`, or a database with no baseline).
    Full,
}

/// Chooses the pass from the flags and whether the database already describes a tree.
///
/// An empty baseline means the database cannot know what changed, so the full pass is the only honest
/// answer; a populated one means re-parsing the tree repeats work whose result is already stored.
pub(crate) fn choose_pass(dirty: bool, full: bool, has_baseline: bool) -> Pass {
    if dirty {
        Pass::Dirty
    } else if full || !has_baseline {
        Pass::Full
    } else {
        Pass::Incremental
    }
}

#[cfg(test)]
mod tests;
