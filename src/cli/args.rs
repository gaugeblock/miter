//! The clap surface: the `Cli` parser and its `Command` subcommands.
//!
//! Every subcommand the binary exposes is declared here and nothing else — no logic, no I/O — so
//! the command set can be read (and reviewed) in one place. `execute` in [`super`] is the only
//! thing that dispatches it, and the types are re-exported from `crate::cli`, so a caller naming
//! `miter::cli::Command` does not have to know this module exists.

use super::args_deps::DepsArgs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// `miter` — deterministic AST co-processor and MCP server.
#[derive(Debug, Parser)]
#[command(
    name = "miter",
    version,
    about = "Deterministic AST co-processor and Model Context Protocol (MCP) server",
    long_about = "Indexes a polyglot source tree into an embedded database and answers structural \n\
                  queries: symbol hydration, interface skeletons, reverse references, blast radius, \n\
                  schema contracts and clone detection. Intended as a tool interface for autonomous \n\
                  coding agents: every command can emit JSON.\n\n\
                  DATABASE LOCKING:\n\
                  a database is one writer at a time. Read-only commands (lookup, references, \n\
                  impact, contracts) take a shared lock and can run concurrently with each other, \n\
                  but a command that writes (index) holds an exclusive lock, and a read against it \n\
                  fails with \"Database already open\". Point --db at a different file to work in \n\
                  parallel."
)]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommand definitions.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Parse a source tree and write the encoded symbols into the database.
    Index {
        /// Directory (or file) to index.
        path: PathBuf,
        /// Database file to write (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Incremental dirty-file indexing mode driven by mtime and git status.
        #[arg(long)]
        dirty: bool,
        /// Re-parse every file, ignoring the stored baseline.
        #[arg(long, conflicts_with = "dirty")]
        full: bool,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Find the innermost declaration containing a source position.
    Enclosing {
        /// Position as `<file>:<line>`, line one-based, as a compiler error reports it.
        target: String,
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Emit JSON instead of the rendered payload.
        #[arg(long)]
        json: bool,
    },
    /// Locate a literal and name the declarations the hits land in.
    Find {
        /// Literal text to search for (`--ignore-case` folds ASCII only).
        pattern: String,
        /// Match ASCII case-insensitively.
        #[arg(long)]
        ignore_case: bool,
        /// Maximum hits to report.
        #[arg(long, default_value_t = 100)]
        max: usize,
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Emit JSON instead of the rendered report.
        #[arg(long)]
        json: bool,
    },
    /// Locate a declaration and print its hydrated context.
    Lookup {
        /// Declaration name, or a qualified path such as `miter_engine::store::Engine`.
        symbol: String,
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Print the hydrated neighbourhood rather than the bare identity.
        #[arg(long)]
        context: bool,
        /// Emit JSON instead of the rendered payload.
        #[arg(long)]
        json: bool,
        /// Token ceiling for the rendered payload.
        #[arg(long, default_value_t = miter_engine::DEFAULT_MAX_TOKENS)]
        max_tokens: usize,
    },
    /// Launch the Model Context Protocol (MCP) stdio server for AI coding agents.
    ///
    /// Speaks JSON-RPC 2.0 over standard I/O (stdin/stdout), exposing `miter_audit`,
    /// `miter_skeleton`, `miter_references`, `miter_lookup`, `miter_slice`, `miter_verify`,
    /// `miter_impact`.
    Mcp {
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Audit the workspace and **exit non-zero** on what it finds.
    ///
    /// Every repository's verification chain gates on this command's exit code, so it is a check that prints a
    /// report rather than a report that happens to run. `--paths` adds the policy gates: the 250-line limit, the
    /// rule that a crate naming `.gaugeblock` must depend on `gaugeblock-workspace`, and the rule that a crate
    /// resolving the suite must have a multi-repository test.
    ///
    /// `miter verify` is a different thing: isolated compiler verification of one crate, used by `storypole
    /// verify` to produce a receipt. It gates nothing in the chains.
    Audit {
        /// Add the policy gates: layout adoption (a crate naming `.gaugeblock` must depend on
        /// `gaugeblock-workspace`) and the multi-repository test requirement.
        #[arg(long)]
        paths: bool,
        /// Target root directory to audit (defaults to current workspace).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Line count threshold to trigger warning/critical violations (default: 500).
        #[arg(long)]
        threshold: Option<usize>,
        /// Output format (`table` or `json`).
        #[arg(long, default_value = "table")]
        format: String,
        /// Record the current 200-250 band as the size ratchet baseline (`GB-195`).
        ///
        /// Run deliberately and review the diff: every entry added is a file the sweep has not reached.
        #[arg(long)]
        write_size_ratchet: bool,
    },
    /// Analyze AST co-reference graphs and generate extraction blueprints.
    Slice {
        /// Target source file path to analyze.
        path: PathBuf,
        /// Target line count per extraction cluster (default: 250).
        #[arg(long, default_value_t = 250)]
        target_lines: usize,
        /// Output format (`human` or `json`).
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Extract token-compressed AST interface skeleton with implementation bodies stripped.
    Skeleton {
        /// Target source file path to extract.
        path: PathBuf,
        /// Output format (`text` or `json`).
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Inverted reverse-lookup of symbol references and call sites across the workspace.
    References {
        /// Target symbol name to find references for.
        symbol: String,
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Maximum number of reference records to return.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Output format (`table` or `json`).
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Multi-hop blast radius analysis and affected unit test selector.
    Impact {
        /// Target symbol name to analyze blast radius for.
        symbol: String,
        /// Maximum reverse traversal depth hops (default: 4).
        #[arg(long, default_value_t = 4)]
        depth: usize,
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Output format (`table` or `json`).
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Verify polyglot schema contracts across SQL, Rust, and TypeScript.
    Contracts {
        /// Database file to read (defaults to `<workspace_root>/.gaugeblock/miter/miter.redb`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Fail when any cross-language drift is detected.
        #[arg(long)]
        check: bool,
        /// Declared-non-mirror allowlist (default: `<path>/.gaugeblock/miter/contracts-allowlist.toml`).
        #[arg(long)]
        allowlist: Option<PathBuf>,
        /// Workspace root used to locate the default allowlist.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Output format (`table` or `json`).
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Detect deterministic AST structural clones and duplication candidates.
    Clones {
        /// Target path or directory to scan for structural clones (defaults to current dir).
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Minimum line count for function analysis (default: 15).
        #[arg(long, default_value_t = 15)]
        min_lines: usize,
        /// Minimum structural similarity threshold in 0.0..=1.0 (default: 0.90).
        #[arg(long, default_value_t = 0.90)]
        similarity: f32,
        /// Minimum identifier-overlap threshold in 0.0..=1.0 (default: 0.35).
        ///
        /// A pair must clear this as well as the structural threshold: two functions with the same
        /// control-flow skeleton but different contents are not duplication a reviewer would act on.
        #[arg(long, default_value_t = 0.35)]
        min_content: f32,
        /// Also analyse test code (excluded by default, because test bodies dominate the report).
        #[arg(long)]
        include_tests: bool,
        /// Output format (`table` or `json`).
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Map crate-to-crate import edges, declared-but-unused dependencies and proof of use.
    Deps(DepsArgs),
    /// Run isolated compiler verification on the workspace or crate.
    Verify {
        /// Target path to verify (defaults to current directory).
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Run in an ephemeral git worktree sandbox.
        #[arg(long)]
        sandbox: bool,
        /// Task ID for the sandbox.
        #[arg(long)]
        task_id: Option<String>,
        /// Timeout budget in milliseconds (default: 15,000ms).
        #[arg(long, default_value_t = 15_000)]
        timeout_ms: u64,
        /// Emit JSON instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
}
