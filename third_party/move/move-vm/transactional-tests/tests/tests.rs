// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use libtest_mimic::{Arguments, Trial};
use move_bytecode_verifier::{verifier::VerificationScope, VerifierConfig};
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use move_vm_runtime::config::VMConfig;
use once_cell::sync::Lazy;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    string::ToString,
};
use walkdir::WalkDir;

// TODO: Deduplicate with compiler-v2 transactional tests.
#[derive(Clone)]
struct TestConfig {
    name: &'static str,
    experiments: &'static [(&'static str, bool)],
    /// Run the tests with specified language version.
    language_version: LanguageVersion,
    /// VM configuration.
    vm_config: VMConfig,
    /// Path substrings for tests to include. If empty, all tests are included.
    include: &'static [&'static str],
    /// Path substrings for tests to exclude (applied after the include filter).
    /// If empty, no additional tests are excluded.
    exclude: &'static [&'static str],
    /// If true, delays runtime type checks to post-execution replay based on the collected trace.
    tracing: bool,
}

/// Note that any config which has different output for a test directory *must* be added to the
/// [SEPARATE_BASELINE] array below, so that a special output file `test.foo.exp` will be used for
/// outputs comparison when running `test.move` or `test.masm` for config "foo".
static TEST_CONFIGS: Lazy<Vec<TestConfig>> = Lazy::new(|| {
    vec![
        TestConfig {
            name: "baseline",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            vm_config: vm_config_for_tests(VerifierConfig::production()),
            include: &[],
            exclude: &["/paranoid-tests/", "/tracing/"],
            tracing: false,
        },
        TestConfig {
            name: "async-paranoid",
            experiments: &[("access-use-function-check", false)],
            language_version: LanguageVersion::latest(),
            // Verifier config is irrelevant here, because we disable verifier for these tests.
            // Importantly, paranoid checks are enabled.
            vm_config: vm_config_for_tests(
                VerifierConfig::unbounded().set_scope(VerificationScope::Nothing),
            ),
            include: &[
                // Note: for functions values the difference between generated files are stack
                // traces only (attached for in-place checks, set to None for async checks).
                "/limits/",
                "/function_values_safety/",
                "/paranoid-tests/",
                "/stack_size/",
                "/trusted_code/",
            ],
            exclude: &[],
            tracing: true,
        },
        TestConfig {
            name: "paranoid",
            experiments: &[("access-use-function-check", false)],
            language_version: LanguageVersion::latest(),
            // Verifier config is irrelevant here, because we disable verifier for these tests.
            // Importantly, paranoid checks are enabled.
            vm_config: vm_config_for_tests(
                VerifierConfig::unbounded().set_scope(VerificationScope::Nothing),
            ),
            include: &[
                "/limits/",
                "/function_values_safety/",
                "/paranoid-tests/",
                "/stack_size/",
                "/trusted_code/",
            ],
            exclude: &[],
            tracing: false,
        },
        TestConfig {
            name: "eager-loading",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            vm_config: VMConfig {
                verifier_config: VerifierConfig::production(),
                paranoid_type_checks: true,
                enable_lazy_loading: false,
                enable_enum_option: false,
                ..VMConfig::default_for_test()
            },
            include: &[],
            exclude: &[
                "/function_values_safety/",
                "/lazy_loading/",
                "/limits/",
                "/paranoid-tests/",
                "/runtime_ref_checks/",
                "/stack_size/",
                "/tracing/",
                "/trusted_code/",
                "/struct_api/",
            ],
            tracing: false,
        },
        // This config is used to test the runtime reference checker.
        TestConfig {
            name: "ref",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            // Verifier config is irrelevant here, because we disable verifier for these tests.
            // Importantly, paranoid checks and runtime ref checks are enabled.
            vm_config: vm_config_for_tests(
                VerifierConfig::unbounded().set_scope(VerificationScope::Nothing),
            )
            .set_paranoid_ref_checks(true),
            include: &["/runtime_ref_checks/"],
            exclude: &[],
            tracing: false,
        },
        TestConfig {
            name: "tracing",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            vm_config: vm_config_for_tests(VerifierConfig::production()),
            include: &["/tracing/"],
            exclude: &[],
            tracing: true,
        },
    ]
});

/// VM configuration used for testing.
/// By default, paranoid mode is always on.
fn vm_config_for_tests(verifier_config: VerifierConfig) -> VMConfig {
    VMConfig {
        paranoid_type_checks: true,
        optimize_trusted_code: true,
        verifier_config,
        enable_enum_option: false,
        ..VMConfig::default_for_test()
    }
}

/// Test files which must use separate baselines because their result is different.
///
/// Note that each config named "foo" above will compare the output of running `test.move` (or
/// `test.masm`) to the same baseline file `test.exp` *unless* there is an entry in this array
/// matching the path of `test.move` or `test.masm`. If there is such an entry, then each config
/// "foo" will have a separate baseline output file `test.foo.exp`.
const SEPARATE_BASELINE: &[&str] = &[
    "/function_values_safety/",
    "/limits/",
    "/module_publishing/",
    "/re_entrancy/",
    "/runtime_ref_checks/",
    "/stack_size/",
    "/trusted_code/",
];

/// Mapping of config names to canonical baseline names.
///
/// When a config name appears as a key here, the corresponding value (canonical baseline)
/// will be used instead. This allows multiple configs to share the same baseline file.
const SAME_BASELINE: &[(&str, &str)] = &[("async-paranoid", "paranoid")];

fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("undeclared test config `{}`", name))
}

/// Resolves the baseline config name for a given config.
///
/// If the config is in SAME_BASELINE, returns the canonical baseline name.
/// Otherwise, returns the original config name.
fn resolve_baseline_config(config_name: &str) -> &str {
    SAME_BASELINE
        .iter()
        .find(|(src, _)| *src == config_name)
        .map(|(_, canonical)| *canonical)
        .unwrap_or(config_name)
}

fn run(path: &Path, config: TestConfig) -> datatest_stable::Result<()> {
    let p = path.to_str().unwrap_or_default();
    let exp_suffix = if SEPARATE_BASELINE.iter().any(|s| p.contains(s)) {
        let baseline_config = resolve_baseline_config(config.name);
        Some(format!("{}.exp", baseline_config))
    } else {
        None
    };
    let experiments = config
        .experiments
        .iter()
        .map(|(s, v)| (s.to_string(), *v))
        .collect::<Vec<_>>();
    let vm_test_config = TestRunConfig {
        language_version: config.language_version,
        experiments,
        vm_config: config.vm_config,
        use_masm: true,
        echo: true,
        cross_compilation_targets: BTreeSet::new(),
        tracing: config.tracing,
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
            if p.ends_with(".move") || p.ends_with(".masm") {
                Some(p)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
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
                .map(move |file| {
                    let prompt = format!("move-vm-txn[config={}]::{}", config.name, file);
                    let path = PathBuf::from(file);
                    Trial::test(prompt, move || {
                        run(&path, get_config_by_name(config.name))
                            .map_err(|err| format!("{:?}", err).into())
                    })
                })
        })
        .collect::<Vec<_>>();
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
