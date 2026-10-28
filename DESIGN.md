# Architectural Design & Decision Records — `miter`

> **Crates:** `miter-cli`, `miter-engine`, `miter-ast`, `miter-storage`, `miter-harness`, `miter-mcp`  
> **Status:** Active Architectural Decision Records (ADRs)

---

## Architectural Decision Records (ADRs)

### ADR-001: Embedded `redb` Over SQLite or In-Memory Graphs
- **Status:** Accepted
- **Context:** `miter` requires an embedded storage engine that can index hundreds of source files in seconds, sustain high-concurrency read operations from multiple agent subshells, and guarantee ACID consistency without requiring an external database server daemon.
- **Decision:** Use `redb`, a pure-Rust embedded key-value store with MVCC multi-reader concurrency, zero C/FFI dependencies, and zero SQLite locking quirks across file boundaries.
- **Consequences:** Lightning-fast cold start (<1.5s for entire repo), zero runtime C dependency compilation issues, and robust zero-panic recovery via automated store purge when schema versions change.

### ADR-002: Content-Addressed FNV-1a64 Symbol Hashing & Neighbourhood Topology
- **Status:** Accepted
- **Context:** Code hydration requires deterministic, stable symbol identifiers that survive across incremental edits without depending on dynamic auto-incrementing database sequence numbers.
- **Decision:** Derive 64-bit symbol IDs using FNV-1a hashing of `path:line:name` (`seed_from_text`). Rank hydration neighbours using a strict 4-tuple comparator: depth delta, name token affinity, line distance, and ID tie-breaker.
- **Consequences:** Hydration returns identical, deterministic outputs across all platforms and runs without being perturbed by file iteration order or RNG.

### ADR-003: Dual-Boundary AST Secret Redaction
- **Status:** Accepted
- **Context:** AI coding agents frequently inspect `.env` files, Docker manifests, and config YAMLs where API keys, tokens (`sk_live_...`), and connection strings reside, creating severe data leak hazards.
- **Decision:** Enforce automatic AST-aware secret scrubbing at two physical boundaries: (1) during raw file ingestion before records enter `.miter/miter.redb`, and (2) during serialization of MCP and CLI query responses.
- **Consequences:** Agents receive structural signatures, environment keys, and port numbers with zero risk of ingesting raw credentials into LLM contexts.

### ADR-004: Ephemeral Git Worktree Sandboxing for Compiler Verification
- **Status:** Accepted
- **Context:** Verifying whether an agent's code patch compiles must never corrupt the active working tree, dirty the index, or alter the developer's checked-out branch.
- **Decision:** Execute `miter verify --sandbox` in an isolated detached git worktree (`git worktree add --detach <ephemeral_path>`). Apply the candidate patch, run compiler checks (`cargo check`, `tsc`), capture stdout/stderr JSON diagnostics, and remove the worktree in a cleanup guard.
- **Consequences:** Provably side-effect-free compiler verification with zero disk footprint pollution on error or interruption.

### ADR-005: 128-Bit SimHash for Fast AST Clone Detection
- **Status:** Accepted
- **Context:** Detecting duplicate or near-duplicate functions across large polyglot repositories must avoid quadratic $O(N^2)$ AST diff comparison overhead.
- **Decision:** Tokenize normalized AST syntax trees into feature bags, compute 128-bit SimHash fingerprints, and compare Hamming distances with a 90% default similarity cutoff.
- **Consequences:** Linear-time clone detection across thousands of functions in under 200 milliseconds.
