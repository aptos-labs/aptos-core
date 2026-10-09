// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::{account_address::AccountAddress, effects::ChangeSet};
use move_coverage::coverage_map::CoverageMap;
use move_model::metadata::CompilerVersion;
use move_package::CompilerConfig;
use move_stdlib::natives::{all_natives, GasParameters};
use move_unit_test::{
    package_test::{run_move_unit_tests, UnitTestResult},
    UnitTestingConfig,
};
use std::{fs, path::PathBuf};
use tempfile::tempdir;

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

fn run_tests_for_pkg(path_to_pkg: impl Into<String>, v2: bool) {
    let pkg_path = path_in_crate(path_to_pkg);

    let natives = all_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        GasParameters::zeros(),
    );

    let result = run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            compiler_config: CompilerConfig {
                compiler_version: if v2 {
                    Some(CompilerVersion::latest())
                } else {
                    None
                },
                ..Default::default()
            },
            ..Default::default()
        },
        UnitTestingConfig::default(),
        natives,
        ChangeSet::new(),
        /* gas_limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
        false,
    );
    if result.is_err() || result.is_ok_and(|r| r == UnitTestResult::Failure) {
        panic!("aborting because of Move unit test failures")
    }
}

#[test]
fn one_bytecode_dep() {
    // TODO: automatically discovers all Move packages under a package directory and runs unit tests for them
    run_tests_for_pkg("tests/packages/one-bytecode-dep", true);
    run_tests_for_pkg("tests/packages/one-bytecode-dep", false);
}

#[test]
fn failing_test_writes_coverage_map() {
    let package_dir = tempdir().expect("failed to create temporary package directory");
    let sources_dir = package_dir.path().join("sources");
    fs::create_dir(&sources_dir).expect("failed to create package sources directory");
    fs::write(
        package_dir.path().join("Move.toml"),
        "[package]\nname = \"failing-test\"\nversion = \"1.0.0\"\n",
    )
    .expect("failed to write package manifest");
    fs::write(
        sources_dir.join("failing_test.move"),
        concat!(
            "module 0x42::failing_test {\n",
            "    #[test]\n",
            "    fun fails() { abort 1 }\n",
            "}\n",
        ),
    )
    .expect("failed to write package source");

    let install_dir = tempdir().expect("failed to create temporary install directory");
    let mut output = Vec::new();
    let result = run_move_unit_tests(
        package_dir.path(),
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(install_dir.path().to_path_buf()),
            compiler_config: CompilerConfig {
                compiler_version: Some(CompilerVersion::latest()),
                ..Default::default()
            },
            ..Default::default()
        },
        UnitTestingConfig::default(),
        all_natives(
            AccountAddress::from_hex_literal("0x1").expect("hardcoded address must be valid"),
            GasParameters::zeros(),
        ),
        ChangeSet::new(),
        /* gas_limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ true,
        &mut output,
        false,
    )
    .expect("running the failing test should not return an internal error");

    assert_eq!(result, UnitTestResult::Failure);
    let coverage_map_path = package_dir.path().join(".coverage_map.mvcov");
    let coverage_map = CoverageMap::from_binary_file(&coverage_map_path).unwrap_or_else(|error| {
        panic!(
            "failed test did not produce a readable coverage map: {error:#}\n{}",
            String::from_utf8_lossy(&output)
        )
    });
    let failing_function_was_traced = coverage_map.exec_maps.values().any(|execution_map| {
        execution_map.module_maps.values().any(|module_map| {
            module_map.module_name.as_str() == "failing_test"
                && module_map
                    .function_maps
                    .iter()
                    .any(|(name, program_counters)| {
                        name.as_str() == "fails"
                            && program_counters.values().any(|count| *count > 0)
                    })
        })
    });
    assert!(
        failing_function_was_traced,
        "coverage map did not contain execution data for the failing test:\n{}",
        String::from_utf8_lossy(&output)
    );
}
