// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Matches CHECK directives against VM outputs.

use crate::parser::{Check, MatchKind};
use anyhow::bail;

/// Verify that the outputs from both VMs match the expected checks.
/// `v2_gc_count` is the number of garbage collections the MonoMove VM ran for
/// this step, checked by `CHECK-GC-COUNT`.
pub fn check_output(
    checks: &[Check],
    v1_output: &str,
    v2_output: &str,
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
            Check::V1(expected, kind) => ("V1", expected.as_str(), *kind, v1_output),
            Check::V2(expected, kind) => ("V2", expected.as_str(), *kind, v2_output),
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
