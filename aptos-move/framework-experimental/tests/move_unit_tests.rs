// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is duplicated from `aptos-move/move-examples/tests/move_unit_tests.rs` then slightly modified.

use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{aptos_test_feature_flags_genesis, Features, TimedFeaturesBuilder},
};
use aptos_vm::natives;
use move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_package::{source_package::std_lib::StdVersion, CompilerConfig};
use move_unit_test::UnitTestingConfig;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::{collections::BTreeMap, path::PathBuf};
use tempfile::tempdir;

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

/// Get the local framework path based on this source file's location.
/// Note: If this source file is moved to a different location, this function
/// may need to be updated.
fn get_local_framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("framework"))
        .expect("framework path")
        .to_string_lossy()
        .to_string()
}

pub fn aptos_test_natives() -> NativeFunctionTable {
    natives::configure_for_unit_test();
    extended_checks::configure_extended_checks_for_unit_test();
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

pub fn run_tests_for_pkg(
    path_to_pkg: impl Into<String>,
    named_addr: BTreeMap<String, AccountAddress>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let ok = run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            override_std: Some(StdVersion::Local(get_local_framework_path())),
            additional_named_addresses: named_addr,
            compiler_config: CompilerConfig {
                known_attributes: extended_checks::get_all_attribute_names().clone(),
                ..Default::default()
            },
            ..Default::default()
        },
        UnitTestingConfig::default(),
        // TODO(Gas): we may want to switch to non-zero costs in the future
        aptos_test_natives(),
        aptos_test_feature_flags_genesis(),
        /* gas limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    )
    .unwrap();
    if ok != UnitTestResult::Success {
        panic!("move unit tests failed")
    }
}

#[test]
fn test_veiled_coin() {
    run_tests_for_pkg("veiled_coin", BTreeMap::default());
}
