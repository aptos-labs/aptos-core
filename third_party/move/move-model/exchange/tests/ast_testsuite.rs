// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Baseline tests for the XAST format: each `.move` input under
//! `tests/ast_sources/` is exported by the `exchange` backend's AST producer
//! (`aptos_move_cli::exchange::move_file_to_ast`), and the pretty-printed
//! JSON — or the export error — is compared against the `.exp` baseline next
//! to the input.  Update baselines with `UB=1`.

use aptos_move_cli::exchange;
use move_prover_test_utils::baseline_test;
use std::path::Path;

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let output = match exchange::move_file_to_ast(path) {
        Ok(module) => {
            // The file table holds absolute paths; normalize them for a
            // location-independent baseline.
            let mut module = module;
            for source in &mut module.sources {
                if let Some(pos) = source.rfind('/') {
                    *source = source[pos + 1..].to_string();
                }
            }
            module.to_pretty_json()
        },
        Err(e) => format!("error: {:#}", e),
    };
    let baseline = path.with_extension("exp");
    baseline_test::verify_or_update_baseline(&baseline, &output)?;
    Ok(())
}

datatest_stable::harness!(test_runner, "tests/ast_sources", r".*\.move$");
