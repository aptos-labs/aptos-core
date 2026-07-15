// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::{account_address::AccountAddress, effects::ChangeSet};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::CompilerConfig;
use move_stdlib::natives::{all_natives, GasParameters};
use move_unit_test::{
    package_test::{run_move_unit_tests, UnitTestResult},
    UnitTestingConfig,
};
use std::path::PathBuf;
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

/// Runs the `debug-assert` package's unit tests with `debug_assert!` macros enabled
/// or disabled via `BuildConfig::debug_assert` (the path `aptos move test` uses).
fn run_debug_assert_pkg(debug_assert: bool) -> UnitTestResult {
    let pkg_path = path_in_crate("tests/packages/debug-assert");
    let natives = all_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        GasParameters::zeros(),
    );
    run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            debug_assert,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            compiler_config: CompilerConfig {
                compiler_version: Some(CompilerVersion::latest()),
                language_version: Some(LanguageVersion::latest()),
                ..Default::default()
            },
            ..Default::default()
        },
        UnitTestingConfig::default(),
        natives,
        ChangeSet::new(),
        Some(100_000),
        None,
        false,
        &mut std::io::sink(),
        false,
    )
    .expect("unit test run should not error")
}

// `debug_assert!` aborts the #[test] when enabled and is stripped when disabled, so
// the same suite fails with debug assertions on and passes with them off.
#[test]
fn debug_assert_toggled_by_build_config() {
    assert!(
        run_debug_assert_pkg(true) == UnitTestResult::Failure,
        "expected failure with debug assertions enabled"
    );
    assert!(
        run_debug_assert_pkg(false) == UnitTestResult::Success,
        "expected success with debug assertions disabled"
    );
}
