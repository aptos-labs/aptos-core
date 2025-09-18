// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use itertools::Itertools;
use libtest_mimic::{Arguments, Trial};
use move_compiler_v2::{logging, Experiment};
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::{
    tasks::SyntaxChoice, vm_test_harness, vm_test_harness::TestRunConfig,
};
use std::{
    path::{Path, PathBuf},
    string::ToString,
};
use walkdir::WalkDir;

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
    /// Cross compile or not
    cross_compile: bool,
}

/// Set of exclusions that apply when using `include: &[]` in TestConfig.
const COMMON_EXCLUSIONS: &[&str] = &[
    "/operator_eval/",
    "/no-recursive-check/",
    "/no-access-check/",
    "/no-recursive-type-check/",
];

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
        language_version: LanguageVersion::latest(),
        include: &[], // all tests except those excluded below
        exclude: COMMON_EXCLUSIONS,
        cross_compile: true,
    },
    // Test optimize/no-optimize/etc., except for `/access_control/`
    TestConfig {
        name: "optimize",
        runner: |p| run(p, get_config_by_name("optimize")),
        experiments: &[
            (Experiment::OPTIMIZE, true),
            (Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true),
        ],
        language_version: LanguageVersion::latest(),
        include: &[], // all tests except those excluded below
        exclude: COMMON_EXCLUSIONS,
        cross_compile: false,
    },
    TestConfig {
        name: "no-optimize",
        runner: |p| run(p, get_config_by_name("no-optimize")),
        experiments: &[(Experiment::OPTIMIZE, false)],
        language_version: LanguageVersion::latest(),
        include: &[], // all tests except those excluded below
        exclude: COMMON_EXCLUSIONS,
        cross_compile: false,
    },
    // Test `/operator_eval/` with language version 1 and 2
    TestConfig {
        name: "operator-eval-lang-1",
        runner: |p| run(p, get_config_by_name("operator-eval-lang-1")),
        experiments: &[(Experiment::OPTIMIZE, true)],
        language_version: LanguageVersion::V1,
        include: &["/operator_eval/"],
        exclude: &[],
        cross_compile: false,
    },
    TestConfig {
        name: "operator-eval-lang-2",
        runner: |p| run(p, get_config_by_name("operator-eval-lang-2")),
        experiments: &[(Experiment::OPTIMIZE, true)],
        language_version: LanguageVersion::latest(),
        include: &["/operator_eval/"],
        exclude: &[],
        cross_compile: true,
    },
    TestConfig {
        name: "no-recursive-check",
        runner: |p| run(p, get_config_by_name("no-recursive-check")),
        experiments: &[(Experiment::RECURSIVE_TYPE_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &["/no-recursive-check/"],
        exclude: &[],
        cross_compile: false,
    },
    TestConfig {
        name: "no-access-check",
        runner: |p| run(p, get_config_by_name("no-access-check")),
        experiments: &[(Experiment::ACCESS_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &["/no-access-check/"],
        exclude: &[],
        cross_compile: false,
    },
    TestConfig {
        name: "no-recursive-type-check",
        runner: |p| run(p, get_config_by_name("no-recursive-type-check")),
        experiments: &[(Experiment::RECURSIVE_TYPE_CHECK, false)],
        language_version: LanguageVersion::latest(),
        include: &["/no-recursive-type-check/"],
        exclude: &[],
        cross_compile: false,
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
    "no-v1-comparison/closures/reentrancy",
    "no-v1-comparison/closures/reentrancy",
    "control_flow/for_loop_non_terminating.move",
    "control_flow/for_loop_nested_break.move",
    "evaluation_order/lazy_assert.move",
    "evaluation_order/short_circuiting_invalid.move",
    "evaluation_order/struct_arguments.move",
    "inlining/bug_11223.move",
    "misc/build_with_warnings.move",
    "optimization/bug_14223_unused_non_droppable.move",
    // Flaky redundant unused assignment error
    "no-v1-comparison/enum/enum_scoping.move",
    // Different error messages depending on optimizations or not
    "no-v1-comparison/fv_as_keys.move",
];

fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("undeclared test config `{}`", name))
}

fn run(path: &Path, config: TestConfig) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing(None);
    let p = path.to_str().unwrap_or_default();
    let exp_suffix = if SEPARATE_BASELINE.iter().any(|s| p.contains(s)) {
        Some(format!("{}.exp", config.name))
    } else {
        None
    };
    let experiments = config
        .experiments
        .iter()
        .map(|(s, v)| (s.to_string(), *v))
        .collect_vec();
    let language_version = config.language_version;
    let mut vm_test_config =
        TestRunConfig::new(language_version, experiments).with_runtime_ref_checks();
    // For cross compilation, we need to always append the config name as a part of the
    // outcome file suffix, as optimizations affect the generated code.
    if config.cross_compile {
        vm_test_config = vm_test_config.cross_compile_into(
            SyntaxChoice::Source,
            true,
            exp_suffix
                .clone()
                .or_else(|| Some(format!("{}.exp", config.name))),
        );
    }

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
    let mut tests = TEST_CONFIGS
        .iter()
        .flat_map(|config| {
            files
                .iter()
                .filter(|file| {
                    (config.include.is_empty()
                        || config.include.iter().any(|include| file.contains(include)))
                        && (!config.exclude.iter().any(|exclude| file.contains(exclude)))
                })
                .map(|file| {
                    let prompt = format!("compiler-v2-txn[config={}]::{}", config.name, file);
                    let path = PathBuf::from(file);
                    let runner = config.runner;
                    Trial::test(prompt, move || {
                        runner(&path).map_err(|err| format!("{:?}", err).into())
                    })
                })
        })
        .collect_vec();
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
