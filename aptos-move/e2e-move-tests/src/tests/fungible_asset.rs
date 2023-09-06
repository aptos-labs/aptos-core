// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_address::{self, AccountAddress};
use move_core_types::{identifier::Identifier, language_storage::StructTag};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FungibleStore {
    metadata: AccountAddress,
    balance: u64,
    allow_ungated_balance_transfer: bool,
}

#[test]
fn test_basic_fungible_token() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("example_addr".to_string(), *alice.address());

    let result = h.publish_package_with_options(
        &alice,
        &common::test_dir_path("../../../move-examples/fungible_asset/managed_fungible_asset"),
        build_options.clone(),
    );

    assert_success!(result);
    let result = h.publish_package_with_options(
        &alice,
        &common::test_dir_path("../../../move-examples/fungible_asset/managed_fungible_token"),
        build_options,
    );
    assert_success!(result);

    let metadata = h
        .execute_view_function(
            str::parse(&format!(
                "0x{}::managed_fungible_token::get_metadata",
                (*alice.address()).to_hex()
            ))
            .unwrap(),
            vec![],
            vec![],
        )
        .unwrap()
        .pop()
        .unwrap();
    let metadata = bcs::from_bytes::<AccountAddress>(metadata.as_slice()).unwrap();

    let result = h.run_entry_function(
        &alice,
        str::parse(&format!(
            "0x{}::managed_fungible_asset::mint_to_primary_stores",
            (*alice.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&vec![alice.address()]).unwrap(),
            bcs::to_bytes(&vec![100u64]).unwrap(), // amount
        ],
    );
    assert_success!(result);

    let result = h.run_entry_function(
        &alice,
        str::parse(&format!(
            "0x{}::managed_fungible_asset::transfer_between_primary_stores",
            (*alice.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&vec![alice.address()]).unwrap(),
            bcs::to_bytes(&vec![bob.address()]).unwrap(),
            bcs::to_bytes(&vec![30u64]).unwrap(), // amount
        ],
    );

    assert_success!(result);
    let result = h.run_entry_function(
        &alice,
        str::parse(&format!(
            "0x{}::managed_fungible_asset::burn_from_primary_stores",
            (*alice.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&vec![bob.address()]).unwrap(),
            bcs::to_bytes(&vec![20u64]).unwrap(), // amount
        ],
    );
    assert_success!(result);

    let token_addr = account_address::create_token_address(
        *alice.address(),
        "test collection name",
        "test token name",
    );
    let alice_primary_store_addr =
        account_address::create_derived_object_address(*alice.address(), token_addr);
    let bob_primary_store_addr =
        account_address::create_derived_object_address(*bob.address(), token_addr);

    let fungible_store_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("fungible_asset").unwrap(),
        name: Identifier::new("FungibleStore").unwrap(),
        type_params: vec![],
    };
    let obj_group_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("object").unwrap(),
        name: Identifier::new("ObjectGroup").unwrap(),
        type_params: vec![],
    };

    // Ensure that the group data can be read
    let mut alice_store: FungibleStore = h
        .read_resource_from_resource_group(
            &alice_primary_store_addr,
            obj_group_tag.clone(),
            fungible_store_tag.clone(),
        )
        .unwrap();

    let bob_store: FungibleStore = h
        .read_resource_from_resource_group(
            &bob_primary_store_addr,
            obj_group_tag,
            fungible_store_tag,
        )
        .unwrap();

    assert_ne!(alice_store, bob_store);
    // Determine that the only difference is the balance
    assert_eq!(alice_store.balance, 70);
    alice_store.balance = 10;
    assert_eq!(alice_store, bob_store);
}
