//! The shapes a contract report is made of.
//!
//! Split from the comparison so neither file has to hold both the data and the rules (`GB-239`).

use serde::{Deserialize, Serialize};

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
    /// Fields suppressed by a declared exception, counted so the report never hides one.
    #[serde(default)]
    pub suppressed: usize,
}
