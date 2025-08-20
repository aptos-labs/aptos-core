// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use libtest_mimic::{Arguments, Trial};
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use once_cell::sync::Lazy;
use std::{
    default::Default,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Clone)]
struct TestConfig {
    name: &'static str,
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
    vec![TestConfig {
        name: "baseline",
        include: &[],
        exclude: &[],
    }]
});

/// Test files which must use separate baselines because their result is different.
///
/// Note that each config named "foo" above will compare the output of running `test.move` (or
/// `test.masm`) to the same baseline file `test.exp` *unless* there is an entry in this array
/// matching the path of `test.move` or `test.masm`. If there is such an entry, then each config
/// "foo" will have a separate baseline output file `test.foo.exp`.
const SEPARATE_BASELINE: &[&str] = &[];

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
    let config = TestRunConfig::default();
    vm_test_harness::run_test_with_config_and_exp_suffix(config, path, &exp_suffix)
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
                    let prompt = format!(
                        "move-bytecode-verifier-txn[config={}]::{}",
                        config.name, file
                    );
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
