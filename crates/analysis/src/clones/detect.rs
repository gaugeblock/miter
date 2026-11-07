//! Clone detection: which pairs are reported, and on what evidence (`GB-197`).
//!
//! Split from extraction when the file passed the 250-line limit. Extraction asks "what is this
//! function's shape and what does it name"; detection asks "do these two agree closely enough to be
//! worth a reviewer's time", and only the second question grows as thresholds are tuned.

use super::*;

/// Jaccard overlap of the two identifier sets, in [0.0, 1.0].
///
/// Both sides are already deduplicated, so a body naming the same handful of things twice scores the
/// same as one naming them once. That is intended: the question is which things two bodies talk
/// about, not how often.
#[must_use]
pub fn content_overlap(first: &[String], second: &[String]) -> f32 {
    if first.is_empty() || second.is_empty() {
        // No names on one side means nothing to compare, not 'identical'.
        return 0.0;
    }
    let shared = first.iter().filter(|name| second.contains(name)).count();
    let union = first.len() + second.len() - shared;
    if union == 0 {
        0.0
    } else {
        shared as f32 / union as f32
    }
}

/// Analyzes fingerprints and finds structural clone pairs across different files.
#[must_use]
pub fn find_clones(
    fingerprints: &[FunctionFingerprint],
    min_similarity: f32,
    min_content: f32,
) -> CloneDetectionReport {
    let mut clone_pairs = Vec::new();
    let max_allowed_distance = ((1.0 - min_similarity) * 128.0).floor() as u32;

    for i in 0..fingerprints.len() {
        for j in (i + 1)..fingerprints.len() {
            let f1 = &fingerprints[i];
            let f2 = &fingerprints[j];

            // Only compare across different files
            if f1.file_path == f2.file_path {
                continue;
            }

            // A SimHash fingerprint is scale-invariant: structural shape alone cannot tell a
            // 22-line helper from a 96-line orchestrator. Require the shorter body to be at least
            // `MIN_SIZE_RATIO` of the longer one, so only pairs a reviewer would call a clone survive.
            let (short, long) = if f1.line_count <= f2.line_count {
                (f1.line_count, f2.line_count)
            } else {
                (f2.line_count, f1.line_count)
            };
            if long > 0 && (short as f32) < (long as f32) * MIN_SIZE_RATIO {
                continue;
            }

            // Shape alone is not evidence: two functions built from the same control-flow skeleton
            // can share a silhouette while doing unrelated work. Both dimensions must agree before a
            // pair is reported, which is what removes the 85 percent nonsense pairs (`GB-197`).
            if content_overlap(&f1.identifiers, &f2.identifiers) < min_content {
                continue;
            }

            let dist = hamming_distance(f1.simhash, f2.simhash);
            if dist <= max_allowed_distance {
                let similarity = similarity_score(dist);
                clone_pairs.push(StructuralClonePair {
                    first: f1.clone(),
                    second: f2.clone(),
                    similarity,
                    hamming_distance: dist,
                });
            }
        }
    }

    // INVARIANT: similarity is clamped to 0.0..=1.0, so `partial_cmp` never sees a NaN.
    clone_pairs.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    CloneDetectionReport {
        functions_analyzed: fingerprints.len(),
        clone_pairs,
    }
}
