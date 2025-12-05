// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_transactional_test_harness::run_aptos_test_with_config_and_exp_suffix;
use libtest_mimic::{Arguments, Trial};
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::vm_test_harness::TestRunConfig;
use once_cell::sync::Lazy;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

const TEST_ROOT: &str = "tests";
const TEST_FILE_EXTENSIONS: &[&str] = &[".move", ".mvir", ".masm"];
const RUNTIME_REF_CHECKS_SEGMENT: &str = "/runtime_ref_checks/";
const ENV_RUNTIME_REF: &str = "APTOS_ENABLE_RUNTIME_REF_CHECKS";
const ENV_CONFIG_FILTER: &str = "APTOS_TXN_TEST_CONFIGS";
const DIRECTIVE_ENABLE_RUNTIME_REF: &str = "enable_runtime_ref_checks";
const DIRECTIVE_ENABLE_RUNTIME_REF_ALT: &str = "enable-runtime-ref-checks";
const SEPARATE_BASELINE: &[&str] = &[RUNTIME_REF_CHECKS_SEGMENT];

#[derive(Clone)]
struct TestConfig {
    name: &'static str,
    experiments: &'static [(&'static str, bool)],
    language_version: LanguageVersion,
    runtime_ref_checks: bool,
    include: &'static [&'static str],
    exclude: &'static [&'static str],
}

impl TestConfig {
    fn matches(&self, path: &Path) -> bool {
        let path_str = path.display().to_string();
        (self.include.is_empty()
            || self
                .include
                .iter()
                .any(|needle| path_str.contains(needle)))
            && !self
                .exclude
                .iter()
                .any(|needle| path_str.contains(needle))
    }
}

static TEST_CONFIGS: Lazy<Vec<TestConfig>> = Lazy::new(|| {
    vec![
        TestConfig {
            name: "baseline",
            experiments: &[("attach-compiled-module", true)],
            language_version: LanguageVersion::latest_stable(),
            runtime_ref_checks: false,
            include: &[],
            exclude: &[],
        },
        TestConfig {
            name: "ref",
            experiments: &[("attach-compiled-module", true)],
            language_version: LanguageVersion::latest_stable(),
            runtime_ref_checks: true,
            include: &[RUNTIME_REF_CHECKS_SEGMENT],
            exclude: &[],
        },
    ]
});

fn main() {
    let files = collect_test_files();
    let configs = selected_configs();

    let mut trials = Vec::new();
    for config in configs {
        for path in files.iter().filter(|path| config.matches(path)) {
            let prompt = format!("aptos-txn[config={}]::{}", config.name, path.display());
            let path = path.clone();
            let config = config.clone();
            trials.push(Trial::test(prompt, move || {
                run_test_case(&path, &config)
                    .map_err(|err| err.to_string().into())
            }));
        }
    }

    trials.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, trials).exit();
}

fn collect_test_files() -> Vec<PathBuf> {
    WalkDir::new(TEST_ROOT)
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.into_path();
            if path.is_file() && has_supported_extension(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

fn has_supported_extension(path: &Path) -> bool {
    let path = path.to_string_lossy();
    TEST_FILE_EXTENSIONS
        .iter()
        .any(|ext| path.ends_with(ext))
}

fn selected_configs() -> Vec<TestConfig> {
    match env::var(ENV_CONFIG_FILTER) {
        Ok(list) if !list.trim().is_empty() => list
            .split(',')
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .map(config_by_name)
            .collect(),
        _ => TEST_CONFIGS.clone(),
    }
}

fn config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .iter()
        .find(|config| config.name.eq_ignore_ascii_case(name))
        .cloned()
        .unwrap_or_else(|| panic!("unknown transactional test config `{name}`"))
}

fn run_test_case(path: &Path, config: &TestConfig) -> Result<()> {
    let experiments = config
        .experiments
        .iter()
        .map(|(flag, enabled)| ((*flag).to_string(), *enabled))
        .collect::<Vec<_>>();

    let mut test_config = TestRunConfig::new(config.language_version, experiments);

    if config.runtime_ref_checks || runtime_ref_checks_requested(path) {
        test_config = test_config.with_runtime_ref_checks();
    }

    let exp_suffix = exp_suffix_for(path, config);

    run_aptos_test_with_config_and_exp_suffix(path, test_config, &exp_suffix)
        .map_err(|error| anyhow!("{}", error))
}

fn runtime_ref_checks_requested(path: &Path) -> bool {
    runtime_ref_checks_requested_by_env()
        || runtime_ref_checks_requested_by_path(path)
        || runtime_ref_checks_requested_by_directive(path)
}

fn runtime_ref_checks_requested_by_env() -> bool {
    match env::var(ENV_RUNTIME_REF) {
        Ok(value) => value.is_empty()
            || value == "1"
            || value.eq_ignore_ascii_case("true")
            || value.eq_ignore_ascii_case("yes"),
        Err(_) => false,
    }
}

fn runtime_ref_checks_requested_by_path(path: &Path) -> bool {
    path.display()
        .to_string()
        .contains(RUNTIME_REF_CHECKS_SEGMENT)
}

fn runtime_ref_checks_requested_by_directive(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|source| {
            source.lines().any(|line| {
                let directive = line.trim_start().strip_prefix("//#").map(|rest| {
                    rest.trim()
                        .trim_matches(|c: char| c.is_ascii_whitespace() || c == ';')
                        .to_ascii_lowercase()
                });
                matches!(
                    directive.as_deref(),
                    Some(DIRECTIVE_ENABLE_RUNTIME_REF)
                        | Some(DIRECTIVE_ENABLE_RUNTIME_REF_ALT)
                )
            })
        })
        .unwrap_or(false)
}

fn exp_suffix_for(path: &Path, config: &TestConfig) -> Option<String> {
    if !uses_separate_baseline(path) {
        return None;
    }

    let suffix = format!("{}.exp", config.name);
    let baseline_candidate = path.with_extension(suffix.as_str());

    if baseline_candidate.exists() {
        Some(suffix)
    } else {
        None
    }
}

fn uses_separate_baseline(path: &Path) -> bool {
    let path_str = path.display().to_string();
    SEPARATE_BASELINE
        .iter()
        .any(|segment| path_str.contains(segment))
}
