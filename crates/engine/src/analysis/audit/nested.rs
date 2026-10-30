//! Nested-repository discovery for meta-repositories.
//!
//! A meta-repository normally ignores its members' working trees, because each member is its own
//! git checkout (`/common/`, `/miter/`, ... in the meta `.gitignore`). A plain walk from the meta
//! root therefore honours `.gitignore` and audits nothing at all - `GB-194`. The members are still
//! part of the suite, so they are discovered here and audited individually, each with its own
//! `.gitignore` and miter config.

use std::fs;
use std::path::{Path, PathBuf};

use super::walk::is_ignored_dir;

/// Immediate subdirectories of `root` that are their own git checkout, sorted by path.
#[must_use]
pub(super) fn nested_repositories(root: &Path) -> Vec<PathBuf> {
    let mut repositories = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return repositories;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if is_ignored_dir(name) {
            continue;
        }
        if path.join(".git").exists() {
            repositories.push(path);
        }
    }

    repositories.sort();
    repositories
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::nested_repositories;
    use std::fs;

    #[test]
    fn test_nested_repositories_are_discovered_and_sorted() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("miter-nested-{stamp}"));

        for member in ["member-b", "member-a"] {
            let dir = base.join(member);
            fs::create_dir_all(dir.join(".git")).expect("member");
        }
        fs::create_dir_all(base.join("plain-dir")).expect("plain dir");
        fs::create_dir_all(base.join(".hidden").join(".git")).expect("hidden");
        fs::create_dir_all(base.join("target").join(".git")).expect("build dir");

        let found = nested_repositories(&base);

        assert_eq!(
            found,
            vec![base.join("member-a"), base.join("member-b")],
            "only immediate git checkouts count, in path order"
        );

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn test_a_plain_directory_has_no_nested_repositories() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("miter-plain-{stamp}"));
        fs::create_dir_all(base.join("crates")).expect("crates");

        assert!(nested_repositories(&base).is_empty());

        fs::remove_dir_all(&base).expect("cleanup");
    }
}
