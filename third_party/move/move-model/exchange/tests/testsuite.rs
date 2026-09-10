// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Baseline tests for the exchange format: each `.masm` or `.move` input
//! under `tests/sources/` is translated by the `exchange` backend
//! (`aptos_move_cli::exchange`), and the pretty-printed JSON — or the
//! translation error — is compared against the `.exp` baseline next to the
//! input.  Update baselines with `UB=1`.

use aptos_move_cli::exchange;
use move_prover_test_utils::baseline_test;
use std::path::Path;

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let ext = path
        .extension()
        .expect("harness only matches .masm/.move")
        .to_string_lossy();
    let result = if ext == "move" {
        exchange::move_file_to_module(path)
    } else {
        exchange::masm_to_module(&std::fs::read_to_string(path)?)
    };
    let output = match result {
        Ok(module) => module.to_pretty_json(),
        Err(e) => format!("error: {:#}", e),
    };
    // Keep the source extension in the baseline name (`foo.masm.exp`), so
    // a `.masm` and a `.move` input may share a stem.
    let baseline = path.with_extension(format!("{}.exp", ext));
    baseline_test::verify_or_update_baseline(&baseline, &output)?;
    Ok(())
}

datatest_stable::harness!(test_runner, "tests/sources", r".*\.(masm|move)$");
