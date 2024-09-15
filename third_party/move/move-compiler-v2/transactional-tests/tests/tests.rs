// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use datatest_stable::Requirements;
use itertools::Itertools;
use move_command_line_common::env::read_bool_env_var;
use move_compiler_v2::{logging, Experiment};
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use once_cell::sync::Lazy;
use std::{path::Path, string::ToString};
use walkdir::WalkDir;

/// Tests containing this string in their path will skip v1-v2 comparison
const SKIP_V1_COMPARISON_PATH: &str = "/no-v1-comparison/";

fn move_test_debug() -> bool {
    static MOVE_TEST_DEBUG: Lazy<bool> = Lazy::new(|| read_bool_env_var("MOVE_TEST_DEBUG"));
    *MOVE_TEST_DEBUG
}

#[derive(Clone)]
struct TestConfig {
    name: &'static str,
    runner: fn(&Path) -> datatest_stable::Result<()>,
    experiments: &'static [(&'static str, bool)],
    language_version: LanguageVersion,
    /// Path substrings for tests to include. If empty, all tests are included.
    include: &'static [&'static str],
    /// Path substrings for tests to exclude (applied after the include filter).
    /// If empty, no additional tests are excluded.
    exclude: &'static [&'static str],
}

const TEST_CONFIGS: &[TestConfig] = &[
    TestConfig {
        name: "optimize",
        runner: |p| run(p, get_config_by_name("optimize")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
            (Experiment::ACQUIRES_CHECK, false),
        ],
        language_version: LanguageVersion::V2_0,
        include: &[], // all tests except those excluded below
        exclude: &["/operator_eval/"],
    },
    TestConfig {
        name: "no-optimize",
        runner: |p| run(p, get_config_by_name("no-optimize")),
        experiments: &[
            (Experiment::OPTIMIZE, false),
            (Experiment::ACQUIRES_CHECK, false),
        ],
        language_version: LanguageVersion::V2_0,
        include: &[], // all tests except those excluded below
        exclude: &["/operator_eval/"],
    },
    TestConfig {
        name: "optimize-no-simplify",
        runner: |p| run(p, get_config_by_name("optimize-no-simplify")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
            (Experiment::AST_SIMPLIFY, false),
            (Experiment::ACQUIRES_CHECK, false),
        ],
        language_version: LanguageVersion::V2_0,
        include: &[], // all tests except those excluded below
        exclude: &["/operator_eval/"],
    },
    TestConfig {
        name: "operator-eval-lang-1",
        runner: |p| run(p, get_config_by_name("operator-eval-lang-1")),
        experiments: &[(Experiment::OPTIMIZE, true)],
        language_version: LanguageVersion::V1,
        include: &["/operator_eval/"],
        exclude: &[],
    },
    TestConfig {
        name: "operator-eval-lang-2",
        runner: |p| run(p, get_config_by_name("operator-eval-lang-2")),
        experiments: &[(Experiment::OPTIMIZE, true)],
        language_version: LanguageVersion::V2_0,
        include: &["/operator_eval/"],
        exclude: &[],
    },
];

/// Test files which must use separate baselines because their result
/// is different.
const SEPARATE_BASELINE: &[&str] = &[
    // Runs into too-many-locals or stack overflow if not optimized
    "inlining/deep_exp.move",
    "constants/large_vectors.move",
    // Printing bytecode is different depending on optimizations
    "no-v1-comparison/print_bytecode.move",
    "bug_14243_stack_size.move",
    // The output of the tests could be different depending on the language version
    "/operator_eval/",
    // Creates different code if optimized or not
    "no-v1-comparison/enum/enum_field_select.move",
    "no-v1-comparison/enum/enum_field_select_different_offsets.move",
    "no-v1-comparison/assert_one.move",
    // Flaky redundant unused assignment error
    "no-v1-comparison/enum/enum_scoping.move",
];

fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("undeclared test config `{}`", name))
}

fn run(path: &Path, config: TestConfig) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing();
    let p = path.to_str().unwrap_or_default();
    let exp_suffix = if SEPARATE_BASELINE.iter().any(|s| p.contains(s)) {
        Some(format!("{}.exp", config.name))
    } else {
        None
    };
    let mut v2_experiments = config
        .experiments
        .iter()
        .map(|(s, v)| (s.to_string(), *v))
        .collect_vec();
    if path.to_string_lossy().contains("/access_control/") {
        // Enable access control file format generation for those tests
        v2_experiments.push((Experiment::GEN_ACCESS_SPECIFIERS.to_string(), true))
    }
    let language_version = config.language_version;
    let vm_test_config = if p.contains(SKIP_V1_COMPARISON_PATH) || move_test_debug() {
        TestRunConfig::CompilerV2 {
            language_version,
            v2_experiments,
        }
    } else {
        TestRunConfig::ComparisonV1V2 {
            language_version,
            v2_experiments,
        }
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
            if p.ends_with(".move") || p.ends_with(".mvir") {
                Some(p)
            } else {
                None
            }
        })
        .collect_vec();
    let reqs = TEST_CONFIGS
        .iter()
        .map(|config| {
            let pattern = files
                .iter()
                .filter(|file| {
                    (config.include.is_empty()
                        || config.include.iter().any(|include| file.contains(include)))
                        && (!config.exclude.iter().any(|exclude| file.contains(exclude)))
                })
                .map(|s| s.to_owned() + "$")
                .join("|");
            Requirements::new(
                config.runner,
                format!("compiler-v2-txn[config={}]", config.name),
                "tests".to_string(),
                pattern,
            )
        })
        .collect_vec();
    datatest_stable::runner(&reqs)
}
