//! Command line interface for autonomous agents and humans.
//!
//! ```text
//! miter index <PATH> [--db <DB_PATH>] [--dirty] [--json]
//! miter lookup <SYMBOL> [--context] [--json] [--max-tokens <N>] [--db <DB_PATH>]
//! miter skeleton <PATH> [--format text|json]
//! miter references <SYMBOL> [--db <DB_PATH>] [--limit <N>] [--format table|json]
//! miter impact <SYMBOL> [--depth <INT>] [--db <DB_PATH>] [--format table|json]
//! miter slice <PATH> [--target-lines <N>] [--format human|json]
//! miter audit [--path <PATH>] [--threshold <N>] [--format table|json]
//! miter contracts [--db <DB_PATH>] [--format table|json]
//! miter clones [--path <PATH>] [--min-lines <N>] [--similarity <F>] [--format table|json]
//! miter deps [--path <PATH>] [--why <CRATE>] [--unused] [--layers [--check]] [--format table|json]
//! miter verify [--path <PATH>] [--sandbox] [--task-id <ID>] [--timeout-ms <N>] [--json]
//! miter mcp [--db <DB_PATH>]
//! ```
//!
//! The binary lives in the root package (`src/main.rs`), so `cargo run -- index .` works
//! from the workspace root without `-p`.
//!
//! JSON output is hand-rolled ([`json`]) — no `serde` for the index surface.
//!
//! ## Layout
//!
//! This module is the facade: it owns the default constants and the two entry points ([`run`] and
//! [`execute`]), and re-exports the clap surface ([`args`]) and the error type ([`error`]) so a
//! caller keeps naming `crate::cli::Command` and `crate::cli::CliError`. Each subcommand's
//! *implementation* lives with its subject:
//!
//! | module | owns |
//! |---|---|
//! | [`args`] | the `Cli` parser and the `Command` subcommands (clap only) |
//! | [`error`] | `CliError` and its exit-code mapping |
//! | [`ingest`] | `index` — the command that writes an index |
//! | [`lookup`] | `lookup` — symbol hydration |
//! | [`skeleton`], [`references`], [`impact`], [`mod@slice`], [`audit`], [`contracts`], [`clones`], [`deps`] | one analysis command each |
//! | [`verify`] | the compiler verification command |
//! | [`mcp`] | the Model Context Protocol stdio server |
//! | [`json`] | the hand-rolled JSON writer the commands emit |

pub mod args;
pub mod args_deps;
pub mod audit;
mod audit_multi_repo;
mod audit_paths;
pub mod clones;
pub mod contracts;
pub mod deps;
pub mod deps_layers;
mod deps_layers_edit;
pub mod error;
pub mod impact;
pub mod ingest;
pub mod json;
pub mod lookup;
pub mod mcp;
pub mod references;
pub mod skeleton;
pub mod slice;
pub mod verify;

mod enclosing;
mod find;
mod find_report;
mod store;

use args::{Cli, Command};
pub use args_deps::DepsArgs;
pub use error::CliError;

use clap::Parser;

/// Resolves the workspace database path for one invocation.
///
/// An explicit `--db` wins. Otherwise the command works in the root of the current directory —
/// the nearest `.gaugeblock/` or `.git/` marker at or above it — and opens
/// `<root>/.gaugeblock/miter/miter.redb` (legacy `<root>/.miter/miter.redb` is still read).
/// There is one root and one database per invocation: nothing is
/// registered, attached or kept alive between runs.
#[must_use]
pub fn resolve_db(explicit: Option<std::path::PathBuf>) -> std::path::PathBuf {
    explicit.unwrap_or_else(|| {
        let start = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        miter_storage::default_db_path(&miter_storage::resolve_workspace_root(&start))
    })
}

/// Runs the CLI, returning the process exit code.
pub fn run() -> i32 {
    let cli = Cli::parse();
    match execute(cli) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("miter: {error}");
            1
        }
    }
}

/// Dispatches a parsed command.
pub fn execute(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Index {
            path,
            db,
            dirty,
            json,
        } => ingest::run_index(&path, &resolve_db(db), dirty, json),
        Command::Find {
            pattern,
            ignore_case,
            max,
            db,
            json,
        } => find::run_find(&pattern, &resolve_db(db), ignore_case, max, json),
        Command::Enclosing { target, db, json } => {
            enclosing::run_enclosing(&target, &resolve_db(db), json)
        }
        Command::Lookup {
            symbol,
            db,
            context,
            json,
            max_tokens,
        } => lookup::run_lookup(&symbol, &resolve_db(db), context, json, max_tokens),
        Command::Mcp { db } => mcp::run_mcp(resolve_db(db)),
        Command::Audit {
            paths,
            path,
            threshold,
            format,
        } => audit::run_audit(path, threshold, audit::AuditFormat::parse(&format), paths),
        Command::Slice {
            path,
            target_lines,
            format,
        } => slice::run_slice(&path, target_lines, slice::SliceFormat::parse(&format)),
        Command::Skeleton { path, format } => {
            skeleton::run_skeleton(&path, skeleton::SkeletonFormat::parse(&format))
        }
        Command::References {
            symbol,
            db,
            limit,
            format,
        } => references::run_references(
            &symbol,
            &resolve_db(db),
            limit,
            references::ReferencesFormat::parse(&format),
        ),
        Command::Impact {
            symbol,
            depth,
            db,
            format,
        } => impact::run_impact(
            &symbol,
            depth,
            &resolve_db(db),
            impact::ImpactFormat::parse(&format),
        ),
        Command::Contracts {
            db,
            check,
            allowlist,
            path,
            format,
        } => {
            let root = path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            let allowlist_path =
                allowlist.unwrap_or_else(|| deps_layers::contracts_allowlist(&root));
            contracts::run_contracts(
                &resolve_db(db),
                contracts::ContractsFormat::parse(&format),
                check,
                &allowlist_path,
            )
        }
        Command::Clones {
            path,
            min_lines,
            similarity,
            min_content,
            include_tests,
            format,
        } => clones::run_clones(
            &path,
            min_lines,
            similarity,
            min_content,
            include_tests,
            clones::ClonesFormat::parse(&format),
        ),
        Command::Deps(deps) => {
            let DepsArgs {
                path,
                why,
                unused,
                layers,
                check,
                allowlist,
                prune,
                adopt,
                ticket,
                format,
            } = deps;
            if layers || check || prune || adopt {
                deps_layers::run_deps_layers(
                    &path,
                    check,
                    allowlist.as_deref(),
                    deps_layers::AllowlistEdit {
                        prune,
                        adopt,
                        ticket: ticket.as_deref(),
                    },
                    &format,
                )
            } else {
                deps::run_deps(
                    &path,
                    why.as_deref(),
                    unused,
                    deps::DepsFormat::parse(&format),
                )
            }
        }
        Command::Verify {
            path,
            sandbox,
            task_id,
            timeout_ms,
            json,
        } => verify::run_verify(verify::VerifyOptions {
            path,
            sandbox,
            task_id,
            timeout_ms,
            json,
        }),
    }
}
