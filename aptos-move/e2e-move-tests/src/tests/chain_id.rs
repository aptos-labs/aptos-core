// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::tests::common;
use crate::{assert_success, MoveHarness};
use language_e2e_tests::account::Account;
use move_core_types::account_address::AccountAddress;
use move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ChainIdStore {
    id: u8,
}

fn call_get_chain_id_from_aptos_framework(harness: &mut MoveHarness, account: &Account) -> u8 {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::chain_id_test::store_chain_id_from_aptos_framework").unwrap(),
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

#[allow(unused)]
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

    assert_success!(harness.publish_package(&account, &path));

    account
}

#[test]
fn test_chain_id_set() {
    let mut harness = MoveHarness::new_mainnet();
    let account = setup(&mut harness);

    assert_eq!(
        call_get_chain_id_from_aptos_framework(&mut harness, &account),
        4u8
    );
}

// #[test]
// fn test_chain_id_set() {
//     let mut harness = MoveHarness::new_mainnet();
//     let account = setup(&mut harness);
//
//     // Initializes the chain ID
//     let chain_id = 128u8;
//     let aptos_framework_acc = harness.aptos_framework_account();
//     let status = harness.run_entry_function(
//         &aptos_framework_acc,
//         str::parse("0x1::chain_id::initialize_for_test").unwrap(),
//         vec![],
//         vec![bcs::to_bytes(&chain_id).unwrap()],
//     );
//
//     assert!(status.status().unwrap().is_success());
//
//     // Tries to fetch it back
//     assert_eq!(call_get_chain_id(&mut harness, &account), chain_id);
// }
