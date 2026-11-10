# Task Plan: GB-199 — `miter deps` must not count literals as references

> **Scope:** `miter/crates/engine/src/analysis/deps` **Parent:** `GB-200` **Status:** InProgress

## 1. Symptom

Reading the suite-wide graph after GB-198 showed two edges that cannot exist:

```
miter-engine -> outfeed          1 site
miter-engine -> storypole-core   1 site
```

Both came from `analysis/deps/tests.rs`, where the *fixture strings* contain lines like

```rust
let source = "pub mod outfeed;\nuse storypole_core::model::TaskNode;\n";
```

The scanner read every physical line of every `.rs` file as code, so data became proof of use, and a
fixture that exists to *test* the parser registered a module (`pub mod outfeed;`) as well.

This matters beyond two edges: every number produced by `miter deps` feeds the layer gate that
`GB-202` will introduce, and a gate built on phantom edges is worse than no gate.

## 2. Fix

A line is passed through `strip::code_only` before anything is matched. It removes:

* `//` line comments and `/* */` block comments (state carried across lines),
* ordinary string literals with backslash escapes,
* raw strings `r"..."`, `r#"..."#`, `r##"..."##`, including multi-line ones (state carried).

`declared_modules` uses the same stripper, so a fixture can no longer declare a module either.

Deliberate conservatism: an unterminated raw string or block comment swallows the remainder of the
file rather than guessing a close. Guessing is precisely what produced the false edges, and a missed
reference is cheaper than a fabricated dependency.

## 3. Proof

| Check | Expectation |
|:--|:--|
| `test_import_looking_content_inside_strings_is_not_a_reference` | fixture string, raw string and hashed raw string yield no roots and no modules |
| `test_a_multiline_raw_string_swallows_the_lines_it_covers` | only the `use` after the closing `"#;` is counted |
| `test_comments_hide_imports_but_real_code_after_a_block_comment_still_counts` | a trailing `/* */` does not swallow the statement before it |
| `miter deps --path miter` | `miter-engine -> outfeed` and `-> storypole-core` are gone |
| `miter deps --path .` | cross-repo edge count drops from 132 |

Gates: `cargo nextest run -p miter-engine` (165), `cargo clippy -p miter-engine -p miter-cli
--all-targets -- -D warnings`, `cargo fmt --all -- --check`.

## 4. Non-goals

* Not changing what counts as *use* otherwise: a dependency used only in `#[cfg(test)]` code still
  counts as used, because the declaration half of that problem belongs to `GB-201`.
* Not touching `miter clones`' similarity semantics (`GB-197`).
