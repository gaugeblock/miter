//! Polyglot Contract Bridge across SQL, Rust, and TypeScript.
//!
//! Verifies cross-language schema consistency across SQLite DDL tables, Rust structs,
//! and TypeScript interfaces/types. Flags contract drift.

use crate::polyglot::{SymbolKind, UniversalSymbol};
use miter_storage::store::WorkspaceStore;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

mod fields;

pub use fields::{
    canonical_entity_name, extract_fields_from_symbol, fields_of, normalize_identifier,
};

/// A field extracted from an entity declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityField {
    /// Raw field name as written in source code.
    pub raw_name: String,
    /// Canonical normalized field name for cross-language matching.
    pub canonical_name: String,
    /// Associated type annotation if available.
    pub type_hint: Option<String>,
}

/// A parsed cross-language entity representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolyglotEntity {
    /// Canonical entity name (e.g. "resident_ledger").
    pub canonical_name: String,
    /// Language family or source kind: "sql", "rust", or "typescript".
    pub source_kind: String,
    /// Raw declaration name (e.g., "residents", "ResidentLedger").
    pub raw_name: String,
    /// Defining file path.
    pub file_path: String,
    /// Line number.
    pub line: usize,
    /// Set of fields declared on this entity.
    pub fields: Vec<EntityField>,
}

/// Drift report for a matched cross-language entity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDrift {
    /// Canonical entity name.
    pub entity: String,
    /// Representation sources evaluated (e.g., ["sql", "rust", "typescript"]).
    pub sources: Vec<String>,
    /// Fields present in some representations but missing in others.
    pub missing_fields: Vec<MissingFieldDrift>,
}

/// A missing field inconsistency across representations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingFieldDrift {
    /// Canonical field name.
    pub field_name: String,
    /// Sources that define this field.
    pub present_in: Vec<String>,
    /// Sources that are missing this field.
    pub missing_in: Vec<String>,
}

/// Overall polyglot contract verification report.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractReport {
    /// Entities matched across 2 or more languages.
    pub verified_entities: usize,
    /// Detected contract drifts.
    pub drifts: Vec<ContractDrift>,
}

/// Evaluates polyglot contracts across stored universal symbols.
pub fn verify_contracts_from_store(
    store: &WorkspaceStore,
) -> Result<ContractReport, miter_storage::StorageError> {
    let symbols = crate::persist::load_redb_universal_from(store).unwrap_or_default();
    Ok(verify_contracts_from_symbols(&symbols))
}

/// Evaluates polyglot contracts across a collection of `UniversalSymbol`s.
#[must_use]
pub fn verify_contracts_from_symbols(symbols: &[UniversalSymbol]) -> ContractReport {
    let mut entities_by_name: HashMap<String, Vec<PolyglotEntity>> = HashMap::new();

    for sym in symbols {
        let is_candidate = matches!(
            sym.kind,
            SymbolKind::Table | SymbolKind::Struct | SymbolKind::Interface
        );
        if !is_candidate {
            continue;
        }

        let fields = fields_of(symbols, sym);
        if fields.is_empty() {
            continue;
        }

        let canonical_name = canonical_entity_name(&sym.name);
        let source_kind = match sym.kind {
            SymbolKind::Table => "sql",
            SymbolKind::Struct => "rust",
            SymbolKind::Interface => "typescript",
            _ => "unknown",
        };

        entities_by_name
            .entry(canonical_name.clone())
            .or_default()
            .push(PolyglotEntity {
                canonical_name,
                source_kind: source_kind.to_string(),
                raw_name: sym.name.clone(),
                file_path: sym.file.to_string_lossy().replace('\\', "/"),
                line: sym.line,
                fields,
            });
    }

    let mut report = ContractReport::default();

    for (canonical_name, variants) in entities_by_name {
        // Only verify entities that exist across at least 2 distinct language kinds
        let mut kinds: BTreeSet<String> = BTreeSet::new();
        for v in &variants {
            kinds.insert(v.source_kind.clone());
        }
        if kinds.len() < 2 {
            continue;
        }

        report.verified_entities += 1;

        // Collect all canonical fields across all variants
        let mut all_fields: BTreeSet<String> = BTreeSet::new();
        let mut fields_by_kind: HashMap<String, BTreeSet<String>> = HashMap::new();

        for v in &variants {
            for f in &v.fields {
                all_fields.insert(f.canonical_name.clone());
                fields_by_kind
                    .entry(v.source_kind.clone())
                    .or_default()
                    .insert(f.canonical_name.clone());
            }
        }

        let mut missing_fields = Vec::new();
        let mut all_missing: Vec<(String, String)> = Vec::new();
        for field in all_fields {
            let mut present_in = Vec::new();
            let mut missing_in = Vec::new();

            for kind in &kinds {
                if fields_by_kind
                    .get(kind)
                    .is_some_and(|fset| fset.contains(&field))
                {
                    present_in.push(kind.clone());
                } else {
                    missing_in.push(kind.clone());
                }
            }

            if missing_in.is_empty() {
                continue;
            }
            // Two ways a line can look like drift without being any, both measured on the suite's own
            // types (`GB-239`).
            if depth_differs(&field, &fields_by_kind, &missing_in) {
                continue;
            }
            if is_subsumed(&field, &missing_in, &all_missing) {
                continue;
            }
            for kind in &missing_in {
                all_missing.push((field.clone(), kind.clone()));
            }
            missing_fields.push(MissingFieldDrift {
                field_name: field,
                present_in,
                missing_in,
            });
        }

        if !missing_fields.is_empty() {
            report.drifts.push(ContractDrift {
                entity: canonical_name,
                sources: kinds.into_iter().collect(),
                missing_fields,
            });
        }
    }

    report.drifts.sort_by(|a, b| a.entity.cmp(&b.entity));
    report
}

/// Whether the same shape is simply nested at a different depth on each side.
///
/// An index does not go equally deep in every grammar - TypeScript inlines an object one level down,
/// Rust names a struct and can be followed further - so `docs.agents` on one side and
/// `docs.agents.path` on the other describe one field, not a missing one. Either relation settles it:
/// the other side has something below this path, or this path is below something the other side has.
fn depth_differs(
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
fn is_subsumed(field: &str, missing_in: &[String], all_missing: &[(String, String)]) -> bool {
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
