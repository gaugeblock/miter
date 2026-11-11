//! Tests for the state-deletion gate (`MTR-1`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use super::*;

#[test]
fn a_decision_citation_is_recognised_in_the_spellings_the_suite_uses() {
    assert!(cites_decision(
        "chore: retire the ledger\n\nADR-0009 accepted."
    ));
    assert!(cites_decision("supersedes adr 12"));
    assert!(cites_decision("see docs/decisions/0014-state.md"));
}

#[test]
fn a_deletion_without_one_is_not() {
    assert!(!cites_decision(
        "chore(storypole): retire the legacy repo-local ledger"
    ));
    assert!(!cites_decision("adr without a number"));
    assert!(!cites_decision(""));
}

#[test]
fn a_commit_that_deletes_state_on_a_branch_is_reported() {
    let root = scratch("reported");
    repo(&root);

    std::fs::create_dir_all(root.join(".gaugeblock").join("storypole")).expect("dir");
    std::fs::write(
        root.join(".gaugeblock")
            .join("storypole")
            .join("ledger.jsonl"),
        "x",
    )
    .expect("write");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "chore: add state"]);

    git(&root, &["checkout", "-q", "-b", "topic"]);
    git(&root, &["rm", "-q", ".gaugeblock/storypole/ledger.jsonl"]);
    git(&root, &["commit", "-q", "-m", "chore: retire the ledger"]);

    let found = deletions_since(&root, "main").expect("scan");
    assert_eq!(found.len(), 1, "the branch deletion is reported");
    assert_eq!(found[0].subject, "chore: retire the ledger");
    assert!(found[0].files.iter().any(|f| f.contains("ledger.jsonl")));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_same_deletion_with_a_citation_passes() {
    let root = scratch("cited");
    repo(&root);

    std::fs::create_dir_all(root.join(".gaugeblock")).expect("dir");
    std::fs::write(root.join(".gaugeblock").join("state.toml"), "x").expect("write");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "chore: add state"]);

    git(&root, &["checkout", "-q", "-b", "topic"]);
    git(&root, &["rm", "-q", ".gaugeblock/state.toml"]);
    git(
        &root,
        &[
            "commit",
            "-q",
            "-m",
            "chore: retire state\n\nADR-0009 supersedes it.",
        ],
    );

    assert!(
        deletions_since(&root, "main").expect("scan").is_empty(),
        "a cited deletion is not a finding"
    );

    std::fs::remove_dir_all(&root).ok();
}

/// A scratch directory under the system temp dir, named so two tests in one process cannot collide.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miter-audit-state-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A throwaway repository with an identity, so commits work on a machine with no global config.
fn repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "t@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git spawns");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
