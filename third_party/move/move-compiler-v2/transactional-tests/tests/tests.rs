// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the Compiler V2 transactional tests defined by the shared test matrix.

use itertools::Itertools;
use libtest_mimic::{Arguments, Trial};
use move_compiler_v2::logging;
use move_transactional_test_matrix::{
    CompilerV2Payload, MatrixConfig, Resolution, VmBackend, COMPILER_V2,
};
use move_transactional_test_runner::{
    tasks::SyntaxChoice, vm_test_harness, vm_test_harness::TestRunConfig,
};
use std::{fs, path::Path, process::Command};
use walkdir::WalkDir;

/// Builds the settings a (source, config) trial runs with.
fn test_run_config(resolution: &Resolution<'static, CompilerV2Payload>) -> TestRunConfig {
    let experiments = resolution
        .config
        .experiments
        .iter()
        .map(|(name, value)| (name.to_string(), *value))
        .collect_vec();
    let mut vm_test_config = TestRunConfig::new(resolution.config.language_version, experiments)
        .with_runtime_ref_checks();
    // Cross-compiled output depends on compiler settings, so it always uses a
    // config-qualified baseline.
    if resolution.effective_payload.cross_compile {
        vm_test_config = vm_test_config.cross_compile_into(
            SyntaxChoice::Source,
            true,
            resolution
                .canonical_exp_suffix
                .clone()
                .or_else(|| Some(format!("{}.exp", resolution.config.name))),
        );
    }
    vm_test_config
}

fn run(
    identity: &str,
    config: &'static MatrixConfig<CompilerV2Payload>,
) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing(None);
    let resolution = COMPILER_V2
        .resolve(config, identity, VmBackend::V1)
        .expect("the trial was registered, so the config selects its source");
    vm_test_harness::run_test_with_config_and_exp_suffix(
        test_run_config(&resolution),
        Path::new(identity),
        &resolution.canonical_exp_suffix,
    )
}

fn lake_available() -> bool {
    Command::new("lake")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn positive_leaner_baselines_are_clean() -> Result<(), String> {
    const FAILURE_MARKERS: &[&str] = &[
        "warning:",
        "bug:",
        "Error: compilation errors:",
        "LINKER_ERROR",
        "exiting with Leaner stackless-bytecode checks failed",
        "exiting with bytecode verification errors",
    ];
    let mut failures = vec![];
    for entry in WalkDir::new("tests/leaner")
        .min_depth(1)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_negative = name.starts_with("reject_");
        let is_differential = path
            .to_string_lossy()
            .contains("/borrow_checker/leaner_permissive_");
        if !name.ends_with(".exp") || is_negative || is_differential {
            continue;
        }
        let output = fs::read_to_string(path).map_err(|error| error.to_string())?;
        for marker in FAILURE_MARKERS {
            if output.contains(marker) {
                failures.push(format!("{} contains `{marker}`", path.display()));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "positive Leaner baselines must not record compilation or linker failures:\n{}",
            failures.join("\n")
        ))
    }
}

fn main() {
    let has_lake = lake_available();
    // The test runs from the corpus root, so each identity is a valid relative
    // source path.
    let identities = COMPILER_V2.sources(Path::new("."));
    let mut tests = COMPILER_V2
        .configs
        .iter()
        .flat_map(|config| {
            identities
                .iter()
                .filter(|identity| config.selects(identity))
                .map(move |identity| {
                    let prompt = format!("compiler-v2-txn[config={}]::{}", config.name, identity);
                    let requires_lean = identity.ends_with(".lean");
                    let identity = identity.clone();
                    Trial::test(prompt, move || {
                        run(&identity, config).map_err(|err| format!("{:?}", err).into())
                    })
                    .with_ignored_flag(requires_lean && !has_lake)
                })
        })
        .collect_vec();
    tests.push(Trial::test("leaner-positive-baselines-are-clean", || {
        positive_leaner_baselines_are_clean().map_err(Into::into)
    }));
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
