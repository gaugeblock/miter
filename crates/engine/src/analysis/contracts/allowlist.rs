//! Declared non-mirrors (`GB-239`): fields that must not cross a language boundary.
//!
//! Some fields are absent from one representation on purpose. `role_override_config.api_key` exists in
//! Rust and must never exist in TypeScript - it is a secret, and the browser has no business receiving
//! it - so satisfying the comparator would be a security regression. Without a way to say that, the
//! only alternatives are a permanently ignored check or a blanket exclusion, and a blanket exclusion
//! would hide the drift that matters.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One field that is deliberately not mirrored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractException {
    /// Canonical entity name the field belongs to.
    pub entity: String,
    /// Field path, dotted when nested.
    pub field: String,
    /// Why it must not be mirrored. Required: an exception without a reason is a suppression.
    pub reason: String,
}

/// The declared non-mirrors.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractAllowlist {
    /// Declared exceptions.
    #[serde(default, rename = "exception")]
    pub exceptions: Vec<ContractException>,
}

impl ContractAllowlist {
    /// The exception covering `entity`/`field`, if one is declared.
    #[must_use]
    pub fn covers(&self, entity: &str, field: &str) -> Option<&ContractException> {
        self.exceptions
            .iter()
            .find(|entry| entry.entity == entity && entry.field == field)
    }
}

/// Reads the allowlist, treating an absent file as "no exceptions".
///
/// # Errors
///
/// A reasonless entry, or a file that exists but cannot be read or parsed: a broken or unexplained
/// allowlist must not silently pass the gate.
pub fn load(path: &Path) -> Result<ContractAllowlist, String> {
    if !path.exists() {
        return Ok(ContractAllowlist::default());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let parsed: ContractAllowlist =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(bare) = parsed
        .exceptions
        .iter()
        .find(|entry| entry.reason.trim().is_empty())
    {
        return Err(format!(
            "{}: exception for {}.{} carries no reason; an unexplained exception is a suppression",
            path.display(),
            bare.entity,
            bare.field
        ));
    }
    Ok(parsed)
}
