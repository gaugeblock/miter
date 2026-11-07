# Product Vision & Specifications — `miter`

> **Domain:** AST Slicing, Cross-Language Contracts & Verification Engine  
> **Ecosystem:** `gaugeblock` Neurosymbolic Meta-Harness  
> **License:** Apache-2.0

---

## 1. Problem Statement & Market Context

Modern AI coding agents (Claude Code, Cursor, Cline, Windsurf) fail or stall when navigating growing codebases for four fundamental reasons:

1. **Context Window Starvation:** Agents dump 800-line source files into prompts just to read 3 method signatures, burning 80%+ of their token budget on syntax bodies they never mutate.
2. **Credential & Secret Leakage:** When querying configuration manifests (`.env`, `Dockerfile`, Kubernetes YAML), raw API keys and database credentials leak into agent prompts and third-party LLM cloud logs.
3. **High-Risk File Splitting:** When asked to break apart bloated files, agents introduce circular imports, broken module trees, and missing `pub` visibility declarations.
4. **Silent Cross-Language Schema Drift:** Database migrations (`.sql`), backend models (Rust `struct`), and frontend types (TypeScript `interface`) diverge unnoticed until runtime production crashes.

`miter` solves this by providing a local, deterministic AST co-processor that replaces stochastic `grep` with structural X-ray vision.

---

## 2. Target Personas & User Journeys

### Persona A: Autonomous AI Coding Agent
- **Needs:** Fast, token-efficient queries for symbol signatures; reverse reference lookups; precise blast radii; zero raw secret exposure.
- **Workflow:** Calls `miter_skeleton` to outline modules, `miter_lookup` for target symbol context, `miter_impact` to identify affected unit tests, and `miter_verify` to confirm compilation in an ephemeral sandbox.

### Persona B: Systems Software Engineer
- **Needs:** Immediate feedback on architectural hygiene, line-count ceilings (<=250 LOC), circular module dependencies, and dead code.
- **Workflow:** Runs `miter audit` in pre-commit hooks; uses `miter slice` to generate clean refactoring seams for oversized files.

### Persona C: Full-Stack Architect
- **Needs:** Continuous guarantee that database columns, backend ORM types, and frontend interfaces remain 100% in sync.
- **Workflow:** Integrates `miter contracts` into CI gates to halt PRs whenever an entity schema drifts.

---

## 3. Jobs to Be Done (JTBD)

| When I... | I want to... | So that I can... |
|---|---|---|
| Need to edit a function in a large file | Retrieve only the function signature, docstring, and immediate neighbours | Conserve 85%+ of context tokens and avoid distracting the LLM with irrelevant implementation bodies. |
| Decompose a 600-line monolithic file | Compute optimal extraction seams and identify needed visibility elevations | Refactor safely into submodules without breaking existing callers or introducing import cycles. |
| Change a shared database entity | Compare schemas across SQL, Rust, and TypeScript definitions | Detect missing fields or type mismatches immediately at compile time. |
| Verify a generated code patch | Run the target compiler inside an isolated, detached git worktree | Verify compilation and capture diagnostic errors with zero side effects on the developer's working tree. |

---

## 4. Key Metrics & Success Criteria

* **Query Latency:** `<10ms` for hydrated symbol lookups from embedded `redb`.
* **Context Efficiency:** `>85%` reduction in prompt token consumption compared to whole-file ingestion.
* **Secret Redaction Fidelity:** `100%` redaction rate on known credential heuristics across all 13 supported formats.
* **Deterministic Reproducibility:** Identical binary output bit-for-bit across any machine and operating system for a given git commit.
