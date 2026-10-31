//! Architectural analysis and AST seam slicing.
//!
//! Exposes:
//! - [`audit`]: Scans workspace source files for line-count threshold violations.
//! - [`mod@slice`]: Analyzes AST co-reference graphs to identify module seams and blueprint extractions.
//! - [`cluster`]: Partitions symbol subgraphs into cohesive ~200–250 line extraction clusters.

pub mod audit;
pub use miter_analysis::clones;
pub use miter_analysis::cluster;
pub mod contracts;
pub mod cycles;
pub mod deps;
pub mod dirty;
pub mod hygiene;
pub mod impact;
pub use miter_analysis::references;
pub use miter_analysis::skeleton;
pub use miter_analysis::slice;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_contracts;
#[cfg(test)]
mod tests_references;

pub use audit::{AuditConfig, AuditSeverity, AuditViolation, WorkspaceAuditor};
pub use clones::{
    CloneDetectionReport, FunctionFingerprint, StructuralClonePair, compute_simhash,
    extract_fingerprints_from_rust, find_clones, hamming_distance, similarity_score,
};
pub use cluster::{CoupledCluster, DeclarationNode};
pub use contracts::{
    ContractAllowlist, ContractDrift, ContractException, ContractReport, EntityField,
    MissingFieldDrift, PolyglotEntity, canonical_entity_name, load_contract_allowlist,
    normalize_identifier, verify_contracts_from_symbols, verify_contracts_with_allowlist,
};
pub use cycles::{CycleDetectionReport, DependencyCycle, FileDependencyGraph};
pub use deps::{
    AllowlistEntry, CrateDependencyReport, CrateEdge, DeclaredCrate, DepsError, EdgeKind,
    ImportSite, Layer, LayerAllowlist, LayerEdge, LayerReport, UnusedDependency,
    analyze_dependencies, assemble_report, build_layer_report, classify_crate, classify_edge,
    collect_crate_imports, crate_ident, import_roots_in_source, load_allowlist,
};
pub use dirty::{DirtyScan, detect_dirty_files, ingest_dirty};
pub use hygiene::{DeadSymbol, HygieneReport, VisibilityCandidate, analyze_hygiene};
pub use impact::{
    AffectedTest, EntrypointHandler, ImpactAnalysis, TransitiveCaller, compute_impact,
};
pub use references::extract_references;
pub use skeleton::{SkeletonOptions, SkeletonResult, extract_skeleton};
pub use slice::{ExtractionBlueprint, SeamSlicer};
