# System Architecture

## Crate & Package Topology

### `miter-ast`
- Path: `crates/ast_indexer`
- Version: `0.1.0-beta.2`
- Description: Tree-sitter AST ingestion: functions, inputs and return types with their provenance and nesting depth

### `miter-cli`
- Path: ``
- Version: `0.1.0-beta.2`
- Description: Workspace member crate

### `miter-engine`
- Path: `crates/engine`
- Version: `0.1.0-beta.2`
- Description: AST storage and analysis engine: polyglot model, hydration, audit, slices, clones, contracts and secret redaction

### `miter-harness`
- Path: `crates/harness`
- Version: `0.1.0-beta.2`
- Description: Deterministic compiler verification: sandboxed git worktrees and diagnostic parsing for `miter verify`

### `miter-mcp`
- Path: `crates/mcp`
- Version: `0.1.0-beta.2`
- Description: Model Context Protocol (MCP) stdio server exposing AST hydration and compiler verification

### `miter-storage`
- Path: `crates/storage`
- Version: `0.1.0-beta.2`
- Description: Canonical redb tables, schema versioning and auto-purge recovery for the miter workspace database

