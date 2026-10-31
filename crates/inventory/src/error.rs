//! Error definitions for workspace inventory extraction.

use thiserror::Error;

/// Errors that can occur during workspace inspection and inventory extraction.
#[derive(Debug, Error)]
pub enum InventoryError {
    /// File or directory I/O failure.
    #[error("inventory I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML manifest deserialization error.
    #[error("manifest TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Missing or malformed workspace structure.
    #[error("invalid workspace layout: {0}")]
    InvalidWorkspace(String),

    /// Git command or metadata extraction error.
    #[error("git metadata error: {0}")]
    Git(String),

    /// CLI argument or AST parsing error.
    #[error("CLI or AST parsing error: {0}")]
    Parse(String),
}
