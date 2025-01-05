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
    /// Run the tests with specified language version.
    language_version: LanguageVersion,
    /// Path substrings for tests to include. If empty, all tests are included.
    include: &'static [&'static str],
    /// Path substrings for tests to exclude (applied after the include filter).
    /// If empty, no additional tests are excluded.
    exclude: &'static [&'static str],
}

/// Note that any config which has different output for a test directory
/// *must* be added to the `SEPARATE_BASELINE` array below, so that a
/// special output file `test.foo.exp` will be generated for the output
/// of `test.move` for config `foo`.
const TEST_CONFIGS: &[TestConfig] = &[
    // Matches all default experiments
    TestConfig {
        name: "baseline",
        runner: |p| run(p, get_config_by_name("baseline")),
        experiments: &[],
        language_version: LanguageVersion::latest_stable(),
        include: &[],
        exclude: &["/operator_eval/", "/access_control/"],
    },
    // Test optimize/no-optimize/etc., except for `/access_control/`
    TestConfig {
        name: "optimize",
        runner: |p| run(p, get_config_by_name("optimize")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
        ],
        language_version: LanguageVersion::latest_stable(),
        include: &[], // all tests except those excluded below
        exclude: &["/operator_eval/", "/access_control/"],
    },
    TestConfig {
        name: "no-optimize",
        runner: |p| run(p, get_config_by_name("no-optimize")),
        experiments: &[(Experiment::OPTIMIZE, false)],
        language_version: LanguageVersion::latest_stable(),
        include: &[], // all tests except those excluded below
        exclude: &["/operator_eval/", "/access_control/"],
    },
    TestConfig {
        name: "optimize-no-simplify",
        runner: |p| run(p, get_config_by_name("optimize-no-simplify")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
            (Experiment::AST_SIMPLIFY, false),
        ],
        language_version: LanguageVersion::latest_stable(),
        include: &[], // all tests except those excluded below
        exclude: &["/operator_eval/", "/access_control/"],
    },
    // Test `/operator_eval/` with language version 1 and 2
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
        language_version: LanguageVersion::latest_stable(),
        include: &["/operator_eval/"],
        exclude: &[],
    },
    // Test `/lambda/` with lambdas enabled
    TestConfig {
        name: "lambda",
        runner: |p| run(p, get_config_by_name("lambda")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::LAMBDA_FIELDS, true),
            (Experiment::LAMBDA_IN_PARAMS, true),
            (Experiment::LAMBDA_IN_RETURNS, true),
            (Experiment::LAMBDA_VALUES, true),
            (Experiment::LAMBDA_LIFTING, true),
        ],
        language_version: LanguageVersion::V2_2,
        include: &["/lambda/"],
        exclude: &[],
    },
    // Test optimize/no-optimize/etc., just for `/access_control/`, which
    // needs to disable `ACQUIRES_CHECK`.
    TestConfig {
        name: "optimize-no-acquires-check",
        runner: |p| run(p, get_config_by_name("optimize-no-acquires-check")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
            (Experiment::ACQUIRES_CHECK, false),
        ],
        language_version: LanguageVersion::latest_stable(),
        include: &["/access_control/"],
        exclude: &[],
    },
    TestConfig {
        name: "no-optimize-no-acquires-check",
        runner: |p| run(p, get_config_by_name("no-optimize-no-acquires-check")),
        experiments: &[
            (Experiment::OPTIMIZE, false),
            (Experiment::ACQUIRES_CHECK, false),
        ],
        language_version: LanguageVersion::latest_stable(),
        include: &["/access_control/"],
        exclude: &[],
    },
    TestConfig {
        name: "optimize-no-simplify-no-acquires-check",
        runner: |p| {
            run(
                p,
                get_config_by_name("optimize-no-simplify-no-acquires-check"),
            )
        },
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
            (Experiment::AST_SIMPLIFY, false),
            (Experiment::ACQUIRES_CHECK, false),
        ],
        language_version: LanguageVersion::latest_stable(),
        include: &["/access_control/"],
        exclude: &[],
    },
];

/// Test files which must use separate baselines because their result
/// is different.
///
/// Note that each config named "foo" above will compare the output of compiling `test.move` with
/// the same baseline file `test.exp` *unless* there is an entry in this array matching the path of
// `test.move`.  If there is such an entry, then each config "foo" will have a
/// separate baseline output file `test.foo.exp`.
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
    // Needs LAMBDA features and V2.2+ to function; baseline checks expected errors
    "/lambda/",
    // Needs ACQUIRES_CHECK disabled to function; baseline checks expected errors
    "/access_control/",
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
