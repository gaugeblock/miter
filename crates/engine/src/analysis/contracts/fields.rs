//! Canonical names and the field list read off a declaration.

use crate::polyglot::{SymbolKind, UniversalSymbol};

use super::EntityField;

/// Normalizes an identifier into lowercased alphanumeric words.
pub fn normalize_identifier(ident: &str) -> String {
    let mut clean = String::new();
    let mut prev_is_lower = false;
    for ch in ident.chars() {
        if ch == '_' || ch == '-' {
            clean.push('_');
            prev_is_lower = false;
        } else if ch.is_uppercase() {
            if prev_is_lower {
                clean.push('_');
            }
            clean.push(ch.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            clean.push(ch);
            prev_is_lower = ch.is_alphabetic();
        }
    }
    clean.trim_matches('_').replace("__", "_")
}

/// Normalizes an entity name into canonical singular snake_case.
#[must_use]
/// Derives the canonical entity name for a declaration (e.g. `ResidentLedger` -> `resident_ledger`).
pub fn canonical_entity_name(name: &str) -> String {
    let norm = normalize_identifier(name);
    if norm.len() > 3 && norm.ends_with('s') && !norm.ends_with("ss") {
        norm[..norm.len() - 1].to_string()
    } else {
        norm
    }
}

/// Reads the fields a declaration carries, when it declares an entity.
pub fn extract_fields_from_symbol(sym: &UniversalSymbol) -> Vec<EntityField> {
    let mut fields = Vec::new();
    let sig = sym.signature.trim();

    // 1. SQL Table signature: TABLE <name> (<col1>: <TYPE>, ...)
    if sym.kind == SymbolKind::Table && sig.starts_with("TABLE ") {
        if let Some(open) = sig.find('(')
            && let Some(close) = sig.rfind(')')
        {
            let cols_str = &sig[open + 1..close];
            for part in cols_str.split(',') {
                let trimmed = part.trim();
                if trimmed.starts_with("FK") {
                    continue;
                }
                let col_name = trimmed.split(':').next().unwrap_or("").trim();
                if !col_name.is_empty() {
                    fields.push(EntityField {
                        raw_name: col_name.to_string(),
                        canonical_name: normalize_identifier(col_name),
                        type_hint: trimmed.split(':').nth(1).map(|t| t.trim().to_string()),
                    });
                }
            }
        }
    } else {
        // 2. Struct or Interface fields from signature or params
        for param in &sym.params {
            fields.push(EntityField {
                raw_name: param.name.clone(),
                canonical_name: normalize_identifier(&param.name),
                type_hint: if param.type_name.is_empty() {
                    None
                } else {
                    Some(param.type_name.clone())
                },
            });
        }
    }

    fields
}

/// The fields of a declaration, read from its own record or from the members beneath it.
///
/// A SQL table carries its columns in its signature — one row per column is not a declaration —
/// while a Rust struct's or a TypeScript interface's fields are their own `Property` symbols under
/// the parent's path. Both are read here, so the contract pass sees one shape whichever language
/// wrote the entity.
#[must_use]
pub fn fields_of(symbols: &[UniversalSymbol], sym: &UniversalSymbol) -> Vec<EntityField> {
    let own = extract_fields_from_symbol(sym);
    if !own.is_empty() {
        return own;
    }
    let mut fields = Vec::new();
    collect_fields(symbols, sym, 0, &mut fields);
    fields.sort_by(|a, b| a.canonical_name.cmp(&b.canonical_name));
    fields.dedup_by(|a, b| a.canonical_name == b.canonical_name);
    fields
}

/// How deep the walk follows nesting, in fields.
///
/// One level is enough for the shapes the suite actually has - a config record holding a small named
/// sub-record - and a bound keeps a self-referential type from being walked forever.
const MAX_NESTING: usize = 2;

/// Collects a declaration's fields, descending into nested objects.
///
/// The two languages express one shape differently and the comparison has to see the same thing
/// either way (`GB-239`). TypeScript inlines an object - `spec_docs: { primary_spec_present: bool }` -
/// and its properties index *beneath the parent's path*, so they read as fields of the parent. Rust
/// names a struct - `pub spec_docs: SpecDocs` - and `SpecDocs`'s fields index beneath `SpecDocs`, which
/// is not the parent's path at all, so they were invisible. Comparing top-level fields alone therefore
/// reported a correctly nested pair as nine or eleven missing fields.
///
/// Fields are emitted under a dotted path (`spec_docs.primary_spec_present`) on both sides, so depth
/// stops mattering.
fn collect_fields(
    symbols: &[UniversalSymbol],
    sym: &UniversalSymbol,
    depth: usize,
    out: &mut Vec<EntityField>,
) {
    if depth > MAX_NESTING {
        return;
    }
    let prefix = format!("{}.", sym.path);
    for child in symbols
        .iter()
        .filter(|child| child.kind == SymbolKind::Property && child.path.starts_with(&prefix))
    {
        // A direct property's path is `<parent>.<name>`; a property of an inlined object is
        // `<parent>.<field>.<name>`. Only the middle segment distinguishes them.
        let relative = child.path.strip_prefix(&prefix).unwrap_or("");
        let owner = relative
            .strip_suffix(child.name.as_str())
            .unwrap_or("")
            .trim_end_matches('.');
        let dotted = if owner.is_empty() {
            child.name.clone()
        } else {
            format!("{owner}.{}", child.name)
        };
        out.push(EntityField {
            raw_name: dotted.clone(),
            canonical_name: canonical_field_path(&dotted),
            // A property record does not always carry its annotation, but the signature it was read
            // from still does - and without it a named nested type can never be followed.
            type_hint: child
                .return_type
                .clone()
                .or_else(|| type_from_signature(&child.signature)),
        });
    }

    // A named nested type is resolved by name and walked under the field that refers to it.
    for field in out.clone() {
        if field.raw_name.contains('.') {
            continue;
        }
        let Some(type_name) = field.type_hint.as_deref().and_then(last_type_name) else {
            continue;
        };
        let Some(nested) = symbols.iter().find(|candidate| {
            matches!(candidate.kind, SymbolKind::Struct | SymbolKind::Interface)
                && candidate.name == type_name
                && candidate.path != sym.path
        }) else {
            continue;
        };
        let mut inner = Vec::new();
        collect_fields(symbols, nested, depth + 1, &mut inner);
        for mut child in inner {
            child.raw_name = format!("{}.{}", field.raw_name, child.raw_name);
            child.canonical_name = canonical_field_path(&child.raw_name);
            out.push(child);
        }
    }
}

/// Canonicalizes a dotted path segment by segment.
fn canonical_field_path(path: &str) -> String {
    path.split('.')
        .map(normalize_identifier)
        .collect::<Vec<_>>()
        .join(".")
}

/// The bare type name of an annotation, ignoring `Option<...>`, `Vec<...>`, `[]` and module paths.
fn last_type_name(annotation: &str) -> Option<String> {
    let mut text = annotation.trim();
    if let Some(start) = text.find('<') {
        let end = text.rfind('>')?;
        text = &text[start + 1..end];
    }
    let text = text.trim().trim_end_matches("[]").trim();
    if text.is_empty() {
        return None;
    }
    let bare = text.rsplit(['.', ':']).next().unwrap_or(text).trim();
    let is_object_shape = bare.contains('{') || bare.contains('[');
    if bare.is_empty() || is_object_shape {
        None
    } else {
        Some(bare.to_string())
    }
}

/// The declared type read off a property's signature, for records that carry no annotation field.
fn type_from_signature(signature: &str) -> Option<String> {
    let (_, after) = signature.rsplit_once(':')?;
    last_type_name(after.trim().trim_end_matches(',').trim_end_matches(';'))
}
