// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config::CORE_CODE_ADDRESS;
use aptos_vm::move_vm_ext::test_transaction_context_natives;
use framework::path_in_crate;
use move_deps::{
    move_cli::package::cli, move_stdlib, move_table_extension, move_unit_test::UnitTestingConfig,
    move_vm_runtime::native_functions::NativeFunctionTable,
};
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    cli::run_move_unit_tests(
        &pkg_path,
        move_deps::move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
        UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_test_natives(),
        /* compute_coverage */ false,
    )
    .unwrap();
}

// move_stdlib has the testing feature enabled to include debug native functions
pub fn aptos_test_natives() -> NativeFunctionTable {
    move_stdlib::natives::all_natives(CORE_CODE_ADDRESS)
        .into_iter()
        .chain(framework::natives::all_natives(CORE_CODE_ADDRESS))
        .chain(move_table_extension::table_natives(CORE_CODE_ADDRESS))
        .chain(test_transaction_context_natives(CORE_CODE_ADDRESS))
        .collect()
}

#[test]
fn move_unit_tests() {
    run_tests_for_pkg("aptos-framework");
}
