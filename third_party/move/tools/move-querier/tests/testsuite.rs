// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use move_prover_test_utils::baseline_test;
use move_querier::querier::{Querier, QuerierOptions};
use std::{fs::read, path::Path};

/// Extension for expected output files
pub const EXP_EXT: &str = "cg.dot";
datatest_stable::harness!(test_runner, "tests", r".*\.mv$");

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let mut output = String::new();

    let querier_options = QuerierOptions::new(true, false);
    let bytecode_bytes = read(path)?;
    let querier = Querier::new(querier_options, bytecode_bytes);
    let res = querier.query()?;

    output += "\n";
    output += res.as_str();
    let baseline_path = path.with_extension(EXP_EXT);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &output)?;
    Ok(())
}
