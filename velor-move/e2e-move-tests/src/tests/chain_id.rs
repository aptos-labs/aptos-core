// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use velor_language_e2e_tests::account::Account;
use move_core_types::{account_address::AccountAddress, parser::parse_struct_tag};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ChainIdStore {
    id: u8,
}

fn call_get_chain_id_from_velor_framework(harness: &mut MoveHarness, account: &Account) -> u8 {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::chain_id_test::store_chain_id_from_velor_framework").unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let chain_id_store = harness
        .read_resource::<ChainIdStore>(
            account.address(),
            parse_struct_tag("0x1::chain_id_test::ChainIdStore").unwrap(),
        )
        .unwrap();

    chain_id_store.id
}

fn call_get_chain_id_from_native_txn_context(harness: &mut MoveHarness, account: &Account) -> u8 {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::chain_id_test::store_chain_id_from_native_txn_context").unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let chain_id_store = harness
        .read_resource::<ChainIdStore>(
            account.address(),
            parse_struct_tag("0x1::chain_id_test::ChainIdStore").unwrap(),
        )
        .unwrap();

    chain_id_store.id
}

fn setup(harness: &mut MoveHarness) -> Account {
    let path = common::test_dir_path("chain_id.data/pack");

    let account = harness.new_account_at(AccountAddress::ONE);

    assert_success!(harness.publish_package_cache_building(&account, &path));

    account
}

#[test]
fn test_chain_id_from_velor_framework() {
    let mut harness = MoveHarness::new();
    let account = setup(&mut harness);

    assert_eq!(
        call_get_chain_id_from_velor_framework(&mut harness, &account),
        4u8
    );
}

#[test]
fn test_chain_id_from_type_info() {
    let mut harness = MoveHarness::new();
    let account = setup(&mut harness);

    assert_eq!(
        call_get_chain_id_from_native_txn_context(&mut harness, &account),
        4u8
    );
}
