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
