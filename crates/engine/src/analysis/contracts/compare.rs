//! Two rules that keep nesting out of the drift report (`GB-239`).
//!
//! Both are properties of a pair of representations rather than of either one, which is why they live
//! beside the comparison instead of inside the field walk.

use std::collections::{BTreeSet, HashMap};

/// Whether the same shape is simply nested at a different depth on each side.
///
/// An index does not go equally deep in every grammar - TypeScript inlines an object one level down,
/// Rust names a struct and can be followed further - so `docs.agents` on one side and
/// `docs.agents.path` on the other describe one field, not a missing one. Either relation settles it:
/// the other side has something below this path, or this path is below something the other side has.
pub(crate) fn depth_differs(
    field: &str,
    fields_by_kind: &HashMap<String, BTreeSet<String>>,
    missing_in: &[String],
) -> bool {
    let below = format!("{field}.");
    missing_in.iter().any(|kind| {
        fields_by_kind.get(kind).is_some_and(|fields| {
            fields.iter().any(|other| {
                // The other side has something beneath this path, or this path sits beneath
                // something the other side has. Either way the shape is nested, not missing.
                other.starts_with(&below) || field.starts_with(&format!("{other}."))
            })
        })
    })
}

/// Whether a nested path is already reported by its parent.
///
/// If `verification` is missing, then `verification.diagnostics.code` is the same omission counted
/// eleven more times. The parent line is the one a person acts on.
pub(crate) fn is_subsumed(
    field: &str,
    missing_in: &[String],
    all_missing: &[(String, String)],
) -> bool {
    let mut prefix = field;
    while let Some((parent, _)) = prefix.rsplit_once('.') {
        if missing_in
            .iter()
            .any(|kind| all_missing.iter().any(|(p, k)| p == parent && k == kind))
        {
            return true;
        }
        prefix = parent;
    }
    false
}
