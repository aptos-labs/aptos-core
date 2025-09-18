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
}

/// Note that any config which has different output for a test directory *must* be added to the
/// [SEPARATE_BASELINE] array below, so that a special output file `test.foo.exp` will be used for
/// outputs comparison when running `test.move` or `test.mvir` for config "foo".
static TEST_CONFIGS: Lazy<Vec<TestConfig>> = Lazy::new(|| {
    vec![
        TestConfig {
            name: "baseline",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            vm_config: vm_config_for_tests(VerifierConfig::production()),
            include: &[],
            exclude: &["/paranoid-tests/"],
        },
        TestConfig {
            name: "paranoid-mode-only",
            experiments: &[("access-use-function-check", false)],
            language_version: LanguageVersion::latest(),
            // Verifier config is irrelevant here, because we disable verifier for these tests.
            // Importantly, paranoid checks are enabled.
            vm_config: vm_config_for_tests(
                VerifierConfig::unbounded().set_scope(VerificationScope::Nothing),
            ),
            include: &["/paranoid-tests/"],
            exclude: &[],
        },
        TestConfig {
            name: "paranoid",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            // Verifier config is irrelevant here, because we disable verifier for these tests.
            // Importantly, paranoid checks are enabled.
            vm_config: vm_config_for_tests(
                VerifierConfig::unbounded().set_scope(VerificationScope::Nothing),
            ),
            include: &["/function_values_safety/", "/trusted_code/"],
            exclude: &[],
        },
        TestConfig {
            name: "eager-loading",
            experiments: &[],
            language_version: LanguageVersion::latest(),
            vm_config: VMConfig {
                verifier_config: VerifierConfig::production(),
                paranoid_type_checks: true,
                enable_lazy_loading: false,
                ..VMConfig::default()
            },
            include: &[],
            exclude: &[
                "/lazy_loading/",
                "/paranoid-tests/",
                "/function_values_safety/",
                "/trusted_code/",
                "/runtime_ref_checks/",
            ],
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
        ..VMConfig::default()
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
    "/module_publishing/",
    "/re_entrancy/",
    "/trusted_code/",
    "/runtime_ref_checks/",
];

fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("undeclared test config `{}`", name))
}

fn run(path: &Path, config: TestConfig) -> datatest_stable::Result<()> {
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
        .collect::<Vec<_>>();
    let vm_test_config = TestRunConfig {
        language_version: config.language_version,
        experiments,
        vm_config: config.vm_config,
        use_masm: true,
        echo: true,
        cross_compilation_targets: BTreeSet::new(),
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
            if p.ends_with(".move") || p.ends_with(".mvir") || p.ends_with(".masm") {
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
