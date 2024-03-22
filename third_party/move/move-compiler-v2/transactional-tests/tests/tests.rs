// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use datatest_stable::Requirements;
use itertools::Itertools;
use move_command_line_common::env::read_bool_env_var;
use move_compiler_v2::logging;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use once_cell::sync::Lazy;
use std::path::Path;
use walkdir::WalkDir;

/// Tests containing this string in their path will skip v1-v2 comparison
const SKIP_V1_COMPARISON_PATH: &str = "/no-v1-comparison/";

/// Experiment env var
const MOVE_COMPILER_EXP_VAR: &str = "MVC_EXP";

fn move_test_debug() -> bool {
    static MOVE_TEST_DEBUG: Lazy<bool> = Lazy::new(|| read_bool_env_var("MOVE_TEST_DEBUG"));
    *MOVE_TEST_DEBUG
}

#[derive(Clone)]
struct TestConfig {
    name: &'static str,
    runner: fn(&Path) -> datatest_stable::Result<()>,
    experiments: &'static str,
}

const TEST_CONFIGS: &[TestConfig] = &[
    TestConfig {
        name: "optimize",
        runner: |p| run(p, get_config_by_name("optimize")),
        experiments: "OPTIMIZE=on",
    },
    TestConfig {
        name: "no-optimize",
        runner: |p| run(p, get_config_by_name("no-optimize")),
        experiments: "OPTIMIZE=off",
    },
    TestConfig {
        name: "optimize-no-simplify",
        runner: |p| run(p, get_config_by_name("optimize-no-simplify")),
        experiments: "OPTIMIZE=on,AST_SIMPLIFY=off",
    },
];

/// Test files which must use separate baselines because their result
/// is different.
const SEPARATE_BASELINE: &[&str] = &[
    // Deep exp runs into too-many-locals if not optimized
    "inlining/deep_exp.move",
    // Printing bytecode is different depending on optimizations
    "no-v1-comparison/print_bytecode.move",
];

fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("undeclared test confio `{}`", name))
}

fn run(path: &Path, config: TestConfig) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing();
    let p = path.to_str().unwrap_or_default();
    let experiments = config.experiments.to_string();
    std::env::set_var(MOVE_COMPILER_EXP_VAR, experiments);
    let exp_suffix = if SEPARATE_BASELINE.iter().any(|s| p.contains(s)) {
        Some(format!("{}.exp", config.name))
    } else {
        None
    };
    let vm_test_config = if p.contains(SKIP_V1_COMPARISON_PATH) || move_test_debug() {
        TestRunConfig::CompilerV2
    } else {
        TestRunConfig::ComparisonV1V2
    };
    vm_test_harness::run_test_with_config_and_exp_suffix(vm_test_config, path, &exp_suffix)
}

fn main() {
    let files = WalkDir::new("tests")
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let p = e.path().display().to_string();
            if p.ends_with(".move") {
                Some(p)
            } else {
                None
            }
        })
        .collect_vec();
    let reqs = TEST_CONFIGS
        .iter()
        .map(|c| {
            Requirements::new(
                c.runner,
                format!("compiler-v2-txn[config={}]", c.name),
                "tests".to_string(),
                files.clone().into_iter().join("|"),
            )
        })
        .collect_vec();
    datatest_stable::runner(&reqs)
}
