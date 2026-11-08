use super::{Pass, choose_pass};

#[test]
fn a_populated_baseline_is_re_read_rather_than_re_parsed() {
    // The regression this pins: the default pass used to re-parse the tree even when the database
    // described every file in it.
    assert_eq!(choose_pass(false, false, true), Pass::Incremental);
}

#[test]
fn an_empty_database_forces_the_full_pass() {
    // Nothing to compare against, so nothing may be skipped.
    assert_eq!(choose_pass(false, false, false), Pass::Full);
}

#[test]
fn the_flags_still_mean_what_they_say() {
    assert_eq!(choose_pass(true, false, true), Pass::Dirty);
    assert_eq!(choose_pass(true, false, false), Pass::Dirty);
    // `--full` overrides a populated baseline, which is the way out when the baseline is wrong.
    assert_eq!(choose_pass(false, true, true), Pass::Full);
    assert_eq!(choose_pass(false, true, false), Pass::Full);
}
