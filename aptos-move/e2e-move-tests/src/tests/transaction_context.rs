// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    move_utils::MemberId,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{account_address::AccountAddress, parser::parse_struct_tag};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct TransactionContextStore {
    sender: AccountAddress,
    secondary_signers: Vec<AccountAddress>,
    gas_payer: AccountAddress,
    max_gas_amount: u64,
    gas_unit_price: u64,
    chain_id: u8,
}

fn setup(harness: &mut MoveHarness) -> Account {
    let path = common::test_dir_path("transaction_context.data/pack");

    let account = harness.new_account_at(AccountAddress::ONE);

    assert_success!(harness.publish_package_cache_building(&account, &path));

    account
}

fn call_get_sender_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> AccountAddress {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::transaction_context_test::store_sender_from_native_txn_context").unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    txn_ctx_store.sender
}

fn call_get_secondary_signers_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> Vec<AccountAddress> {
    let status = harness.run_entry_function(
        account,
        str::parse(
            "0x1::transaction_context_test::store_secondary_signers_from_native_txn_context",
        )
        .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    txn_ctx_store.secondary_signers
}

fn call_get_gas_payer_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> AccountAddress {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::transaction_context_test::store_gas_payer_from_native_txn_context")
            .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    txn_ctx_store.gas_payer
}

fn call_get_max_gas_amount_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> u64 {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::transaction_context_test::store_max_gas_amount_from_native_txn_context")
            .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    txn_ctx_store.max_gas_amount
}

fn call_get_gas_unit_price_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> u64 {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::transaction_context_test::store_gas_unit_price_from_native_txn_context")
            .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    txn_ctx_store.gas_unit_price
}

fn call_get_chain_id_from_native_txn_context(
    harness: &mut MoveHarness,
    account: &Account,
) -> u8 {
    let status = harness.run_entry_function(
        account,
        str::parse("0x1::transaction_context_test::store_chain_id_from_native_txn_context")
            .unwrap(),
        vec![],
        vec![],
    );

    assert!(status.status().unwrap().is_success());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            account.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    txn_ctx_store.chain_id
}


fn new_move_harness() -> MoveHarness {
    MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            FeatureFlag::TRANSACTION_CONTEXT_EXTENSION,
        ],
        vec![],
    )
}

#[test]
fn test_transaction_context_sender() {
    let mut harness = new_move_harness();
    let account = setup(&mut harness);

    let addr = call_get_sender_from_native_txn_context(&mut harness, &account);
    assert_eq!(addr, AccountAddress::ONE);
}

#[test]
fn test_transaction_context_max_gas_amount() {
    let mut harness = new_move_harness();
    let account = setup(&mut harness);

    let max_gas_amount = call_get_max_gas_amount_from_native_txn_context(&mut harness, &account);
    assert_eq!(max_gas_amount, 2000000);
}

#[test]
fn test_transaction_context_gas_unit_price() {
    let mut harness = new_move_harness();
    let account = setup(&mut harness);

    let max_gas_amount = call_get_gas_unit_price_from_native_txn_context(&mut harness, &account);
    assert_eq!(max_gas_amount, 100);
}

#[test]
fn test_transaction_context_chain_id() {
    let mut harness = new_move_harness();
    let account = setup(&mut harness);

    let chain_id = call_get_chain_id_from_native_txn_context(&mut harness, &account);
    assert_eq!(chain_id, 4);
}

#[test]
fn test_transaction_context_gas_payer_as_sender() {
    let mut harness = new_move_harness();
    let account = setup(&mut harness);

    let gas_payer = call_get_gas_payer_from_native_txn_context(&mut harness, &account);
    assert_eq!(gas_payer, *account.address());
}

#[test]
fn test_transaction_context_secondary_signers_empty() {
    let mut harness = new_move_harness();
    let account = setup(&mut harness);

    let secondary_signers =
        call_get_secondary_signers_from_native_txn_context(&mut harness, &account);
    assert_eq!(secondary_signers, vec![]);
}

#[test]
fn test_transaction_context_gas_payer_as_separate_account() {
    let mut harness = new_move_harness();

    let alice = setup(&mut harness);
    let bob = harness.new_account_with_balance_and_sequence_number(1000000, 0);

    let fun: MemberId =
        str::parse("0x1::transaction_context_test::store_gas_payer_from_native_txn_context")
            .unwrap();
    let MemberId {
        module_id,
        member_id: function_id,
    } = fun;
    let ty_args = vec![];
    let args = vec![];
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        function_id,
        ty_args,
        args,
    ));
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(harness.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = harness.run_raw(transaction);
    assert_success!(*output.status());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            alice.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    let gas_payer = txn_ctx_store.gas_payer;
    assert_eq!(gas_payer, *bob.address());
}

#[test]
fn test_transaction_context_secondary_signers() {
    let mut harness = new_move_harness();

    let alice = setup(&mut harness);
    let bob = harness.new_account_with_balance_and_sequence_number(1000000, 0);

    let fun: MemberId = str::parse(
        "0x1::transaction_context_test::store_secondary_signers_from_native_txn_context_multi",
    )
    .unwrap();
    let MemberId {
        module_id,
        member_id: function_id,
    } = fun;
    let ty_args = vec![];
    let args = vec![];
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        function_id,
        ty_args,
        args,
    ));
    let transaction = TransactionBuilder::new(alice.clone())
        .secondary_signers(vec![bob.clone()])
        .payload(payload)
        .sequence_number(harness.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_multi_agent();

    let output = harness.run_raw(transaction);
    assert_success!(*output.status());

    let txn_ctx_store = harness
        .read_resource::<crate::tests::transaction_context::TransactionContextStore>(
            alice.address(),
            parse_struct_tag("0x1::transaction_context_test::TransactionContextStore").unwrap(),
        )
        .unwrap();

    let secondary_signers = txn_ctx_store.secondary_signers;
    assert_eq!(secondary_signers, vec![*bob.address()]);
}
