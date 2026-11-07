//! Pruning and adopting allowlist entries (`GB-248`).
//!
//! Deliberately textual rather than a TOML round-trip: the allowlist opens with a comment explaining what an
//! entry means and how one leaves the file, and re-serializing would delete it. That prose is part of the
//! artifact - an entry without its explanation is the suppression this file exists to prevent.

use super::error::CliError;
use miter_engine::analysis::LayerReport;
use std::path::Path;

/// Applies `--prune` and `--adopt`, writing only when something changed.
pub(super) fn apply_edits(
    layers: &LayerReport,
    path: &Path,
    edit: &super::deps_layers::AllowlistEdit<'_>,
) -> Result<(), CliError> {
    if !edit.prune && !edit.adopt {
        return Ok(());
    }
    if edit.adopt && edit.ticket.is_none() {
        return Err(CliError::Usage(
            "--adopt needs --ticket: an exception without an owner is a suppression".to_string(),
        ));
    }

    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let (header, mut blocks) = split_blocks(&existing);

    if edit.prune {
        blocks.retain(|block| {
            let (from, to) = block_names(block);
            !layers
                .stale
                .iter()
                .any(|entry| entry.from == from && entry.to == to)
        });
    }

    if edit.adopt {
        let ticket = edit.ticket.unwrap_or_default();
        let adopted: Vec<String> = layers
            .unallowlisted_lateral()
            .map(|edge| {
                format!(
                    "[[lateral]]\nfrom = \"{}\"\nto = \"{}\"\nticket = \"{ticket}\"\n",
                    edge.from, edge.to
                )
            })
            .collect();
        blocks.extend(adopted);
    }

    let rendered = render(&header, &blocks);
    if rendered != existing {
        std::fs::write(path, rendered)
            .map_err(|error| CliError::Usage(format!("{}: {error}", path.display())))?;
    }
    Ok(())
}

/// Splits the file into its leading comment and one block per `[[lateral]]` table.
///
/// The comment is kept separately because it is not a block: folding it into the first block would rewrite
/// the file's own instructions the first time an entry was pruned.
fn split_blocks(text: &str) -> (String, Vec<String>) {
    let Some(first) = text.find("[[lateral]]") else {
        return (text.to_string(), Vec::new());
    };
    let header = text[..first].to_string();
    let blocks = text[first..]
        .split("[[lateral]]")
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| format!("[[lateral]]{}", chunk.trim_end()))
        .collect();
    (header, blocks)
}

/// `(from, to)` from a block's text, empty strings when a key is absent.
fn block_names(block: &str) -> (String, String) {
    let value = |key: &str| {
        block
            .lines()
            .find_map(|line| line.trim().strip_prefix(key)?.trim().strip_prefix('='))
            .map(|rest| rest.trim().trim_matches('"').to_string())
            .unwrap_or_default()
    };
    (value("from"), value("to"))
}

/// Reassembles the header and blocks, one blank line between tables.
fn render(header: &str, blocks: &[String]) -> String {
    if blocks.is_empty() {
        return header.trim_end().to_string() + "\n";
    }
    let body = blocks
        .iter()
        .map(|block| block.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("{}\n{}\n", header.trim_end(), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "# An entry names the ticket that owes the burn-down.\n#\n";

    #[test]
    fn the_header_comment_survives_pruning() {
        let text = format!("{HEADER}[[lateral]]\nfrom = \"a\"\nto = \"b\"\nticket = \"GB-1\"\n");
        let (header, blocks) = split_blocks(&text);
        assert!(header.contains("owes the burn-down"));
        assert_eq!(blocks.len(), 1);
        assert_eq!(render(&header, &[]), HEADER.to_string());
    }

    #[test]
    fn a_block_reports_its_two_crate_names() {
        let block =
            "[[lateral]]\nfrom = \"dado-core\"\nto = \"miter-polyglot\"\nticket = \"GB-204\"";
        assert_eq!(
            block_names(block),
            ("dado-core".to_string(), "miter-polyglot".to_string())
        );
    }

    #[test]
    fn a_file_with_no_entries_round_trips_its_comment() {
        let (header, blocks) = split_blocks(HEADER);
        assert!(blocks.is_empty());
        assert_eq!(render(&header, &blocks), HEADER);
    }
}
