// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Matches CHECK directives against VM outputs.

use crate::{
    parser::{Check, MatchKind},
    runner::{Output, ParityOutcome},
};
use anyhow::bail;

/// Verify that the outputs from both VMs match the expected checks.
/// `v2_gc_count` is the number of garbage collections the MonoMove VM ran for
/// this step, checked by `CHECK-GC-COUNT`.
pub(crate) fn check_output(
    checks: &[Check],
    v1: &Output,
    v2: &Output,
    v2_gc_count: usize,
) -> anyhow::Result<()> {
    for check in checks {
        let (label, expected, kind, actual) = match check {
            Check::GcCount(expected) => {
                if v2_gc_count != *expected {
                    bail!(
                        "CHECK-GC-COUNT mismatch (V2):\n  expected: {}\n  actual:   {}",
                        expected,
                        v2_gc_count,
                    );
                }
                continue;
            },
            Check::ErrorParity => {
                check_error_parity(v1, v2)?;
                continue;
            },
            Check::V1(expected, kind) => ("V1", expected.as_str(), *kind, v1.display.as_str()),
            Check::V2(expected, kind) => ("V2", expected.as_str(), *kind, v2.display.as_str()),
        };
        let actual = actual.trim_end();
        let expected_trimmed = expected.trim_end();
        let matched = match kind {
            MatchKind::Exact => actual == expected_trimmed,
            MatchKind::Substring => actual.contains(expected_trimmed),
        };
        if !matched {
            let label_suffix = match kind {
                MatchKind::Exact => "",
                MatchKind::Substring => "-SUBSTR",
            };
            bail!(
                "CHECK-{}{} mismatch:\n  expected: {}\n  actual:   {}",
                label,
                label_suffix,
                expected,
                actual,
            );
        }
    }
    Ok(())
}

/// Asserts MonoMove's failure, mapped into V1 terms, matches the one V1 reported.
fn check_error_parity(v1: &Output, v2: &Output) -> anyhow::Result<()> {
    let expected = match &v1.parity {
        ParityOutcome::Comparable(expected) => expected,
        // A Move abort is not a VM error, so it never reaches the mapping this
        // directive checks. Both VMs render aborts identically, so `CHECK:`
        // already compares the code, message, and location.
        ParityOutcome::Aborted => bail!(
            "CHECK-ERROR-PARITY does not cover Move aborts, use CHECK: instead:\n  V1: {}",
            v1.display,
        ),
        // V1 is the reference: it defines the expected value, so it has to fail
        // for there to be one.
        ParityOutcome::NoFailure => bail!(
            "CHECK-ERROR-PARITY: V1 did not fail, so there is no error to match:\n  V1: {}",
            v1.display,
        ),
        ParityOutcome::Unmappable => unreachable!("V1 states its own errors"),
    };
    match &v2.parity {
        ParityOutcome::Comparable(actual) if actual == expected => Ok(()),
        ParityOutcome::Comparable(actual) => bail!(
            "CHECK-ERROR-PARITY mismatch:\n  V1: {}\n  V2: {}",
            expected,
            actual,
        ),
        ParityOutcome::Aborted => bail!(
            "CHECK-ERROR-PARITY: V2 aborted where V1 raised an error:\n  V1: {}\n  V2: {}",
            expected,
            v2.display,
        ),
        ParityOutcome::NoFailure => bail!(
            "CHECK-ERROR-PARITY: V2 did not fail, but V1 did:\n  V1: {}\n  V2: {}",
            expected,
            v2.display,
        ),
        ParityOutcome::Unmappable => bail!(
            "CHECK-ERROR-PARITY: V2 cannot state what V1 reports for its failure:\n  \
             V1: {}\n  V2: {}",
            expected,
            v2.display,
        ),
    }
}
