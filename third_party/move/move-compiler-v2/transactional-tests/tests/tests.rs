// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use move_command_line_common::env::read_bool_env_var;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use once_cell::sync::Lazy;
use std::path::Path;

datatest_stable::harness!(run, TEST_DIR, r".*\.move$");

/// Tests containing this string in their path will skip v1-v2 comparison
const SKIP_V1_COMPARISON_PATH: &str = "/no-v1-comparison/";

/// Tests which should be run with the `no-safety` experiment on.
const NO_SAFETY_PATH: &str = "/no-safety/";

/// Experiment env var
const MOVE_COMPILER_EXP: &str = "MOVE_COMPILER_EXP";

fn move_test_debug() -> bool {
    static MOVE_TEST_DEBUG: Lazy<bool> = Lazy::new(|| read_bool_env_var("MOVE_TEST_DEBUG"));
    *MOVE_TEST_DEBUG
}

fn run(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let p = path.to_str().unwrap_or_default();
    if p.contains(NO_SAFETY_PATH) {
        std::env::set_var(MOVE_COMPILER_EXP, "no-safety")
    }
    let test_config = if p.contains(SKIP_V1_COMPARISON_PATH) || move_test_debug() {
        TestRunConfig::CompilerV2
    } else {
        TestRunConfig::ComparisonV1V2
    };
    vm_test_harness::run_test_with_config(test_config, path)
}
