//! Workspace line-count auditor.
//!
//! Types live here; the traversal lives in [`auditor`], nested-repository discovery in
//! [`nested`], and per-file measurement rules in [`walk`].

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Severity classification of a line count violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// 500–799 lines: File is growing large and should be monitored.
    Warning,
    /// ≥ 800 lines: Violates the configured modularity ceiling and must be decomposed.
    Critical,
}

impl AuditSeverity {
    /// Human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

/// A violation flagged during workspace auditing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditViolation {
    /// Path to the violating file.
    pub path: PathBuf,
    /// Total physical line count.
    pub line_count: usize,
    /// Number of top-level declarations identified.
    pub declaration_count: usize,
    /// Violation severity: "Warning" or "Critical".
    pub severity: AuditSeverity,
}

/// Configuration options for workspace auditing.
#[derive(Clone, Debug)]
pub struct AuditConfig {
    /// Maximum allowed lines before a warning or violation is reported (default: 500).
    pub max_lines: usize,
    /// Optional glob patterns or directory substrings to ignore.
    pub ignore_patterns: Vec<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_lines: 500,
            ignore_patterns: Vec::new(),
        }
    }
}

impl AuditConfig {
    /// Creates a configuration with the given line ceiling.
    #[must_use]
    pub fn new(max_lines: usize) -> Self {
        Self {
            max_lines,
            ignore_patterns: Vec::new(),
        }
    }

    /// Resolves audit configuration from the workspace, overriding the threshold if explicitly provided.
    #[must_use]
    pub fn from_workspace_with_threshold(root: &Path, explicit_threshold: Option<usize>) -> Self {
        let config = crate::config::MiterConfig::load_from_workspace(root);
        let effective_threshold = explicit_threshold.or(config.audit.threshold).unwrap_or(500);
        Self::new(effective_threshold)
    }

    /// Adds ignore patterns to the configuration.
    #[must_use]
    pub fn with_ignores(mut self, ignores: impl IntoIterator<Item = String>) -> Self {
        self.ignore_patterns.extend(ignores);
        self
    }
}

mod auditor;
mod nested;
mod walk;

pub use auditor::WorkspaceAuditor;
