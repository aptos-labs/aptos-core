// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_address::{self, AccountAddress};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::str::FromStr;
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::account_config::AccountResource;
use aptos_types::on_chain_config::FeatureFlag;
use move_core_types::move_resource::MoveStructType;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FungibleStore {
    metadata: AccountAddress,
    balance: u64,
    allow_ungated_balance_transfer: bool,
}

pub static FUNGIBLE_STORE_TAG: Lazy<StructTag> = Lazy::new(|| StructTag {
    address: AccountAddress::from_hex_literal("0x1").unwrap(),
    module: Identifier::new("fungible_asset").unwrap(),
    name: Identifier::new("FungibleStore").unwrap(),
    type_args: vec![],
});

pub static OBJ_GROUP_TAG: Lazy<StructTag> = Lazy::new(|| StructTag {
    address: AccountAddress::from_hex_literal("0x1").unwrap(),
    module: Identifier::new("object").unwrap(),
    name: Identifier::new("ObjectGroup").unwrap(),
    type_args: vec![],
});
#[test]
fn test_basic_fungible_token() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());
    let root = h.aptos_framework_account();

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

    assert_success!(h.run_entry_function(
        &root,
        str::parse(&format!(
            "0x{}::coin::create_coin_conversion_map",
            (*root.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![],
    ));

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
        .values
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

    // Ensure that the group data can be read
    let mut alice_store: FungibleStore = h
        .read_resource_from_resource_group(
            &alice_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone(),
        )
        .unwrap();

    let bob_store: FungibleStore = h
        .read_resource_from_resource_group(
            &bob_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone(),
        )
        .unwrap();

    assert_ne!(alice_store, bob_store);
    // Determine that the only difference is the balance
    assert_eq!(alice_store.balance, 70);
    alice_store.balance = 10;
    assert_eq!(alice_store, bob_store);
}

// A simple test to verify gas paying still work for prologue and epilogue.
#[test]
fn test_coin_to_fungible_asset_migration() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let alice_primary_store_addr =
        account_address::create_derived_object_address(*alice.address(), AccountAddress::TEN);
    let root = h.aptos_framework_account();

    assert_success!(h.run_entry_function(
        &root,
        str::parse(&format!(
            "0x{}::coin::create_coin_conversion_map",
            (*root.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &root,
        str::parse(&format!(
            "0x{}::coin::create_pairing",
            (*root.address()).to_hex()
        ))
        .unwrap(),
        vec![TypeTag::from_str("0x1::supra_coin::SupraCoin").unwrap()],
        vec![],
    ));
    assert!(h
        .read_resource_from_resource_group::<FungibleStore>(
            &alice_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone()
        )
        .is_none());

    let result = h.run_entry_function(
        &alice,
        str::parse("0x1::coin::migrate_to_fungible_store").unwrap(),
        vec![TypeTag::from_str("0x1::supra_coin::SupraCoin").unwrap()],
        vec![],
    );
    assert_success!(result);

    assert!(h
        .read_resource_from_resource_group::<FungibleStore>(
            &alice_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone()
        )
        .is_some());
}

#[test]
fn test_sponsered_tx() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = Account::new();
    let root = h.aptos_framework_account();

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

    assert_success!(h.run_entry_function(
        &root,
        str::parse(&format!(
            "0x{}::coin::create_coin_conversion_map",
            (*root.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![],
    ));

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
        .values
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
    
    let sender_address = *bob.address();
    let sender_hex = sender_address.to_hex();
    let module_src_string = format!(
        r#"
    module 0x{}::test_module {{
        #[view]
        public fun return_tuple(): (u64, u64) {{
            (1, 2)
        }}
    }}
    "#,
        sender_hex
    );
    let module_src = module_src_string.as_str();
    let payload = aptos_stdlib::publish_module_source(
        "test_module",
        module_src
    );
    let transaction = TransactionBuilder::new(bob.clone())
        .fee_payer(alice.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();
    
    let output = h.run_raw(transaction);
    assert_success!(*output.status());
    
    // Make sure bob's account is created
    let exists = h.exists_resource(bob.address(), AccountResource::struct_tag());
    assert!(exists, "Bob's account should exist after the sponsored transaction");

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
    let token_addr = account_address::create_token_address(
        *alice.address(),
        "test collection name",
        "test token name",
    );
    let alice_primary_store_addr =
        account_address::create_derived_object_address(*alice.address(), token_addr);
    let bob_primary_store_addr =
        account_address::create_derived_object_address(*bob.address(), token_addr);
    
    // Ensure that the group data can be read
    let alice_store: FungibleStore = h
        .read_resource_from_resource_group(
            &alice_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone(),
        )
        .unwrap();
    
    let bob_store: FungibleStore = h
        .read_resource_from_resource_group(
            &bob_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone(),
        )
        .unwrap();
    
    assert_ne!(alice_store, bob_store);
}
