// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    on_chain_config::{Features, TimedFeatures},
};
use aptos_vm::natives;
use move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_unit_test::UnitTestingConfig;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
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
    let ok = run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            dev_mode: true,
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            additional_named_addresses: named_addr,
            ..Default::default()
        },
        UnitTestingConfig::default_with_bound(Some(100_000)),
        // TODO(Gas): we may want to switch to non-zero costs in the future
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
    natives::configure_for_unit_test();
    natives::aptos_natives(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
        TimedFeatures::enable_all(),
        Arc::new(Features::default()),
    )
}

fn test_common(pkg: &str) {
    run_tests_for_pkg(pkg, BTreeMap::new());
}

#[test]
fn test_common_account() {
    test_common("common_account");
}

#[test]
fn test_data_structures() {
    test_common("data_structures");
}

#[test]
fn test_defi() {
    test_common("defi");
}

#[test]
fn test_groth16() {
    test_common("groth16_example");
}

#[test]
fn test_hello_blockchain() {
    test_common("hello_blockchain");
}

#[test]
fn test_marketplace() {
    test_common("marketplace")
}

#[test]
fn test_message_board() {
    test_common("message_board");
}

#[test]
fn test_fungible_asset() {
    test_common("fungible_asset/managed_fungible_asset");
    test_common("fungible_asset/managed_fungible_token");
    test_common("fungible_asset/preminted_managed_coin");
    test_common("fungible_asset/simple_managed_coin");
}

#[test]
fn test_mint_nft() {
    test_common("mint_nft/4-Getting-Production-Ready");
}

#[test]
fn test_minter() {
    test_common("scripts/minter");
}

#[test]
fn test_resource_account() {
    test_common("resource_account");
}

#[test]
fn test_resource_groups() {
    test_common("resource_groups/primary");
    test_common("resource_groups/secondary");
}

#[test]
fn test_shared_account() {
    test_common("shared_account");
}

#[test]
fn test_token_objects() {
    test_common("token_objects/hero");
    test_common("token_objects/token_lockup");
    test_common("token_objects/ambassador/move");
}

#[test]
fn test_two_by_two_transfer() {
    test_common("scripts/two_by_two_transfer");
}

#[test]
fn test_post_mint_reveal_nft() {
    test_common("post_mint_reveal_nft");
}

#[test]
fn test_nft_dao_test() {
    test_common("dao/nft_dao");
}
