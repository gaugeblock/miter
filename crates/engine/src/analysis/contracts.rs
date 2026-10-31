//! Polyglot Contract Bridge across SQL, Rust, and TypeScript.
//!
//! Verifies cross-language schema consistency across SQLite DDL tables, Rust structs,
//! and TypeScript interfaces/types. Flags contract drift.

use crate::polyglot::{SymbolKind, UniversalSymbol};
use miter_storage::store::WorkspaceStore;
use std::collections::{BTreeSet, HashMap};

mod allowlist;
mod compare;
mod fields;
mod model;

pub use allowlist::{ContractAllowlist, ContractException, load as load_contract_allowlist};
use compare::{depth_differs, is_subsumed};
pub use fields::{
    canonical_entity_name, extract_fields_from_symbol, fields_of, normalize_identifier,
};
pub use model::{ContractDrift, ContractReport, EntityField, MissingFieldDrift, PolyglotEntity};

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
    verify_contracts_with_allowlist(symbols, &ContractAllowlist::default())
}

/// Evaluates polyglot contracts, honouring declared non-mirrors.
#[must_use]
pub fn verify_contracts_with_allowlist(
    symbols: &[UniversalSymbol],
    allow: &ContractAllowlist,
) -> ContractReport {
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
            if allow.covers(&canonical_name, &field).is_some() {
                report.suppressed += 1;
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
