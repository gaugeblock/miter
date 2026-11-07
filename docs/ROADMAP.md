# Product Roadmap & Strategic Horizons — `miter`

> **Active Suite Lifecycle:** `0.1.0-beta.X`  
> **Target:** Autonomous AST Slicing, Cross-Language Contracts & Verification

---

## 1. Active Milestone: `v0.1.0-beta.2` (Current Focus)

- [x] **13 Polyglot Formats:** Complete Tree-sitter and declarative ingestion for Rust, TS, JS, Python, SQL, Docker, K8s, Terraform, Markdown, HTML, CSS, SCSS, and `.env`.
- [x] **Embedded `redb` Engine:** MVCC multi-reader storage with automatic purge recovery.
- [x] **AST Secret Scrubbing:** Automatic AST-aware credential redaction at ingestion and query boundaries.
- [x] **Model Context Protocol (MCP):** JSON-RPC 2.0 stdio server exposing 7 core tools.
- [x] **Ephemeral Worktree Sandboxing:** `miter verify --sandbox` with detached git worktree lifecycle.
- [x] **9-Document Architecture Compliance:** Full alignment with `chalkline docs check`.

---

## 2. Horizon 1: Polyglot Expansion & Semantic Call Graphs (`v0.2.0`)

- [ ] **Expanded Language Support:** Add Tree-sitter parsers for Go, C/C++, and Java.
- [ ] **Cross-Language Semantic Call Graphs:** Infer end-to-end call paths across boundaries (e.g. tracing TypeScript frontend HTTP calls to Rust `axum` routes or Python `FastAPI` endpoints).
- [ ] **File Watcher & Incremental Ingestion Daemon:** File-system notification (`notify` crate) daemon for zero-latency instant dirty-symbol reindexing.
- [ ] **SQL Query Table References:** AST parsing of complex SQL joins and view definitions for dependency tracking.

---

## 3. Horizon 2: Live Memory-Mapped Cache & LSP Integration (`v0.3.0`)

- [ ] **Bidirectional LSP Bridge:** Translate Language Server Protocol (LSP) diagnostics and completion items directly into `miter` symbol models.
- [ ] **Zero-Copy Memory-Mapped IPC:** Shared-memory ring buffer for sub-millisecond symbol queries between `miter` and `joiner`.
- [ ] **Multi-Workspace Aggregation:** Support seamless symbol queries across multi-repo workspaces without per-repo flag juggling.

---

## 4. Horizon 3: Formal Verification & Enterprise Scale (`v1.0.0`)

- [ ] **SMT Solver Integration:** Connect symbol contracts with Z3 / cvc5 for formal invariant verification.
- [ ] **Monorepo Scale Optimizations:** Scale to 10M+ LOC codebases with sub-second incremental re-indexing.
- [ ] **Long-Term Wire Stability:** Lock `ENGINE_STORAGE_VERSION = 1` binary layout for multi-year backward compatibility.
