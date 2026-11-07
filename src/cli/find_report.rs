//! Rendering for `miter find`, split from the search so neither file outgrows the line limit.

use std::collections::BTreeMap;

use super::enclosing::quote;
use super::find::Hit;

/// The rendered report.
pub(super) fn render_text(
    pattern: &str,
    hits: &[Hit<'_>],
    counts: &BTreeMap<String, usize>,
    truncated: bool,
    unreadable: usize,
) -> String {
    let files = hits
        .iter()
        .map(|hit| &hit.file)
        .collect::<std::collections::BTreeSet<_>>();
    let inside = hits.iter().filter(|hit| hit.enclosing.is_some()).count();
    let mut out = format!(
        "{pattern}: {} hit(s) in {} file(s), {inside} inside a declaration\n",
        hits.len(),
        files.len()
    );
    for hit in hits {
        let where_ = match hit.enclosing {
            Some(symbol) => format!("in {}", symbol.path),
            None => "(outside any declaration)".to_string(),
        };
        out.push_str(&format!(
            "  {}:{}:{}\n    {where_}\n    {}\n",
            hit.file.display(),
            hit.line,
            hit.column,
            hit.text
        ));
    }
    if !counts.is_empty() {
        out.push_str("  by crate: ");
        let rendered: Vec<String> = counts
            .iter()
            .map(|(name, count)| format!("{name}={count}"))
            .collect();
        out.push_str(&rendered.join(", "));
        out.push('\n');
    }
    if truncated {
        out.push_str("  stopped at the limit; raise --max to see the rest\n");
    }
    if unreadable > 0 {
        out.push_str(&format!(
            "  {unreadable} indexed file(s) could not be read\n"
        ));
    }
    out
}

/// The same payload as [`render_text`], for a caller that parses it.
pub(super) fn render_json(
    pattern: &str,
    hits: &[Hit<'_>],
    counts: &BTreeMap<String, usize>,
    truncated: bool,
    unreadable: usize,
) -> String {
    let rows: Vec<String> = hits
        .iter()
        .map(|hit| {
            let enclosing = match hit.enclosing {
                Some(symbol) => format!(
                    "{{\"name\":{},\"path\":{}}}",
                    quote(&symbol.name),
                    quote(&symbol.path)
                ),
                None => "null".to_string(),
            };
            format!(
                "{{\"file\":{},\"line\":{},\"column\":{},\"enclosing\":{},\"text\":{}}}",
                quote(&hit.file.display().to_string()),
                hit.line,
                hit.column,
                enclosing,
                quote(&hit.text)
            )
        })
        .collect();
    let crate_rows: Vec<String> = counts
        .iter()
        .map(|(name, count)| format!("{}:{count}", quote(name)))
        .collect();
    format!(
        "{{\"pattern\":{},\"total\":{},\"truncated\":{truncated},\"unreadable\":{unreadable},\"by_crate\":{{{}}},\"hits\":[{}]}}",
        quote(pattern),
        hits.len(),
        crate_rows.join(","),
        rows.join(",")
    )
}
