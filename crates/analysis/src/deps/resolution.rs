//! Name-resolution rules for the dependency graph (`GB-232`).
//!

use std::collections::BTreeSet;

/// Whether a local module name should hide a reference to that name.
///
/// `mod regex;` and the `regex` crate can coexist, and `regex::escape` then means the crate: a
/// module's own name is not in scope inside itself. Filtering by name alone made a live dependency
/// look dead, and a purge attempt on that report failed to compile (`GB-232`). When the name is a
/// declared dependency the reference is counted; a genuinely local module named like a dependency is
/// rare enough that preferring the dependency is the safer error.
pub(crate) fn masks_a_dependency(
    ident: &str,
    own_modules: &BTreeSet<String>,
    declared_dependencies: &BTreeSet<String>,
) -> bool {
    if !own_modules.contains(ident) {
        return false;
    }
    !declared_dependencies.contains(ident)
}

/// Whether `before` ends somewhere a path may begin.
///
/// Only the *first* segment of a path names a crate. Every later segment is a module or an item, and
/// counting them manufactured edges that did not exist: `outfeed_orchestrator::storypole::gather(..)`
/// reported a dependency on a crate called `storypole`, which then appeared as a lateral edge in the
/// layering gate and sent someone renaming a module to silence a finding that was never true (`GB-290`).
///
/// A path continues while the characters before it are an identifier or `:`. A leading `::` is a path
/// that starts at a root, so it is allowed: the character before the first colon is not an identifier.
pub(crate) fn starts_a_path(before: &str) -> bool {
    let mut rev = before.chars().rev();
    match (rev.next(), rev.next(), rev.next()) {
        // `a::b`: the two colons belong to an identifier on their left, so this is a later segment.
        (Some(':'), Some(':'), Some(previous)) => {
            !(previous.is_alphanumeric() || previous == '_' || previous == ':')
        }
        // A leading `::` starts a path at a root, and has no identifier to its left.
        _ => true,
    }
}
