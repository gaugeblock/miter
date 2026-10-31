//! Conversions between this engine's language table and the suite's wire `Language`.
//!
//! `PolyglotLanguage` is the internal taxonomy: thirteen formats, no serde derives, owning the
//! `name()` spellings. `gaugeblock_contracts::Language` is the wire view of the same thirteen
//! formats. The two are total, mutually invertible, and kept in lockstep by a test that walks
//! every variant, so adding a format to one without the other fails the suite.

use gaugeblock_contracts::Language as WireLanguage;

use super::language::PolyglotLanguage;

impl From<PolyglotLanguage> for WireLanguage {
    fn from(language: PolyglotLanguage) -> Self {
        match language {
            PolyglotLanguage::Rust => Self::Rust,
            PolyglotLanguage::TypeScript => Self::TypeScript,
            PolyglotLanguage::Tsx => Self::Tsx,
            PolyglotLanguage::JavaScript => Self::JavaScript,
            PolyglotLanguage::Python => Self::Python,
            PolyglotLanguage::Sql => Self::Sql,
            PolyglotLanguage::Html => Self::Html,
            PolyglotLanguage::Css => Self::Css,
            PolyglotLanguage::Markdown => Self::Markdown,
            PolyglotLanguage::Dockerfile => Self::Dockerfile,
            PolyglotLanguage::Yaml => Self::Yaml,
            PolyglotLanguage::Hcl => Self::Hcl,
            PolyglotLanguage::Env => Self::Env,
        }
    }
}

impl From<WireLanguage> for PolyglotLanguage {
    fn from(language: WireLanguage) -> Self {
        match language {
            WireLanguage::Rust => Self::Rust,
            WireLanguage::TypeScript => Self::TypeScript,
            WireLanguage::Tsx => Self::Tsx,
            WireLanguage::JavaScript => Self::JavaScript,
            WireLanguage::Python => Self::Python,
            WireLanguage::Sql => Self::Sql,
            WireLanguage::Html => Self::Html,
            WireLanguage::Css => Self::Css,
            WireLanguage::Markdown => Self::Markdown,
            WireLanguage::Dockerfile => Self::Dockerfile,
            WireLanguage::Yaml => Self::Yaml,
            WireLanguage::Hcl => Self::Hcl,
            WireLanguage::Env => Self::Env,
        }
    }
}
