// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{extended_checks, path_in_crate};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use aptos_vm::natives;
use move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_unit_test::UnitTestingConfig;
use move_vm_runtime::native_functions::NativeFunctionTable;
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    let ok = run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..Default::default()
        },
        // TODO(Gas): double check if this is correct
        UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_test_natives(),
        /* cost_table */ None,
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
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeatures::enable_all(),
        Features::default(),
    )
}

#[test]
fn move_framework_unit_tests() {
    run_tests_for_pkg("aptos-framework");
}

#[test]
fn move_aptos_stdlib_unit_tests() {
    run_tests_for_pkg("aptos-stdlib");
}

#[test]
fn move_stdlib_unit_tests() {
    run_tests_for_pkg("move-stdlib");
}

#[test]
fn move_token_unit_tests() {
    run_tests_for_pkg("aptos-token");
}

#[test]
fn move_token_objects_unit_tests() {
    run_tests_for_pkg("aptos-token-objects");
}
