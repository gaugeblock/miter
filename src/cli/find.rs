//! `miter find`: locate a literal and name the declarations it lands in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use miter_engine::polyglot::UniversalSymbol;

use super::CliError;
use super::enclosing::{byte_offset_of_line, innermost, line_of_offset};

/// One located literal, with the declaration containing it when there is one.
pub(super) struct Hit<'a> {
    pub(super) file: PathBuf,
    pub(super) line: usize,
    pub(super) column: usize,
    pub(super) text: String,
    pub(super) enclosing: Option<&'a UniversalSymbol>,
}

/// `miter find <PATTERN> [--ignore-case] [--max <N>] [--json] [--db <PATH>]`.
///
/// Searches the files the index knows about and reports each hit with its enclosing declaration, so a
/// caller can act on the result without a second pass. Hits outside any declaration are reported as
/// such rather than dropped: a literal in a `mod.rs` re-export list is still a hit, and hiding it would
/// make the count disagree with what the reader finds.
pub(super) fn run_find(
    pattern: &str,
    db: &Path,
    ignore_case: bool,
    limit: usize,
    json: bool,
) -> Result<(), CliError> {
    if pattern.is_empty() {
        return Err(CliError::Usage("search pattern is empty".to_string()));
    }
    let engine = super::store::load(db)?;

    let mut by_file: BTreeMap<PathBuf, Vec<&UniversalSymbol>> = BTreeMap::new();
    for symbol in engine.universal_symbols() {
        by_file.entry(symbol.file.clone()).or_default().push(symbol);
    }

    let mut hits: Vec<Hit> = Vec::new();
    let mut truncated = false;
    let mut unreadable = 0usize;
    for (file, symbols) in &by_file {
        let Ok(source) = std::fs::read_to_string(file) else {
            // A file in the index that is gone or is not UTF-8 is counted, not silently ignored.
            unreadable += 1;
            continue;
        };
        for offset in offsets_of(&source, pattern, ignore_case) {
            if hits.len() >= limit {
                truncated = true;
                break;
            }
            let line = line_of_offset(&source, offset);
            let column = offset - byte_offset_of_line(&source, line) + 1;
            hits.push(Hit {
                file: file.clone(),
                line,
                column,
                text: line_text(&source, line),
                enclosing: innermost(symbols, offset),
            });
        }
        if truncated {
            break;
        }
    }

    let counts = crate_counts(&hits);
    if json {
        println!(
            "{}",
            super::find_report::render_json(pattern, &hits, &counts, truncated, unreadable)
        );
    } else {
        print!(
            "{}",
            super::find_report::render_text(pattern, &hits, &counts, truncated, unreadable)
        );
    }
    Ok(())
}

/// Byte offsets of every occurrence of `needle`.
///
/// Case-insensitive matching lowercases bytewise (`to_ascii_lowercase`), which preserves string length.
/// General Unicode case folding does not, and an offset into a differently-sized string would attribute
/// hits to the wrong declaration - confidently wrong, which is worse than refusing.
fn offsets_of(source: &str, needle: &str, ignore_case: bool) -> Vec<usize> {
    if ignore_case {
        let folded_needle = needle.to_ascii_lowercase();
        source
            .to_ascii_lowercase()
            .match_indices(&folded_needle)
            .map(|(offset, _)| offset)
            .collect()
    } else {
        source
            .match_indices(needle)
            .map(|(offset, _)| offset)
            .collect()
    }
}

/// The source line, trimmed, for display beside a hit.
fn line_text(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .chars()
        .take(160)
        .collect()
}

/// Hits per crate, derived from the path so the summary needs no manifest read.
fn crate_counts(hits: &[Hit<'_>]) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for hit in hits {
        *counts.entry(crate_of(&hit.file)).or_default() += 1;
    }
    counts
}

/// The crate a file belongs to: the component after `crates/`, else the first component.
fn crate_of(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_string())
        .collect();
    if let Some(index) = parts.iter().position(|part| part == "crates")
        && let Some(name) = parts.get(index + 1)
    {
        return name.clone();
    }
    parts
        .first()
        .cloned()
        .unwrap_or_else(|| "(root)".to_string())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_offsets_land_where_match_indices_says() {
        let source = "alpha beta alpha";
        assert_eq!(offsets_of(source, "alpha", false), vec![0, 11]);
        assert_eq!(offsets_of(source, "missing", false), Vec::<usize>::new());
    }

    #[test]
    fn ascii_case_folding_preserves_offsets() {
        let source = "CamelCase camelcase";
        let offsets = offsets_of(source, "CAMELCASE", true);
        assert_eq!(
            offsets,
            vec![0, 10],
            "offsets must index the original source"
        );
        assert_eq!(source.as_bytes()[offsets[1]], b'c');
    }

    #[test]
    fn crate_is_the_component_after_crates() {
        assert_eq!(
            crate_of(Path::new("common/crates/gaugeblock-workspace/src/a.rs")),
            "gaugeblock-workspace"
        );
        assert_eq!(crate_of(Path::new("miter/src/lib.rs")), "miter");
        assert_eq!(crate_of(Path::new("top.rs")), "top.rs");
    }
}
