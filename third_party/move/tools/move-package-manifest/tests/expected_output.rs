// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Write, path::Path};

datatest_stable::harness!(expected_output_tests, "tests", r".*\.toml$");

fn expected_output_tests(path: &Path) -> datatest_stable::Result<()> {
    let content = std::fs::read_to_string(path)?;

    let parse_result = move_package_manifest::parse_package_manifest(&content);

    let mut output = String::new();
    match parse_result {
        Ok(parsed_manifest) => {
            writeln!(output, "success")?;
            writeln!(output)?;
            writeln!(output, "{:#?}", parsed_manifest)?;
        },
        Err(err) => {
            move_package_manifest::render_error(&mut output, &content, &err)?;
        },
    }

    move_prover_test_utils::baseline_test::verify_or_update_baseline(
        &path.with_extension("exp"),
        &output,
    )?;

    Ok(())
}
