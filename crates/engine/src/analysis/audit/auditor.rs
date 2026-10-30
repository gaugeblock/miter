//! The workspace auditor: traversal, ignore handling and per-file measurement.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::nested::nested_repositories;
use super::walk::{count_declarations, is_ignored_dir, is_supported_source};
use super::{AuditConfig, AuditSeverity, AuditViolation};

/// Workspace line-count auditor.
#[derive(Clone, Debug, Default)]
pub struct WorkspaceAuditor {
    config: AuditConfig,
}

impl WorkspaceAuditor {
    /// Creates an auditor with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: AuditConfig::default(),
        }
    }

    /// Creates an auditor with a custom configuration.
    #[must_use]
    pub fn with_config(config: AuditConfig) -> Self {
        Self { config }
    }

    /// Recursively audits the given root directory and emits all violations.
    ///
    /// # Errors
    /// Returns an I/O error if directory traversal fails critically.
    pub fn audit_root(&self, root: &Path) -> Result<Vec<AuditViolation>, std::io::Error> {
        let mut violations = Vec::new();
        let mut ignored_paths = HashSet::new();
        self.load_gitignores(root, &mut ignored_paths);

        let miter_config = crate::config::MiterConfig::load_from_workspace(root);
        let glob_matcher = miter_config.build_audit_matcher().ok();

        let mut queue = vec![root.to_path_buf()];
        while let Some(dir) = queue.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if is_ignored_dir(file_name)
                    || self.is_ignored(&path, root, &ignored_paths, glob_matcher.as_ref())
                {
                    continue;
                }

                let Ok(file_type) = entry.file_type() else {
                    continue;
                };

                if file_type.is_dir() {
                    queue.push(path);
                } else if file_type.is_file()
                    && is_supported_source(&path)
                    && let Some(violation) = self.audit_file(&path)
                {
                    violations.push(violation);
                }
            }
        }

        violations.sort_by_key(|a| std::cmp::Reverse(a.line_count));
        Ok(violations)
    }

    /// Audits the given root directory and every immediately nested git repository.
    ///
    /// A meta-repository ignores its members' checkouts in `.gitignore`, so [`Self::audit_root`]
    /// alone reports a clean pass while the members hold the violations (`GB-194`). Members are
    /// audited with their own `.gitignore` and config, and the result is one ranked list.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if traversal of the root or of a member fails critically.
    pub fn audit_root_with_members(
        &self,
        root: &Path,
    ) -> Result<Vec<AuditViolation>, std::io::Error> {
        let mut violations = self.audit_root(root)?;
        for member in nested_repositories(root) {
            violations.extend(self.audit_root(&member)?);
        }
        violations.sort_by(|a, b| {
            b.line_count
                .cmp(&a.line_count)
                .then_with(|| a.path.cmp(&b.path))
        });
        violations.dedup_by(|a, b| a.path == b.path);
        Ok(violations)
    }

    /// Audits an individual file, returning an [`AuditViolation`] if `line_count >= max_lines`.
    pub fn audit_file(&self, path: &Path) -> Option<AuditViolation> {
        if crate::polyglot::env::is_env_file(path) {
            return None;
        }
        let content = fs::read_to_string(path).ok()?;
        let line_count = content.lines().count();
        if line_count < self.config.max_lines {
            return None;
        }

        let declaration_count = count_declarations(path, &content);
        let severity = if line_count >= 800 {
            AuditSeverity::Critical
        } else {
            AuditSeverity::Warning
        };

        Some(AuditViolation {
            path: path.to_path_buf(),
            line_count,
            declaration_count,
            severity,
        })
    }

    fn load_gitignores(&self, root: &Path, ignored: &mut HashSet<PathBuf>) {
        let gitignore_path = root.join(".gitignore");
        if let Ok(content) = fs::read_to_string(gitignore_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                    continue;
                }
                let clean = trimmed.trim_end_matches('/');
                ignored.insert(PathBuf::from(clean));
            }
        }
    }

    fn is_ignored(
        &self,
        path: &Path,
        root: &Path,
        gitignored: &HashSet<PathBuf>,
        glob_matcher: Option<&globset::GlobSet>,
    ) -> bool {
        if let Some(matcher) = glob_matcher
            && crate::config::is_path_excluded(matcher, path, root)
        {
            return true;
        }
        let path_str = path.to_string_lossy();
        self.config
            .ignore_patterns
            .iter()
            .any(|p| path_str.contains(p))
            || gitignored
                .iter()
                .any(|i| path_str.contains(i.to_string_lossy().as_ref()))
    }
}
