// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::{
    account::{Account, TransactionBuilder},
    transaction_status_eq,
};
use aptos_types::{
    account_address::AccountAddress, account_config::CoinStoreResource,
    on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_core_types::{move_resource::MoveStructType, vm_status::StatusCode};

#[test]
fn test_normal_tx_with_signer_with_fee_payer() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start = h.read_aptos_balance(alice.address());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert_success!(*output.status());

    let alice_after = h.read_aptos_balance(alice.address());
    let bob_after = h.read_aptos_balance(bob.address());

    assert_eq!(alice_start, alice_after);
    assert!(bob_start > bob_after);
}

#[test]
fn test_account_not_exist_with_fee_payer_create_account() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = Account::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_start.is_none());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert_success!(*output.status());

    let alice_after =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_after.is_none());
    let bob_after = h.read_aptos_balance(bob.address());

    assert!(bob_start > bob_after);
}

#[test]
fn test_account_not_exist_with_fee_payer_without_create_account() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![
        FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_CREATION,
    ]);

    let alice = Account::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_start.is_none());

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST)
    ));
}

#[test]
fn test_normal_tx_with_fee_payer_insufficient_funds() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![]);

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let bob = h.new_account_with_balance_and_sequence_number(1, 0);

    let payload = aptos_stdlib::aptos_account_set_allow_direct_coin_transfers(true);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
    ));
}
