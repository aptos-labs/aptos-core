// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod generate;
pub mod simulate;
pub mod verify;
pub mod verify_framework_deployment;

/// Collapse a non-empty list of errors into a single aggregated error.
pub(crate) fn combine_errors(label: &str, errors: &[String]) -> anyhow::Error {
    let mut msg = format!("{} found {} error(s):\n", label, errors.len());
    for error in errors {
        msg.push_str(&format!("  - {}\n", error));
    }
    anyhow::anyhow!(msg)
}
