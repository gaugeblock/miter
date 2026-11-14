//! The universal half of one parse (`GB-315`).
//!
//! [`Engine::insert`] writes the function-shaped record the query pipeline ranks; this writes the universal
//! record the coherence scanner and context hydration read. They are complementary halves of the same pass, so
//! the identity rule has to match: one entry per declaration, not one per read.

use crate::store::Engine;

impl Engine {
    /// Returns the universal polyglot symbols, if any were ingested.
    pub fn universal_symbols(&self) -> &[crate::polyglot::UniversalSymbol] {
        &self.universal_symbols
    }

    /// Records every universal symbol a polyglot pass read.
    ///
    /// The two ingestion paths are complementary halves of one parse, and both have to land:
    /// [`Self::insert`] writes the *function-shaped* record the query pipeline ranks, while this
    /// writes the *universal* record the coherence scanner and context hydration read. A `struct`,
    /// an `interface` or a decorated `def` produces no function record at all, so an engine that
    /// skipped this step could not say anything about one — which is why the ingestion passes
    /// call it rather than leaving it to the caller.
    ///
    /// Appends rather than replaces, because a tracked pass is incremental: it reports the files
    /// it re-read, and the symbols of an unchanged file were recorded by the pass that read it.
    pub fn add_universal_symbols(
        &mut self,
        symbols: impl IntoIterator<Item = crate::polyglot::UniversalSymbol>,
    ) {
        // One entry per declaration, not one entry per read (`GB-315`). A tracked pass appends the files it
        // re-read while a full pass re-reads everything, so without this the list holds the same declaration
        // twice and every consumer walks it twice - the index reported 7542 universal symbols beside 4868
        // function records, more declarations than code. Identity is where the declaration lives: the same
        // name in two files is two symbols, and the same span in one file is one symbol however often it is
        // read.
        let mut seen: std::collections::HashSet<(
            std::path::PathBuf,
            crate::polyglot::Language,
            (usize, usize),
        )> = self
            .universal_symbols
            .iter()
            .map(|symbol| (symbol.file.clone(), symbol.language, symbol.span))
            .collect();
        for symbol in symbols {
            if seen.insert((symbol.file.clone(), symbol.language, symbol.span)) {
                self.universal_symbols.push(symbol);
            }
        }
    }
}

#[cfg(test)]
mod universal_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::polyglot::{Language, SymbolKind, UniversalSymbol};
    use std::path::PathBuf;

    fn symbol(file: &str, span: (usize, usize)) -> UniversalSymbol {
        UniversalSymbol {
            name: "scale".to_string(),
            language: Language::Rust,
            kind: SymbolKind::Function,
            path: "geom::scale".to_string(),
            depth: 1,
            docstring: None,
            signature: "fn scale(x: f64) -> f64".to_string(),
            span,
            file: PathBuf::from(file),
            line: 1,
            params: Vec::new(),
            return_type: None,
            exported: true,
        }
    }

    #[test]
    fn the_same_declaration_read_twice_is_stored_once() {
        // The measured symptom: more universal symbols than function records, because a re-read appended.
        let mut store = Engine::new();
        store.add_universal_symbols(vec![symbol("a.rs", (0, 10))]);
        store.add_universal_symbols(vec![symbol("a.rs", (0, 10))]);
        assert_eq!(
            store.universal_symbols().len(),
            1,
            "one declaration, two reads"
        );
    }

    #[test]
    fn the_same_name_in_two_files_or_two_spans_is_two_symbols() {
        let mut store = Engine::new();
        store.add_universal_symbols(vec![symbol("a.rs", (0, 10)), symbol("b.rs", (0, 10))]);
        store.add_universal_symbols(vec![symbol("a.rs", (20, 30))]);
        assert_eq!(
            store.universal_symbols().len(),
            3,
            "identity is the declaration's location, not its name"
        );
    }
}
