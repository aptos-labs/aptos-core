// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use aptos_types::account_address::AccountAddress;
use aptos_vm::natives;
use move_deps::move_unit_test::UnitTestingConfig;
use move_deps::{
    move_cli::base::test::run_move_unit_tests,
    move_vm_runtime::native_functions::NativeFunctionTable,
};
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

pub fn run_tests_for_pkg(
    path_to_pkg: impl Into<String>,
    named_addr: BTreeMap<String, AccountAddress>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    run_move_unit_tests(
        &pkg_path,
        move_deps::move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            additional_named_addresses: named_addr,
            ..Default::default()
        },
        UnitTestingConfig::default_with_bound(Some(100_000)),
        // TODO(Gas): we may want to switch to non-zero costs in the future
        aptos_test_natives(),
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    )
    .unwrap();
}

pub fn aptos_test_natives() -> NativeFunctionTable {
    natives::configure_for_unit_test();
    natives::aptos_natives(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
    )
}

#[test]
fn test_data_structures() {
    let named_address = BTreeMap::from([(
        String::from("data_structures"),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    )]);
    run_tests_for_pkg("data_structures", named_address);
}

#[test]
fn test_hello_blockchain() {
    let named_address = BTreeMap::from([(
        String::from("hello_blockchain"),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    )]);
    run_tests_for_pkg("hello_blockchain", named_address);
}

#[test]
fn test_message_board() {
    let named_address = BTreeMap::from([(
        String::from("message_board"),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    )]);
    run_tests_for_pkg("message_board", named_address);
}

#[test]
fn test_minter() {
    let named_address = BTreeMap::new();
    run_tests_for_pkg("minter_script", named_address);
}

#[test]
fn test_shared_account() {
    let named_address = BTreeMap::from([(
        String::from("shared_account"),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    )]);
    run_tests_for_pkg("shared_account", named_address);
}
