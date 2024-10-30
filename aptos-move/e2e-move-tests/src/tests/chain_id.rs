// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use move_core_types::{account_address::AccountAddress, parser::parse_struct_tag};
use serde::{Deserialize, Serialize};
use rstest::rstest;

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

fn setup(harness: &mut MoveHarness, stateless_account: bool) -> Account {
    let path = common::test_dir_path("chain_id.data/pack");

    let seq_num = if stateless_account { None } else { Some(0) };
    let account = harness.new_account_at(AccountAddress::ONE, seq_num);

    assert_success!(harness.publish_package_cache_building(&account, &path));

    account
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_chain_id_from_aptos_framework(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut harness = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = setup(&mut harness, stateless_account);

    assert_eq!(
        call_get_chain_id_from_aptos_framework(&mut harness, &account),
        4u8
    );
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_chain_id_from_type_info(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut harness = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = setup(&mut harness, stateless_account);

    assert_eq!(
        call_get_chain_id_from_native_txn_context(&mut harness, &account),
        4u8
    );
}
