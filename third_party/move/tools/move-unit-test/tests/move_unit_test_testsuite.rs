// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_command_line_common::testing::{
    add_update_baseline_fix, format_diff, get_compiler_exp_extension, read_env_update_baseline,
};
use move_unit_test::{self, test_reporter::UnitTestFactoryWithCostTable, UnitTestingConfig};
use regex::RegexBuilder;
use std::{
    fs,
    path::{Path, PathBuf},
};

// We don't support statistics tests as that includes times which are variable and will make these
// tests flaky.
const TEST_MODIFIER_STRS: &[&str] = &["storage"];

pub fn modify(mut base_config: UnitTestingConfig, modifier_str: &str) -> Option<UnitTestingConfig> {
    // Add future test modifiers here
    match modifier_str {
        "storage" => base_config.report_storage_on_error = true,
        _ => return None,
    };
    Some(base_config)
}

fn run_test_with_modifiers(
    unit_test_config: UnitTestingConfig,
    path: &Path,
) -> anyhow::Result<Vec<((Vec<u8>, bool), PathBuf)>> {
    let mut results = Vec::new();

    let exp_ext = get_compiler_exp_extension();
    for modifier in TEST_MODIFIER_STRS.iter() {
        let modified_exp_path = path.with_extension(format!("{}.{}", modifier, exp_ext));
        if let (Some(test_config), true) = (
            modify(unit_test_config.clone(), modifier),
            modified_exp_path.exists(),
        ) {
            let buffer = Vec::new();
            let test_plan = test_config.build_test_plan();
            if test_plan.is_none() {
                anyhow::bail!(
                    "No test plan constructed for {:?} with modifier {}",
                    path,
                    modifier
                );
            }

            results.push((
                test_config.run_and_report_unit_tests(
                    test_plan.unwrap(),
                    None,
                    None,
                    buffer,
                    UnitTestFactoryWithCostTable::new(None, None),
                )?,
                modified_exp_path,
            ))
        }
    }

    // Now run with no modifiers
    let buffer = Vec::new();
    let test_plan = unit_test_config.build_test_plan();
    if test_plan.is_none() {
        anyhow::bail!("No test plan constructed for {:?}", path);
    }

    results.push((
        unit_test_config.run_and_report_unit_tests(
            test_plan.unwrap(),
            None,
            None,
            buffer,
            UnitTestFactoryWithCostTable::new(None, None),
        )?,
        path.with_extension(exp_ext),
    ));

    Ok(results)
}

// Runs all tests under the test/test_sources directory.
fn run_test_impl(path: &Path) -> anyhow::Result<()> {
    std::env::set_var("NO_COLOR", "1");
    let update_baseline = read_env_update_baseline();
    let source_files = vec![path.to_str().unwrap().to_owned()];
    let unit_test_config = UnitTestingConfig {
        num_threads: 1,
        source_files,
        dep_files: move_stdlib::move_stdlib_files(),
        named_address_values: move_stdlib::move_stdlib_named_addresses()
            .into_iter()
            .collect(),
        verbose: true,
        report_stacktrace_on_abort: true,

        ..UnitTestingConfig::default()
    };

    let regex = RegexBuilder::new(r"(┌─ ).+/([^/]+)$")
        .multi_line(true)
        .build()
        .unwrap();

    for ((buffer, _), exp_path) in run_test_with_modifiers(unit_test_config, path)? {
        let base_output = String::from_utf8(buffer)?;
        let cleaned_output = regex.replacen(&base_output, 0, r"$1$2");
        if update_baseline {
            fs::write(&exp_path, &*cleaned_output)?
        }

        let exp_exists = exp_path.is_file();

        if exp_exists {
            let expected = fs::read_to_string(&exp_path)?;
            if expected != cleaned_output {
                let msg = format!(
                    "Expected outputs differ for {:?}:\n{}",
                    exp_path,
                    format_diff(expected, cleaned_output)
                );
                anyhow::bail!(add_update_baseline_fix(msg));
            }
        } else {
            let msg = format!("No expected output found for {:?}", path);
            anyhow::bail!(add_update_baseline_fix(msg));
        }
    }

    Ok(())
}

fn run_test(path: &Path) -> datatest_stable::Result<()> {
    run_test_impl(path)?;
    Ok(())
}

datatest_stable::harness!(run_test, "tests/test_sources", r".*\.move$");
