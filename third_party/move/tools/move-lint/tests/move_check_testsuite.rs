// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::{emit, termcolor::Buffer, Config};
use move_command_line_common::{
    env::read_bool_env_var,
    testing::{add_update_baseline_fix, format_diff, read_env_update_baseline, EXP_EXT, OUT_EXT},
};
use std::{fs, path::Path};
const KEEP_TMP: &str = "KEEP";

fn move_check_testsuite(path: &Path) -> datatest_stable::Result<()> {
    let exp_path = path.with_extension(EXP_EXT);
    let out_path = path.with_extension(OUT_EXT);

    run_test(path, &exp_path, &out_path)?;
    Ok(())
}

// Runs all tests under the test/testsuite directory.
pub fn run_test(path: &Path, exp_path: &Path, out_path: &Path) -> anyhow::Result<()> {
    let _ = run_test_inner(path, exp_path, out_path);
    Ok(())
}

// Runs all tests under the test/testsuite directory.
pub fn run_test_inner(path: &Path, exp_path: &Path, out_path: &Path) -> anyhow::Result<()> {
    let (diags, files) = move_lint::move_lint(path.to_path_buf());
    let has_diags = !diags.is_empty();
    let mut writer = Buffer::no_color();
    for diag in diags {
        let _ = emit(&mut writer, &Config::default(), &files, &diag);
    }
    let diag_buffer = writer.into_inner();
    let rendered_diags = std::str::from_utf8(&diag_buffer)?;
    fs::write(exp_path, rendered_diags)?;
    let save_diags = read_bool_env_var(KEEP_TMP);
    let update_baseline = read_env_update_baseline();
    if save_diags {
        fs::write(out_path, &diag_buffer)?;
    }
    if update_baseline {
        if has_diags {
            fs::write(exp_path, rendered_diags)?;
        } else if exp_path.is_file() {
            fs::remove_file(exp_path)?;
        }
        return Ok(());
    }

    let exp_exists = exp_path.is_file();
    match (has_diags, exp_exists) {
        (false, false) => Ok(()),
        (true, false) => {
            let msg = format!(
                "Expected success. Unexpected diagnostics:\n{}",
                rendered_diags
            );
            anyhow::bail!(add_update_baseline_fix(msg))
        },
        (false, true) => {
            let msg = format!(
                "Unexpected success. Expected diagnostics:\n{}",
                fs::read_to_string(exp_path)?
            );
            anyhow::bail!(add_update_baseline_fix(msg))
        },
        (true, true) => {
            let expected_diags = fs::read_to_string(exp_path)?;
            if rendered_diags != expected_diags {
                let msg = format!(
                    "Expected diagnostics differ from actual diagnostics:\n{}",
                    format_diff(expected_diags, rendered_diags),
                );
                anyhow::bail!(add_update_baseline_fix(msg))
            }
            Ok(())
        },
    }
}

datatest_stable::harness!(
    move_check_testsuite,
    "tests/cases/use_mul_div",
    r".*\.move$"
);
