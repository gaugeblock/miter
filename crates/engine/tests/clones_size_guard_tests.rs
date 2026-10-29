//! The clone detector must not pair functions whose bodies differ wildly in size (GB-154).
//!
//! A SimHash fingerprint is scale-invariant, so before the guard a 22-line helper matched a 96-line
//! orchestrator at 90.6%, which no reviewer would act on.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use miter_engine::{FunctionFingerprint, find_clones};

fn fingerprint(file: &str, name: &str, lines: usize, simhash: (u64, u64)) -> FunctionFingerprint {
    FunctionFingerprint {
        file_path: file.to_string(),
        name: name.to_string(),
        line_span: (1, lines),
        line_count: lines,
        simhash,
    }
}

#[test]
fn test_pairs_with_matching_shape_and_size_are_reported() {
    let prints = vec![
        fingerprint("src/a.rs", "alpha", 30, (0xAAAA, 0xBBBB)),
        fingerprint("src/b.rs", "beta", 34, (0xAAAA, 0xBBBB)),
    ];
    let report = find_clones(&prints, 0.90);
    assert_eq!(
        report.clone_pairs.len(),
        1,
        "a 30 vs 34 line pair is a real clone"
    );
}

#[test]
fn test_pairs_with_divergent_sizes_are_dropped() {
    let prints = vec![
        fingerprint("src/a.rs", "short_helper", 22, (0xAAAA, 0xBBBB)),
        fingerprint("src/b.rs", "long_orchestrator", 96, (0xAAAA, 0xBBBB)),
    ];
    let report = find_clones(&prints, 0.90);
    assert!(
        report.clone_pairs.is_empty(),
        "a 22 vs 96 line pair must not be reported, got {:?}",
        report.clone_pairs
    );
}

#[test]
fn test_identical_fingerprints_still_require_similar_size() {
    // Same file is never compared; different files with equal bodies are.
    let prints = vec![
        fingerprint("src/a.rs", "one", 40, (1, 2)),
        fingerprint("src/b.rs", "two", 41, (1, 2)),
        fingerprint("src/c.rs", "three", 200, (1, 2)),
    ];
    let report = find_clones(&prints, 0.90);
    assert_eq!(
        report.clone_pairs.len(),
        1,
        "only the 40 vs 41 pair survives"
    );
    let pair = &report.clone_pairs[0];
    assert_eq!(pair.first.name, "one");
    assert_eq!(pair.second.name, "two");
}
