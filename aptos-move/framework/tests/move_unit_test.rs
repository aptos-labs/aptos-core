// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use aptos_vm::natives;
use framework::path_in_crate;
use move_deps::move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_deps::{
    move_unit_test::UnitTestingConfig, move_vm_runtime::native_functions::NativeFunctionTable,
};
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    let ok = run_move_unit_tests(
        &pkg_path,
        move_deps::move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
        // TODO(Gas): double check if this is correct
        UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_test_natives(),
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    )
    .unwrap();
    if ok != UnitTestResult::Success {
        panic!("move unit tests failed")
    }
}

pub fn aptos_test_natives() -> NativeFunctionTable {
    // By side effect, configure for unit tests
    natives::configure_for_unit_test();
    // move_stdlib has the testing feature enabled to include debug native functions
    natives::aptos_natives(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
    )
}

#[test]
fn move_framework_unit_tests() {
    run_tests_for_pkg("aptos-framework");
}

#[test]
fn move_stdlib_unit_tests() {
    run_tests_for_pkg("aptos-stdlib");
}

#[test]
fn move_token_unit_tests() {
    run_tests_for_pkg("aptos-token");
}
