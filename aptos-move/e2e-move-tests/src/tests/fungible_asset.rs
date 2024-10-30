// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done

use crate::{assert_success, tests::common, BlockSplit, MoveHarness, SUCCESS};
use aptos_cached_packages::aptos_stdlib::{aptos_account_batch_transfer, aptos_account_transfer};
use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecutorMode, FakeExecutor},
};
use aptos_types::{
    account_address::{self, AccountAddress},
    on_chain_config::FeatureFlag,
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use once_cell::sync::Lazy;
use rstest::rstest;
use serde::Deserialize;
use std::str::FromStr;

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

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_basic_fungible_token(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let alice = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap(), Some(0));
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
#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_coin_to_fungible_asset_migration(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let alice = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
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
        vec![TypeTag::from_str("0x1::aptos_coin::AptosCoin").unwrap()],
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
        vec![TypeTag::from_str("0x1::aptos_coin::AptosCoin").unwrap()],
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

/// Trigger speculative error in prologue, from accessing delayed field that was created later than
/// last committed index (so that read_last_commited_value fails speculatively)
///
/// We do that by having an expensive transaction first (to make sure committed index isn't moved),
/// and then create some new aggregators (concurrent balances for new accounts), and then have them issue
/// transactions - so their balance is checked in prologue.
#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_prologue_speculation(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let executor = FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::ParallelOnly);

    let mut harness = MoveHarness::new_with_executor_and_flags(
        executor,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    harness.enable_features(
        vec![
            FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE,
            FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE,
            FeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE,
        ],
        vec![],
    );
    let independent_account =
        harness.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let sink_txn = harness.create_transaction_payload(
        &independent_account,
        aptos_account_batch_transfer(vec![AccountAddress::random(); 50], vec![10_000_000_000; 50]),
    );

    // 0x1 can't be a stateless account. The account address (0x1) isn't equaly to Hash(public key||scheme).
    // A transaction signed by 0x1 succeeds prologue only if an Account resource containing an authentication key
    // exists for 0x1.
    let account = harness.new_account_at(AccountAddress::ONE, Some(0));
    let dst_1 = Account::new();
    let dst_2 = Account::new();
    let dst_3 = Account::new();

    let fund_txn = harness.create_transaction_payload(
        &account,
        aptos_account_batch_transfer(
            vec![*dst_1.address(), *dst_2.address(), *dst_3.address()],
            vec![10_000_000_000, 10_000_000_000, 10_000_000_000],
        ),
    );

    let transfer_1_txn =
        harness.create_transaction_payload(&dst_1, aptos_account_transfer(*dst_2.address(), 1));
    let transfer_2_txn =
        harness.create_transaction_payload(&dst_2, aptos_account_transfer(*dst_3.address(), 1));
    let transfer_3_txn =
        harness.create_transaction_payload(&dst_3, aptos_account_transfer(*dst_1.address(), 1));

    harness.run_block_in_parts_and_check(BlockSplit::Whole, vec![
        (SUCCESS, sink_txn),
        (SUCCESS, fund_txn),
        (SUCCESS, transfer_1_txn),
        (SUCCESS, transfer_2_txn),
        (SUCCESS, transfer_3_txn),
    ]);
}
