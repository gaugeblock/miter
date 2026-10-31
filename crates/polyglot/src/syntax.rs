//! Syntax diagnostics: what a grammar rejects, in spans a caller can act on.
//!
//! [`super::parse`] answers "which declarations are here"; this module answers "what is malformed".
//! Both read the one grammar table ([`miter_ast::grammar_for`]), so a caller that validates code
//! before splicing it sees exactly the grammar the indexer and the universal reader use.
//!
//! The snippet is sliced on character boundaries: a byte range from `tree-sitter` can land inside a
//! multi-byte character, and a diagnostic must never panic on the code it is describing.

use super::{Language, PolyglotError};

/// Why a node was reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxDiagnosticKind {
    /// `tree-sitter` produced an `ERROR` node for this span.
    Error,
    /// A node the grammar required is absent; the payload names what it expected.
    Missing {
        /// The node kind `tree-sitter` inserted in place of the missing syntax.
        expected: String,
    },
}

/// One malformed span, positioned for a human and addressable by byte for a fixer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    /// What went wrong.
    pub kind: SyntaxDiagnosticKind,
    /// One-based line of the span start.
    pub line: usize,
    /// One-based column of the span start.
    pub column: usize,
    /// Inclusive start byte.
    pub start_byte: usize,
    /// Exclusive end byte.
    pub end_byte: usize,
    /// Source text of the span, whitespace-collapsed and truncated.
    pub snippet: String,
}

/// Parses `source` and returns every `ERROR` and missing node, in tree order.
///
/// An empty vector means the grammar accepted the source.
///
/// # Errors
///
/// As [`super::parse`]: the language must have a tree-sitter grammar and the parser must return a
/// tree. Declarative formats are rejected by [`super::parse_declarative`] instead.
pub fn syntax_diagnostics(
    source: &str,
    language: Language,
) -> Result<Vec<SyntaxDiagnostic>, PolyglotError> {
    let mut parser = parser_for(language)?;
    let tree = parser.parse(source, None).ok_or(PolyglotError::Parse {
        language,
        path: None,
    })?;

    let mut diagnostics = Vec::new();
    collect(tree.root_node(), source, &mut diagnostics);
    Ok(diagnostics)
}

/// `true` when `start_byte..end_byte` falls inside a single syntax node.
///
/// A hole in a template may only replace a whole node; a span that straddles two nodes cannot be
/// substituted without producing code no grammar accepts.
///
/// # Errors
///
/// As [`syntax_diagnostics`].
pub fn span_within_node(
    source: &str,
    language: Language,
    start_byte: usize,
    end_byte: usize,
) -> Result<bool, PolyglotError> {
    let mut parser = parser_for(language)?;
    let tree = parser.parse(source, None).ok_or(PolyglotError::Parse {
        language,
        path: None,
    })?;

    let root = tree.root_node();
    Ok(root
        .descendant_for_byte_range(start_byte, end_byte)
        .is_some())
}

/// Builds a parser for `language` from the workspace grammar table.
pub(super) fn parser_for(language: Language) -> Result<tree_sitter::Parser, PolyglotError> {
    let Some(index_language) = language.index_language() else {
        return Err(PolyglotError::Unsupported { language });
    };
    let grammar = miter_ast::grammar_for(index_language).map_err(|error| match error {
        miter_ast::IndexError::UnsupportedLanguage { .. } => {
            PolyglotError::Unsupported { language }
        }
        other => PolyglotError::Grammar {
            language,
            message: other.to_string(),
        },
    })?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|error| PolyglotError::Grammar {
            language,
            message: error.to_string(),
        })?;
    Ok(parser)
}

/// Walks the tree collecting malformed nodes.
fn collect(node: tree_sitter::Node<'_>, source: &str, diagnostics: &mut Vec<SyntaxDiagnostic>) {
    let kind = if node.is_error() {
        Some(SyntaxDiagnosticKind::Error)
    } else if node.is_missing() {
        Some(SyntaxDiagnosticKind::Missing {
            expected: node.kind().to_string(),
        })
    } else {
        None
    };

    if let Some(kind) = kind {
        let start = node.start_position();
        diagnostics.push(SyntaxDiagnostic {
            kind,
            line: start.row + 1,
            column: start.column + 1,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            snippet: snippet(source, node.start_byte(), node.end_byte()),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, source, diagnostics);
    }
}

/// Collapses and truncates the span text without splitting a character.
fn snippet(source: &str, start_byte: usize, end_byte: usize) -> String {
    let end = end_byte.min(source.len());
    if start_byte >= end {
        return String::new();
    }
    let slice = &source[start_byte..end];
    let collapsed = slice.replace('\n', " ");
    if collapsed.chars().count() <= 40 {
        return collapsed;
    }
    let truncated: String = collapsed.chars().take(40).collect();
    format!("{truncated}...")
}
