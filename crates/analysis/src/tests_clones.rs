//! Unit tests for Deterministic AST Structural Clone Detection.

use super::clones::{
    extract_fingerprints_from_rust, find_clones, hamming_distance, similarity_score,
};
use std::path::Path;

#[test]
fn test_hamming_distance_and_similarity() {
    let fp1 = (0u64, 0u64);
    let fp2 = (0u64, 0u64);
    assert_eq!(hamming_distance(fp1, fp2), 0);
    assert!((similarity_score(0) - 1.0).abs() < 1e-6);

    let fp3 = (0u64, 0b1111u64);
    assert_eq!(hamming_distance(fp1, fp3), 4);
    let sim = similarity_score(4);
    assert!((sim - (124.0 / 128.0)).abs() < 1e-6);
}

#[test]
fn test_detect_structural_clones_across_files() {
    let func_a = r#"
fn process_items_v1(items: &[String]) -> Result<usize, String> {
    let mut count = 0;
    for item in items {
        if item.is_empty() {
            continue;
        }
        match item.as_str() {
            "skip" => continue,
            "stop" => break,
            _ => count += 1,
        }
    }
    if count == 0 {
        Err("no valid items".to_string())
    } else {
        Ok(count)
    }
}
"#;

    let func_b = r#"
fn handle_records_v2(records: &[Record]) -> Result<usize, CustomError> {
    let mut total = 0;
    for record in records {
        if record.is_invalid() {
            continue;
        }
        match record.status() {
            Status::Skip => continue,
            Status::Halt => break,
            _ => total += 1,
        }
    }
    if total == 0 {
        Err(CustomError::Empty)
    } else {
        Ok(total)
    }
}
"#;

    let fps_a = extract_fingerprints_from_rust(Path::new("src/a.rs"), func_a, 10);
    let fps_b = extract_fingerprints_from_rust(Path::new("src/b.rs"), func_b, 10);

    assert_eq!(fps_a.len(), 1, "expected 1 fingerprint from file A");
    assert_eq!(fps_b.len(), 1, "expected 1 fingerprint from file B");

    let all_fps = vec![fps_a[0].clone(), fps_b[0].clone()];
    // Content similarity is disabled here on purpose: this fixture exists to prove that structural
    // matching works at all, and its two functions name different things. The content dimension has its
    // own test below (`GB-197`).
    let report = find_clones(&all_fps, 0.90, 0.0);

    assert_eq!(report.functions_analyzed, 2);
    assert_eq!(
        report.clone_pairs.len(),
        1,
        "expected clone pair with >= 90% structural similarity"
    );

    let pair = &report.clone_pairs[0];
    assert!(
        pair.similarity >= 0.90,
        "actual similarity: {}",
        pair.similarity
    );
    assert_eq!(pair.first.file_path, "src/a.rs");
    assert_eq!(pair.second.file_path, "src/b.rs");
}

#[test]
fn shape_without_shared_content_is_not_a_clone() {
    // `GB-197`: two bodies with the same control-flow skeleton but unrelated contents were reported as
    // duplication. `dado`'s `main` matched `outfeed`'s `parse_header` at 85 percent, which no reviewer
    // would act on. Both must agree now.
    let first = r#"
fn first(a: i32) -> i32 {
    if a > 0 {
        let x = a * 2;
        let y = x + 1;
        let z = y * 3;
        z - a
    } else {
        let x = a * 2;
        let y = x + 1;
        let z = y * 3;
        z + a
    }
}
"#;
    let second = r#"
fn second(b: i32) -> i32 {
    if b > 0 {
        let p = b * 2;
        let q = p + 1;
        let r = q * 3;
        r - b
    } else {
        let p = b * 2;
        let q = p + 1;
        let r = q * 3;
        r + b
    }
}
"#;
    let a = extract_fingerprints_from_rust(Path::new("src/one.rs"), first, 5);
    let b = extract_fingerprints_from_rust(Path::new("src/two.rs"), second, 5);
    assert_eq!((a.len(), b.len()), (1, 1));

    let structural = find_clones(&[a[0].clone(), b[0].clone()], 0.50, 0.0);
    assert_eq!(structural.clone_pairs.len(), 1, "shape alone still matches");

    let with_content = find_clones(&[a[0].clone(), b[0].clone()], 0.50, 0.90);
    assert!(
        with_content.clone_pairs.is_empty(),
        "different identifiers must not be reported as duplication"
    );
}
