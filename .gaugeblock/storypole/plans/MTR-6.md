# Task Plan: GB-98 — Harmonize miter AGENTS.md, Workspace Metadata, and Clippy Lints

> **Task Key:** `GB-98`  
> **Scope:** `miter-cli` / workspace  
> **Status:** Implemented — Tier 2 gate green

---

## 1. Problem Statement

`miter` is the oldest repository in the suite and never received the metadata and lint harmonization the other nine did:

1. **Wrong canonical remote.** `[workspace.package]` pointed `repository`/`homepage` at
   `https://github.com/colinwolff/miter` instead of `https://github.com/gaugeblock/miter` /
   `https://gaugeblock.dev`, and `authors` carried no email.
2. **Missing publish metadata.** No `readme` key, so `cargo publish` falls back to `README.md` by
   convention rather than by declaration.
3. **No suite lint floor.** The workspace declared only `[workspace.lints.rust]`; the suite standard
   (`unwrap_used`/`expect_used` = deny) was absent, so production paths could silently unwrap.
4. **Stale README badge.** The badge advertised `0.1.0-beta.2` while the workspace is `0.1.0-beta.4`,
   and release links pointed at the personal remote.

---

## 2. Technical Blueprint

1. `Cargo.toml`: canonical `repository`/`homepage`, suite `authors`, explicit `readme`, and
   `[workspace.lints.clippy] unwrap_used = "deny"`, `expect_used = "deny"`.
2. Fix every library-path `unwrap()`/`expect()`:
   * `analysis/cycles.rs` — two `lowlink.get_mut(node)` unwraps became `if let Some(..)` guards;
   * `analysis/clones.rs` — `partial_cmp().unwrap()` became `unwrap_or(Ordering::Equal)` with an
     `// INVARIANT:` note that similarities are clamped to `0.0..=1.0`.
3. Add `#![allow(clippy::unwrap_used, clippy::expect_used)]` to test-only modules (43 files: `tests/`
   trees, `*tests.rs`, `tests_*.rs`, `cases.rs` fixture modules and inline `#[cfg(test)]` blocks),
   matching the suite convention that tests may unwrap while library paths may not.
4. README: badge to `0.1.0-beta.4`, release links to the `gaugeblock` org.

---

## 3. Execution Steps

* **Step 1:** Add suite clippy lints and fix library-path unwraps. Gate: `cargo clippy --workspace --all-targets -- -D warnings`.
* **Step 2:** Allow unwrap/expect in test-only modules. Gate: same clippy command.
* **Step 3:** Harmonize workspace metadata and README badge. Gate: `cargo metadata --no-deps`.
* **Step 4:** Tier 2 verification. Gate: `cargo nextest run --workspace` + `chalkline docs check -d miter`.

---

## 4. Boundaries

* `AGENTS.md` already carried the suite directives (250-line ceiling, tiered verification, storypole
  deferral), so no rewrite was needed beyond metadata.
* Dependency upgrades (tree-sitter 0.27, sqlparser 0.63) stay with `GB-20`/`GB-21`.
