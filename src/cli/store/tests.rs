//! Index loading: the friendly error, and the variable that turns auto-indexing off.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::path::PathBuf;

#[test]
fn loading_a_missing_database_is_a_friendly_error() {
    // `/tmp/x.redb` has no three-level ancestor, so no root can be derived and the build cannot start:
    // the caller gets the instruction instead of an index of the filesystem root.
    let path = PathBuf::from("/tmp/miter_never_written_9f3a.redb");
    let error = load(&path).expect_err("an unbuildable index is an error");
    assert!(matches!(error, CliError::MissingIndex(_)));
    assert!(error.to_string().contains("miter index"), "{error}");
}

#[test]
fn the_opt_out_variable_is_spelled_like_the_tool() {
    // It read `MITTER_NO_AUTO_INDEX` until 2026-11-07: a name nobody guesses correctly, and one that
    // punishes a reader who does. Two assertions, because either half alone would pass on a rename to
    // something equally wrong.
    assert!(
        NO_AUTO_INDEX_ENV.starts_with("MITER_"),
        "the variable must be prefixed with the tool's name, one T: {NO_AUTO_INDEX_ENV}"
    );
    assert!(
        !NO_AUTO_INDEX_ENV.starts_with("MITTER_"),
        "MITTER_ is a misspelling of miter: {NO_AUTO_INDEX_ENV}"
    );
}
