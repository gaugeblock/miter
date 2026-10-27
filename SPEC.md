# Behavioral Specifications & Invariants

## Formal Type Contracts

- **`impact::AffectedTest`** (`struct`): An affected unit test or integration test detected in the blast radius.
- **`indexer::AstIndexer`** (`struct`): Turns source code into encoded [`FunctionSymbol`]s.  The indexer is stateless, so it is `Sync` and can be shared by the parallel ingestion pipeline in the engine.
- **`slicer::Attention`** (`struct`): Public interface entity
- **`mod::AuditArgs`** (`struct`): Arguments parsed for the `miter_audit` tool.
- **`audit::AuditConfig`** (`struct`): Configuration options for workspace auditing.
- **`audit::AuditFormat`** (`enum`): Format choice for `miter audit`.
- **`audit::AuditSeverity`** (`enum`): Severity classification of a line count violation.
- **`config::AuditToolConfig`** (`struct`): Tool-specific configuration for file-length auditing.
- **`audit::AuditViolation`** (`struct`): A violation flagged during workspace auditing.
- **`mod::CacheStore`** (`struct`): A cache of decoded index entries.
- **`protocol::CallToolResult`** (`struct`): Payload returned by `tools/call`.
- **`cargo_extractor::CargoExtraction`** (`struct`): Discovered cargo manifests and toolchain config.
- **`args::Cli`** (`struct`): Public interface entity
- **`types::CliArgInventory`** (`struct`): Discovered CLI argument or flag.
- **`types::CliCommandInventory`** (`struct`): Discovered CLI commands and arguments.
- **`error::CliError`** (`enum`): Failures reported by the CLI, mapped to exit code 1.
- **`clones::CloneDetectionReport`** (`struct`): Summary report of detected AST structural clones.
- **`clones::ClonesFormat`** (`enum`): Format choice for `miter clones`.
- **`args::Command`** (`enum`): Subcommand definitions.
- **`tests::Commands`** (`enum`): Public interface entity
