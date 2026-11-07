//! `miter enclosing`: map a source position to the declaration that contains it.

use std::path::{Path, PathBuf};

use miter_engine::polyglot::UniversalSymbol;

use super::CliError;

/// `miter enclosing <FILE>:<LINE> [--json] [--db <PATH>]`.
///
/// The bridge from a compiler error or a grep hit to the symbol around it. Every indexed
/// declaration already carries a byte `span`, so this needs no end line of its own: the span is
/// converted to a line range using the file's current contents.
///
/// The file is read rather than trusted from the index so that a position resolves against the
/// source the caller is looking at. A stale index and an edited file disagree, and the honest
/// outcome then is "no declaration contains this line", not a plausible near-miss.
pub(super) fn run_enclosing(target: &str, db: &Path, json: bool) -> Result<(), CliError> {
    let (file_text, line) = parse_target(target)?;
    let path = PathBuf::from(&file_text);
    let source = std::fs::read_to_string(&path).map_err(CliError::Io)?;

    let line_count = source.lines().count();
    if line > line_count {
        return Err(CliError::Usage(format!(
            "line {line} is past the end of {} which has {line_count} lines",
            path.display()
        )));
    }

    let offset = byte_offset_of_line(&source, line);
    let engine = super::store::load(db)?;
    let candidates: Vec<&UniversalSymbol> = engine
        .universal_symbols()
        .iter()
        .filter(|symbol| same_file(&symbol.file, &path))
        .collect();

    let enclosing = innermost(&candidates, offset);
    if json {
        println!("{}", render_json(path.as_path(), line, enclosing, &source));
    } else {
        print!("{}", render_text(path.as_path(), line, enclosing, &source));
    }
    Ok(())
}

/// Splits `<file>:<line>`, at the last colon so a path containing one cannot confuse the parse.
fn parse_target(target: &str) -> Result<(String, usize), CliError> {
    let (file, line) = target
        .rsplit_once(':')
        .ok_or_else(|| CliError::Usage(format!("expected <file>:<line>, got `{target}`")))?;
    let line: usize = line
        .trim()
        .parse()
        .map_err(|_| CliError::Usage(format!("`{line}` is not a line number")))?;
    if file.is_empty() || line == 0 {
        // Line numbers are one-based everywhere the caller could have got one from.
        return Err(CliError::Usage(format!(
            "expected <file>:<line>, got `{target}`"
        )));
    }
    Ok((file.to_string(), line))
}

/// Byte offset of the first character of a one-based line.
pub(super) fn byte_offset_of_line(source: &str, line: usize) -> usize {
    let mut offset = 0;
    for (index, text) in source.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            break;
        }
        offset += text.len();
    }
    offset
}

/// One-based line containing a byte offset.
pub(super) fn line_of_offset(source: &str, offset: usize) -> usize {
    let capped = offset.min(source.len());
    1 + source[..capped]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
}

/// The narrowest declaration whose span contains `offset`.
///
/// Narrowest rather than first: a method sits inside its type's span, and the useful answer for a
/// position inside the method is the method.
pub(super) fn innermost<'a>(
    symbols: &[&'a UniversalSymbol],
    offset: usize,
) -> Option<&'a UniversalSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.span.0 <= offset && offset < symbol.span.1)
        .min_by_key(|symbol| symbol.span.1 - symbol.span.0)
        .copied()
}

/// Whether an indexed path names the file the caller typed.
fn same_file(indexed: &Path, wanted: &Path) -> bool {
    if indexed == wanted {
        return true;
    }
    let resolve = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if resolve(indexed) == resolve(wanted) {
        return true;
    }
    indexed.ends_with(wanted) || wanted.ends_with(indexed)
}
/// The rendered declaration, or a statement that no declaration contains the position.
fn render_text(path: &Path, line: usize, symbol: Option<&UniversalSymbol>, source: &str) -> String {
    let Some(symbol) = symbol else {
        return format!(
            "{}:{line}\n  no declaration contains this line\n",
            path.display()
        );
    };
    format!(
        "{}:{line}\n  {}\n  {} · {:?} · level {}\n  {}:{}-{}\n  {}\n",
        path.display(),
        symbol.path,
        symbol.name,
        symbol.kind,
        symbol.depth,
        path.display(),
        symbol.line,
        line_of_offset(source, symbol.span.1),
        symbol.signature,
    )
}

/// The same payload as [`render_text`], for a caller that parses it.
fn render_json(path: &Path, line: usize, symbol: Option<&UniversalSymbol>, source: &str) -> String {
    let Some(symbol) = symbol else {
        return format!(
            "{{\"file\":{},\"line\":{line},\"enclosing\":null}}",
            quote(&path.display().to_string())
        );
    };
    format!(
        "{{\"file\":{},\"line\":{line},\"enclosing\":{{\"name\":{},\"path\":{},\"kind\":{},\"depth\":{},\"start_line\":{},\"end_line\":{},\"signature\":{}}}}}",
        quote(&path.display().to_string()),
        quote(&symbol.name),
        quote(&symbol.path),
        quote(&format!("{:?}", symbol.kind)),
        symbol.depth,
        symbol.line,
        line_of_offset(source, symbol.span.1),
        quote(&symbol.signature),
    )
}

/// JSON string quoting for the fields that could carry a control character.
pub(super) fn quote(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|character| match character {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            control if (control as u32) < 0x20 => format!("\\u{:04x}", control as u32),
            other => other.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use miter_engine::polyglot::{Language, SymbolKind};

    fn symbol(name: &str, start: usize, end: usize) -> UniversalSymbol {
        UniversalSymbol {
            name: name.to_string(),
            language: Language::Rust,
            kind: SymbolKind::Function,
            path: name.to_string(),
            depth: 0,
            docstring: None,
            signature: format!("fn {name}()"),
            span: (start, end),
            file: PathBuf::from("src/lib.rs"),
            line: 1,
            params: Vec::new(),
            return_type: None,
            exported: true,
        }
    }

    #[test]
    fn innermost_prefers_the_narrowest_containing_span() {
        let (outer, inner) = (symbol("outer", 0, 100), symbol("inner", 40, 60));
        let found = innermost(&[&outer, &inner], 50).map(|found| found.name.as_str());
        assert_eq!(found, Some("inner"));
    }

    #[test]
    fn a_position_outside_every_span_has_no_containing_declaration() {
        let outer = symbol("outer", 0, 10);
        assert!(innermost(&[&outer], 10).is_none(), "span end is exclusive");
        assert!(innermost(&[&outer], 99).is_none());
    }

    #[test]
    fn line_and_offset_round_trip() {
        let source = "one\ntwo\nthree\n";
        assert_eq!(byte_offset_of_line(source, 1), 0);
        assert_eq!(byte_offset_of_line(source, 3), 8);
        assert_eq!(line_of_offset(source, 0), 1);
        assert_eq!(line_of_offset(source, 8), 3);
    }

    #[test]
    fn a_target_without_a_usable_line_is_a_usage_error() {
        assert!(matches!(
            parse_target("src/lib.rs"),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(parse_target(":12"), Err(CliError::Usage(_))));
        assert!(matches!(
            parse_target("src/lib.rs:0"),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(parse_target("src/a:b.rs:12"), Ok((_, 12))));
    }
}
