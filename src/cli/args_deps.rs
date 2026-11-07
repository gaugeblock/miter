//! Arguments for `miter deps` (`GB-248`).
//!
//! A struct rather than an enum variant body: the command accumulated enough switches that its fields alone
//! pushed `args.rs` past the 250-line limit, and an argument group with its own maintenance verbs (`--prune`,
//! `--adopt`) reads better as a unit.

use clap::Args;
use std::path::PathBuf;

/// `miter deps`: who imports whom, how that maps onto layers, and how the allowlist is maintained.
#[derive(Args, Debug, Clone)]
pub struct DepsArgs {
    /// Workspace root to analyse (defaults to current dir).
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    /// Show every import site that proves use of this crate.
    #[arg(long)]
    pub why: Option<String>,
    /// Only report declared dependencies that no source file imports.
    #[arg(long)]
    pub unused: bool,
    /// Classify every edge as foundation / domain / integrator instead of listing edges.
    #[arg(long)]
    pub layers: bool,
    /// Fail when a lateral edge is unallowlisted or an edge points at a higher layer.
    #[arg(long)]
    pub check: bool,
    /// Layer allowlist path (default: `<path>/.gaugeblock/miter/layers-allowlist.toml`).
    #[arg(long)]
    pub allowlist: Option<PathBuf>,
    /// Delete allowlist entries that match no current edge.
    #[arg(long)]
    pub prune: bool,
    /// Write an allowlist entry for every unallowlisted lateral edge.
    #[arg(long)]
    pub adopt: bool,
    /// Ticket recorded on adopted entries (required with `--adopt`).
    #[arg(long)]
    pub ticket: Option<String>,
    /// Output format (`table` or `json`).
    #[arg(long, default_value = "table")]
    pub format: String,
}
