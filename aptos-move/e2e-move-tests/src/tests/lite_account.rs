// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use aptos_types::account_address::AccountAddress;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FungibleStore {
    metadata: AccountAddress,
    balance: u64,
    allow_ungated_balance_transfer: bool,
}

fn get_balance(h: &mut MoveHarness, address: AccountAddress) -> u64 {
    let serialized_balance = h
        .execute_view_function(
            str::parse("0x1::primary_fungible_store::balance").unwrap(),
            vec![move_core_types::language_storage::TypeTag::from_str(
                "0x1::fungible_asset::Metadata",
            )
            .unwrap()],
            vec![
                bcs::to_bytes(&address).unwrap(),
                bcs::to_bytes(&AccountAddress::TEN).unwrap(),
            ],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    bcs::from_bytes::<u64>(&serialized_balance).unwrap()
}

#[test]
fn test_basic_new_account() {
    let mut h = MoveHarness::new();
    // Genesis already create `account` at 0x1, so we make create root before using lite_account.
    let root = h.aptos_framework_account();

    assert_success!(h.run_entry_function(
        &root,
        str::parse("0x1::coin::create_coin_conversion_map").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &root,
        str::parse("0x1::coin::create_pairing").unwrap(),
        vec![
            move_core_types::language_storage::TypeTag::from_str("0x1::aptos_coin::AptosCoin")
                .unwrap(),
        ],
        vec![],
    ));

    h.set_use_lite_account();
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let pre_bob_balance = get_balance(&mut h, *bob.address());

    let result = h.run_entry_function(
        &alice,
        str::parse("0x1::primary_fungible_store::transfer").unwrap(),
        vec![
            move_core_types::language_storage::TypeTag::from_str("0x1::fungible_asset::Metadata")
                .unwrap(),
        ],
        vec![
            bcs::to_bytes(&AccountAddress::TEN).unwrap(),
            bcs::to_bytes(&bob.address()).unwrap(),
            bcs::to_bytes(&100u64).unwrap(), // amount
        ],
    );
    assert_success!(result);

    let post_bob_balance = get_balance(&mut h, *bob.address());
    assert_eq!(post_bob_balance - pre_bob_balance, 100);
}
