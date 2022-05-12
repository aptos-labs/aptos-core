// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use aptos_vm::natives::aptos_natives;
use move_deps::{move_cli::package::cli, move_unit_test::UnitTestingConfig};
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
    cli::run_move_unit_tests(
        &pkg_path,
        move_deps::move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            additional_named_addresses: named_addr,
            ..Default::default()
        },
        UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_natives(),
        /* compute_coverage */ false,
    )
    .unwrap();
}

#[test]
fn test_hello_blockchain() {
    let named_address = BTreeMap::from([(
        String::from("HelloBlockchain"),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    )]);
    run_tests_for_pkg("hello_blockchain", named_address);
}

#[test]
fn test_message_board() {
    let named_address = BTreeMap::from([(
        String::from("MessageBoard"),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    )]);
    run_tests_for_pkg("messageboard", named_address);
}
