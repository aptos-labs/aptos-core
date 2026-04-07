// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Matches CHECK directives against VM outputs.

use crate::parser::Check;
use anyhow::bail;

/// Verify that the outputs from both VMs match the expected checks.
pub fn check_output(checks: &[Check], v1_output: &str, v2_output: &str) -> anyhow::Result<()> {
    for check in checks {
        let (label, expected, actual) = match check {
            Check::V1(expected) => ("V1", expected.as_str(), v1_output),
            Check::V2(expected) => ("V2", expected.as_str(), v2_output),
        };
        if actual != expected {
            bail!(
                "CHECK-{} mismatch:\n  expected: {}\n  actual:   {}",
                label,
                expected,
                actual
            );
        }
    }
    Ok(())
}
