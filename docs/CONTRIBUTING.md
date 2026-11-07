# Contributor Guide — `miter`

> Guidelines for engineering, testing, and contributing to the `miter` codebase.

---

## 1. Toolchain & Environment Setup

`miter` is developed using the **Rust 2024 edition**:

```bash
# Verify rust toolchain
rustc --version # Rust 1.90+ (2024 edition compatible, rust-version = "1.90")

# Install cargo-nextest (preferred test runner)
cargo install cargo-nextest --locked
```

Clone the repository and compile the workspace:

```bash
git clone https://github.com/gaugeblock/miter.git
cd miter
cargo check --workspace --all-targets
```

---

## 2. Workspace Organization

```text
miter/
├── src/                       # Root facade and miter CLI binary
└── crates/
    ├── ast_indexer/           # miter-ast: Tree-sitter parsers and grammar tables
    ├── engine/                # miter-engine: Storage, hydration, audits, contracts
    ├── storage/               # miter-storage: Embedded redb B-Tree schemas
    ├── harness/               # miter-harness: Ephemeral worktree compiler verification
    └── mcp/                   # miter-mcp: JSON-RPC 2.0 stdio server
```

---

## 3. Tiered Verification Workflow

Before submitting a Pull Request, all changes must pass through the tiered verification gates:

### Tier 1 (Inner Loop)
Rapid iteration check while modifying a crate:
```bash
cargo check -p miter-engine
cargo nextest run -p miter-engine --lib hydrate
```

### Tier 2 (Commit Gate)
Run on affected crates prior to making an atomic commit:
```bash
cargo fmt --check -p miter-engine
cargo clippy -p miter-engine --all-targets -- -D warnings
cargo nextest run -p miter-engine
chalkline docs check -d .
```

### Tier 3 (Release Gate)
Full workspace regression check:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
./scripts/test.sh
```

---

## 4. Coding Standards & Invariants

1. **Hard 250-Line File Limit:** No `.rs` source file may exceed 250 lines. Decompose proactively into submodules.
2. **Strict Error Handling:** Library crates must use `thiserror`. Never use `.unwrap()` or `.expect()` in non-test code.
3. **Deterministic Output:** Code must never invoke wall-clock time, RNG, or un-sorted map iteration on query paths.
4. **Secret Scrubbing:** All extractors must route string literals through `engine::security::scrub_secrets`.
5. **Conventional Commits:** `<type>(<scope>): <summary>` (e.g. `feat(miter-engine): add SimHash clone detector`).
