//! `miter audit --paths`: the 200-250 band may not grow (`GB-195`).
//!
//! The band is where the next 250-line breach comes from: 85 files on 2026-10-31, 131 on 2026-11-10, and every
//! extraction so far has moved files into it rather than out. In one session two files crossed 250 not by an
//! edit but by a reformat (`audit_multi_repo.rs` at 266, `handlers.rs` at 266), which is what "just under" means
//! in a repository with a hard ceiling and a formatter.
//!
//! A ratchet rather than a demand: 131 splits are not one change, and a gate that fails on day one teaches
//! people to pass a flag. So the current sizes are recorded, and the rule is that a file in the band may shrink
//! or leave it but never grow, and a new file may not enter it without the entry being added deliberately -
//! which makes the growth visible in review, in the diff, by name.
//!
//! Cleared entries are not removed automatically. The baseline tightens when someone regenerates it, so the
//! ratchet's history stays readable and a split shows up as a deletion.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use miter_engine::analysis::{AuditConfig, WorkspaceAuditor};

/// Where the band starts. Below the hard ceiling, so there is room to react.
pub(super) const BAND: usize = 200;

/// The recorded sizes, relative to the audited root, one `path<TAB>lines` per line.
pub(super) const BASELINE: &str = ".gaugeblock/miter/size-ratchet.txt";

/// A file in the band that grew, or entered without an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RatchetFinding {
    /// Path relative to the audited root.
    pub path: PathBuf,
    /// Recorded size, absent when the file is new to the band.
    pub was: Option<usize>,
    /// Current size.
    pub now: usize,
}

impl RatchetFinding {
    /// One line for a human, naming what to do.
    pub(super) fn describe(&self) -> String {
        let now = self.now;
        match self.was {
            Some(was) => format!(
                "{} grew {} -> {now} in the {BAND}-line band; split it or justify the growth",
                self.path.display(),
                was
            ),
            None => format!(
                "{} entered the {BAND}-line band at {now}; split it or record it deliberately",
                self.path.display()
            ),
        }
    }
}

/// The band as it is now: every file at or above [`BAND`], relative to `root`.
///
/// Deliberately the same walk as the line audit, at a lower threshold: a second walker would be a second answer
/// to "which files count", which is the duplication this repository has spent three tickets removing.
pub(super) fn band_files(root: &Path) -> Vec<(PathBuf, usize)> {
    let config = AuditConfig::from_workspace_with_threshold(root, Some(BAND));
    let auditor = WorkspaceAuditor::with_config(config);
    let Ok(violations) = auditor.audit_root_with_members(root) else {
        return Vec::new();
    };
    let mut files: Vec<(PathBuf, usize)> = violations
        .into_iter()
        .map(|violation| (relative(root, &violation.path), violation.line_count))
        .filter(|(_, lines)| *lines >= BAND)
        .collect();
    files.sort();
    files
}

/// Reads the recorded sizes. A missing file is not an error: the ratchet is adopted by writing it once.
pub(super) fn load(root: &Path) -> BTreeMap<PathBuf, usize> {
    let path = root.join(BASELINE);
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (path, lines) = line.split_once('\t')?;
            Some((PathBuf::from(path), lines.trim().parse().ok()?))
        })
        .collect()
}

/// Writes the band as it stands, so the ratchet's baseline is a reviewable diff rather than a number in prose.
///
/// # Errors
///
/// The I/O error from creating the directory or writing the file.
pub(super) fn write(root: &Path, files: &[(PathBuf, usize)]) -> std::io::Result<()> {
    let path = root.join(BASELINE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = String::from(
        "# Files in the 200-250 band, with their size when the ratchet was last set (GB-195).\n\
         # A file here may shrink or leave the band; it may not grow, and a new entry must be added\n\
         # deliberately. Regenerate with: miter audit --paths --write-size-ratchet\n",
    );
    for (file, lines) in files {
        text.push_str(&format!("{}\t{lines}\n", file.display()));
    }
    std::fs::write(path, text)
}

/// Compares the band against what was recorded.
pub(super) fn findings(
    recorded: &BTreeMap<PathBuf, usize>,
    current: &[(PathBuf, usize)],
) -> Vec<RatchetFinding> {
    let mut findings = Vec::new();
    for (path, lines) in current {
        match recorded.get(path) {
            Some(was) if lines > was => findings.push(RatchetFinding {
                path: path.clone(),
                was: Some(*was),
                now: *lines,
            }),
            Some(_) => {}
            None => findings.push(RatchetFinding {
                path: path.clone(),
                was: None,
                now: *lines,
            }),
        }
    }
    findings
}

/// `path` relative to `root`, so a baseline is portable across checkouts.
fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn entry(path: &str, lines: usize) -> (PathBuf, usize) {
        (PathBuf::from(path), lines)
    }

    #[test]
    fn a_file_that_shrank_or_left_the_band_is_not_a_finding() {
        let mut recorded = BTreeMap::new();
        recorded.insert(PathBuf::from("a.rs"), 240);
        assert!(findings(&recorded, &[entry("a.rs", 210)]).is_empty());
        assert!(findings(&recorded, &[]).is_empty());
    }

    #[test]
    fn growth_by_one_line_is_still_growth() {
        let mut recorded = BTreeMap::new();
        recorded.insert(PathBuf::from("a.rs"), 249);
        let findings = findings(&recorded, &[entry("a.rs", 250)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].was, Some(249));
        assert_eq!(findings[0].now, 250);
        assert!(findings[0].describe().contains("grew 249 -> 250"));
    }

    #[test]
    fn a_new_file_in_the_band_needs_a_deliberate_entry() {
        let findings = findings(&BTreeMap::new(), &[entry("new.rs", 201)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].was, None);
        assert!(findings[0].describe().contains("entered"));
    }

    #[test]
    fn a_line_in_the_baseline_parses_and_a_comment_does_not() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".gaugeblock/miter")).unwrap();
        std::fs::write(
            tmp.path().join(BASELINE),
            "# a comment\ncrates/a.rs\t231\n\ncrates/b.ts\t205\n",
        )
        .unwrap();
        let recorded = load(tmp.path());
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded.get(Path::new("crates/a.rs")), Some(&231));
    }

    #[test]
    fn a_missing_baseline_is_empty_rather_than_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load(tmp.path()).is_empty());
    }

    #[test]
    fn writing_then_reading_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let files = vec![entry("crates/b.rs", 210), entry("crates/a.rs", 249)];
        write(tmp.path(), &files).unwrap();
        let recorded = load(tmp.path());
        assert_eq!(recorded.get(Path::new("crates/a.rs")), Some(&249));
        assert_eq!(recorded.get(Path::new("crates/b.rs")), Some(&210));
        assert!(findings(&recorded, &files).is_empty());
    }
}
