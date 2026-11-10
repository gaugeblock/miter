# Task Plan: GB-157 — Extract `miter-syntax` Below `miter-engine`

> **Epic Key:** `GB-157`
> **Scope:** `miter` (new crate, two re-export shims), then `dado` and `batten`
> **Status:** Planned — ready to execute, nothing edited yet

---

## 1. Objective

Move the grammar-level surface — language taxonomy, grammar table, parser construction, syntax
diagnostics — into a `miter-syntax` crate, so a consumer that only needs to *parse* or *name a
format* stops linking the analysis stack.

Evidence (measured, not assumed):

| Consumer | Imports from `miter-engine` | Needs the analysis stack? |
|:--|:--|:--|
| `chalkline` | `inspect_workspace`, `WorkspaceInventory`, `PublicTypeInventory` | yes |
| `benchwork` | `inspect_workspace`, `WorkspaceInventory`, `ToolchainInventory` | yes |
| `batten` | `Language` only (3 uses) | **no** |
| `dado` | `polyglot` only (3 lines, the GB-148 adapters) | **no** |
| `joiner` | `Language` + `security::scrub_source_text` | only for the scrubber → `GB-158` |
| `rabbet` | `Language` + `security::{…}` | only for the scrubber → `GB-158` |

`miter-engine` today pulls `miter-ast`, `miter-storage` (redb), `redb`, `rayon`, `sqlparser`, `hcl`,
`dockerfile-parser`, `serde_yaml`, `globset`, `toml`. `batten` and `dado` link all of it to obtain an
enum naming thirteen formats, or to parse one string.

Target: `miter-engine`'s dependents go from six to two, with no functional change anywhere.

---

## 2. Constraints discovered before writing code

1. **`grammar_for` cannot move alone.** It lives at
   `miter-ast/src/extract/grammar.rs:83` and takes `miter_ast::Language`. Moving the table without the
   taxonomy creates a cycle (`miter-ast → miter-syntax → miter-ast`). Both move together.
2. **The `From` impls must move too (orphan rule).** `polyglot/contracts.rs` (50 lines) defines
   `impl From<PolyglotLanguage> for WireLanguage` **and** `impl From<WireLanguage> for PolyglotLanguage`.
   Once `PolyglotLanguage` lives in another crate, neither impl may stay in `miter-engine`. So
   `miter-syntax` depends on `gaugeblock-contracts`, which is correct — contracts is the bottom layer
   and the conversions are what keep the two taxonomies in lockstep.
3. **`polyglot/language.rs` is 246 lines** — four from the hard 250 limit and 46 over the soft target.
   It must not be moved verbatim; split it while moving.
4. **`miter-ast` keeps its semantics.** `FunctionNode`, `Param`, `parse_source`, `signature_of` and the
   indexer visitor stay; only the taxonomy and the grammar table leave. `miter-ast` re-exports both so
   `miter_ast::{Language, grammar_for}` keep resolving.

---

## 3. Slices (each a branch, a gate, and an atomic commit)

### Slice 1 — create the crate and shim the surface (`miter`)
Branch `feat/miter-syntax-crate`.

* `crates/syntax/` → package `miter-syntax`, `version.workspace = true`, `[lints] workspace = true`.
* Move `Language`/`PolyglotLanguage` out of `ast_indexer/src/extract/mod.rs` **split** so no file is near
  the limit: `language/mod.rs` (variants, `name`, `from_extension`, `from_path`, `from_file_name`,
  `is_declarative`) and `language/formats.rs` (the `EXTENSIONS` table, ~90 lines).
* Move `extract/grammar.rs::grammar_for` unchanged; it becomes `miter_syntax::grammar_for`.
* Move `polyglot/contracts.rs` verbatim (the two `From` impls).
* Move `polyglot/syntax.rs` (158 lines) unchanged.
* `miter-ast`: depend on `miter-syntax`; `pub use miter_syntax::{Language, grammar_for};` so
  `miter_ast::{Language, grammar_for}` and the `extract` re-export list keep working.
* `miter-engine`: depend on `miter-syntax`; `polyglot/mod.rs` re-exports the taxonomy, conversions and
  diagnostics at today's paths, so no downstream import changes in this slice.

Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo nextest run --workspace` (baseline 238 tests).

### Slice 2 — repoint `dado`
Branch `feat/dado-miter-syntax`. Replace `miter-engine` with `miter-syntax` in `dado-core`; the
`ast_check` adapter changes one import path. Drop the dead `miter-ast` dependency too (declared today,
never used). Gate: `cargo nextest run -p dado-core` (baseline 14).

### Slice 3 — repoint `batten`
Branch `feat/batten-miter-syntax`. Three `Language` uses. Gate: batten workspace suite.

### Slice 4 — verify
`storypole verify GB-157` with the dependency counts before and after; leave `joiner`/`rabbet` for
`GB-158`.

---

## 4. Risks and how they are handled

* **Re-export drift.** A shim that silently diverges from the real path is how `GB-148` began. Slice 1
  re-exports rather than aliases (`pub use`), so there is exactly one definition.
* **Crate version coupling.** All miter crates share `version.workspace = true` and publish in lockstep,
  so a fourth crate adds a publish target (handled by `outfeed publish`'s topological walk), not a
  version to coordinate. No version changes are made in this ticket; the bump belongs to the next release.
* **Test coverage moves with the code.** The six `syntax_diagnostics_tests` and the language contract
  test move into `miter-syntax`'s own `tests/`, so the new crate is not shipped untested.
