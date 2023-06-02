// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::termcolor::Buffer;
use move_compiler_v2::Options;
use move_prover_test_utils::{baseline_test, extract_test_directives};
use std::path::{Path, PathBuf};

/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let mut experiments = extract_test_directives(path, "// experiment:")?;
    if experiments.is_empty() {
        // If there is no experiment, use "" as the 'default' experiment.
        experiments.push("".to_string()) // default experiment
    }
    let mut sources = extract_test_directives(path, "// dep:")?;
    sources.push(path.to_string_lossy().to_string());
    let deps = vec![path_from_crate_root("../move-stdlib/sources")];

    // For each experiment, run the compiler.
    for exp in experiments {
        // Construct options, compiler and collect output.
        let mut options = Options {
            testing: true,
            sources: sources.clone(),
            dependencies: deps.clone(),
            named_address_mapping: vec!["std=0x1".to_string()],
            ..Options::default()
        };
        let file_ext = if exp.is_empty() {
            EXP_EXT.to_string()
        } else {
            options.experiments.push(exp.clone());
            format!("{}.{}", EXP_EXT, exp)
        };

        let mut error_writer = Buffer::no_color();
        let mut out = match move_compiler_v2::run_move_compiler(&mut error_writer, options) {
            Ok(_) => "succeeded".to_owned(),
            Err(e) => format!("failed: {}", e),
        };
        let diag = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
        if !diag.is_empty() {
            out = format!("{}\nDiagnostics:\n{}", out, diag)
        }

        // Generate/check baseline.
        let baseline_path = path.with_extension(file_ext);
        baseline_test::verify_or_update_baseline(baseline_path.as_path(), &out)?;
    }
    Ok(())
}

fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}

datatest_stable::harness!(test_runner, "tests", r".*\.move$");
