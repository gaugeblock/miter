//! `miter` binary entry point.
//!
//! All logic lives in the library ([`miter::cli`]) so that the subcommands are covered by
//! the crate's tests; this shim only maps the result to a process exit code.

fn main() {
    // One span per invocation (`GB-339`). The shim is the ideal place for it: `run()` returns a process code rather
    // than a `Result`, so the whole command sits between open and finish with nowhere for an early return to escape.
    let command = std::env::args().nth(1).filter(|arg| !arg.starts_with('-'));
    let invocation = gaugeblock_workspace::InvocationSpan::start("miter", command.as_deref());

    let code = miter::cli::run();

    // Finished explicitly, before `exit`, because `std::process::exit` does not run destructors - a span closed by
    // `Drop` in this binary would never reach the log at all.
    invocation.finish(code == 0);
    std::process::exit(code);
}
