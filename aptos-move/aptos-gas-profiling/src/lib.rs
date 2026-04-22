// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod aggregate;
mod erased;
mod flamegraph;
mod log;
mod misc;
mod profiler;
mod render;
mod report;
mod unique_stack;

pub use log::{ConsistencyError, ConsistencyErrorKind, FrameName, TransactionGasLog};
pub use profiler::GasProfiler;
pub use report::HtmlReportOptions;

/// Runs the gas profiler's consistency checks on `log` and reports any
/// discrepancies to the user.
///
/// Consistency errors always indicate a bug in the gas profiler itself, not in
/// the transaction being profiled or in the gas meter. This helper centralizes
/// the user-facing messaging so every CLI entry point reports the issue the
/// same way (and surfaces the same opt-out flag).
///
/// - If `skip_consistency_check` is `true`, any inconsistency is reported as a
///   warning on stderr and the generated gas report (potentially incomplete)
///   is preserved.
/// - Otherwise, the first inconsistency causes a `panic!` so the caller fails
///   loudly.
pub fn warn_or_panic_on_inconsistency(log: &TransactionGasLog, skip_consistency_check: bool) {
    let errors = [
        log.exec_io.check_consistency(),
        log.storage.check_consistency(),
    ];
    for err in errors.into_iter().filter_map(Result::err) {
        if skip_consistency_check {
            eprintln!(
                "warning: {}\n\
                 (consistency check was bypassed via --skip-gas-profiler-consistency-check; \
                 the generated gas report may be incomplete or inaccurate.)",
                err
            );
        } else {
            panic!(
                "{}\n\nRerun with --skip-gas-profiler-consistency-check to bypass this \
                 check and still produce a (possibly incomplete) gas report.",
                err
            );
        }
    }
}
