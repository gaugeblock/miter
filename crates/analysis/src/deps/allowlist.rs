//! The layer allowlist (`GB-202`): named exceptions to the lateral-edge rule.
//!
//! An exception without an owner is just a suppression, so every entry carries the ticket that owes
//! the burn-down (`GB-204`). Phase B2 of `GB-200` finishes when this file is empty and the gate is
//! green without it - an edge leaves the allowlist only when its site count reaches zero, so partial
//! progress never silences the check.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One allowlisted lateral edge, carrying the ticket that will remove it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowlistEntry {
    /// Importing crate.
    pub from: String,
    /// Imported crate.
    pub to: String,
    /// Ticket that owes the burn-down, so an exception is always attributed debt.
    #[serde(default)]
    pub ticket: String,
}

/// The allowlist file contents.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerAllowlist {
    /// Permitted lateral edges.
    #[serde(default)]
    pub lateral: Vec<AllowlistEntry>,
}

impl LayerAllowlist {
    /// Returns the entry covering `from -> to`, if any.
    #[must_use]
    pub fn entry(&self, from: &str, to: &str) -> Option<&AllowlistEntry> {
        self.lateral
            .iter()
            .find(|item| item.from == from && item.to == to)
    }
}

/// Reads the allowlist, treating an absent file as "no exceptions".
///
/// # Errors
///
/// [`super::DepsError::Allowlist`] when the file exists but cannot be read or parsed: a broken
/// allowlist must not silently pass the gate.
pub fn load_allowlist(path: &Path) -> Result<LayerAllowlist, super::DepsError> {
    if !path.exists() {
        return Ok(LayerAllowlist::default());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| super::DepsError::Allowlist(format!("{}: {error}", path.display())))?;
    toml::from_str(&text)
        .map_err(|error| super::DepsError::Allowlist(format!("{}: {error}", path.display())))
}
