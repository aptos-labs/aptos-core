// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_language_e2e_tests::{
    account::{Account, TransactionBuilder},
    transaction_status_eq,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::CoinStoreResource,
    move_utils::MemberId,
    on_chain_config::{ApprovedExecutionHashes, FeatureFlag, OnChainConfig},
    transaction::{EntryFunction, ExecutionStatus, Script, TransactionPayload, TransactionStatus},
};
use move_core_types::{move_resource::MoveStructType, vm_status::StatusCode};

// Fee payer has several modes and requires several tests to validate:
// Account exists:
// * Account exists and transaction executes successfully
// * Account exists and transaction aborts but is kept
// * Account doesn't exist (seq num 0) and transaction executes successfully
// * Account doesn't exist (seq num 0), transaction aborts due to move abort, and account is created
// * Account doesn't exist (seq num 0), transaction aborts due to out of gas, and account is created
// * Account doesn't exist (seq num 0), transaction aborts due to move abort, during charging of
// account creation changeset, we run out of gas, but account must still be created. Note, this is
// likely a duplicate of the first out of gas, but included.
// * Invalid transactions are discarded during prologue, specifically the special case of seq num 0

#[test]
fn test_existing_account_with_fee_payer() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = h.new_account_with_balance_and_sequence_number(0, 0);
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start = h.read_aptos_balance(alice.address());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_coin_transfer(*alice.address(), 0);
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
fn test_existing_account_with_fee_payer_aborts() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = h.new_account_with_balance_and_sequence_number(0, 0);
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start = h.read_aptos_balance(alice.address());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_coin_transfer(*alice.address(), 1);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    // Alice has an insufficient balance, trying to 1 when she has 0.
    assert_abort!(output.status(), 65542);

    let alice_after = h.read_aptos_balance(alice.address());
    let bob_after = h.read_aptos_balance(bob.address());

    assert_eq!(alice_start, alice_after);
    assert!(bob_start > bob_after);
}

#[test]
fn test_account_not_exist_with_fee_payer() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

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
fn test_account_not_exist_with_fee_payer_insufficient_gas() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_start.is_none());
    let bob_start = h.read_aptos_balance(bob.address());

    let payload = aptos_stdlib::aptos_coin_transfer(*alice.address(), 1);
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(99_999) // This is not enough to execute this transaction
        .gas_unit_price(1)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert!(transaction_status_eq(
        output.status(),
        &TransactionStatus::Discard(StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS)
    ));

    let alice_after =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_after.is_none());
    let bob_after = h.read_aptos_balance(bob.address());
    assert_eq!(bob_start, bob_after);
}

#[test]
fn test_account_not_exist_and_move_abort_with_fee_payer_create_account() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let alice_start =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_start.is_none());
    let bob_start = h.read_aptos_balance(bob.address());

    // script {
    //     fun main() {
    //         1/0;
    //     }
    // }
    let data =
        hex::decode("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102")
            .unwrap();
    let script = Script::new(data, vec![], vec![]);

    // Offered max fee is 10000 + gas_units * 10, the minimum to execute this transaction
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .script(script)
        .sequence_number(0)
        .max_gas_amount(50_010)
        .gas_unit_price(2)
        .sign_fee_payer();

    let output = h.run_raw(transaction);
    assert!(matches!(
        output.status(),
        TransactionStatus::Keep(ExecutionStatus::ExecutionFailure { .. })
    ));
    // We need to charge less than or equal to the max and at least more than a storage slot
    assert!(output.gas_used() * 2 <= 100020);
    assert!(output.gas_used() * 2 > 50000);

    let alice_after =
        h.read_resource::<CoinStoreResource>(alice.address(), CoinStoreResource::struct_tag());
    assert!(alice_after.is_none());
    let bob_after = h.read_aptos_balance(bob.address());

    assert_eq!(h.sequence_number(alice.address()), 1);
    assert!(bob_start > bob_after);
}

#[test]
fn test_account_not_exist_out_of_gas_with_fee_payer() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = Account::new();
    let beef = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // Load the code
    assert_success!(h.publish_package_cache_building(
        &beef,
        &common::test_dir_path("infinite_loop.data/empty_loop"),
    ));

    let MemberId {
        module_id,
        member_id,
    } = str::parse("0xbeef::test::run").unwrap();
    let payload =
        TransactionPayload::EntryFunction(EntryFunction::new(module_id, member_id, vec![], vec![]));
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(beef.clone())
        .payload(payload)
        .sequence_number(0)
        .max_gas_amount(100_010) // This is the minimum to execute this transaction
        .gas_unit_price(1)
        .sign_fee_payer();
    let result = h.run_raw(transaction);

    assert_eq!(
        result.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            aptos_types::vm_status::StatusCode::EXECUTION_LIMIT_REACHED
        ))),
    );
}

#[test]
fn test_account_not_exist_move_abort_with_fee_payer_out_of_gas() {
    // Very large transaction to trigger the out of gas error aborted seqno 0 sponsored transactions
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

    let alice = Account::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let root = h.aptos_framework_account();

    let data = vec![0; 1000 * 1024];
    let entries = ApprovedExecutionHashes {
        entries: vec![(0, HashValue::sha3_256_of(&data).to_vec())],
    };

    let script = Script::new(data, vec![], vec![]);

    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &entries,
    );
    let transaction = TransactionBuilder::new(alice.clone())
        .fee_payer(bob.clone())
        .script(script.clone())
        .sequence_number(0)
        .max_gas_amount(100_010) // This is the minimum to execute this transaction
        .gas_unit_price(1)
        .sign_fee_payer();
    let result = h.run_raw(transaction);
    assert_eq!(result.gas_used(), 100_010);

    let new_alice = Account::new();
    let transaction = TransactionBuilder::new(new_alice.clone())
        .fee_payer(bob.clone())
        .script(script.clone())
        .sequence_number(0)
        .max_gas_amount(100_011) // Bump by one to ensure more gas can be used
        .gas_unit_price(1)
        .sign_fee_payer();
    let result = h.run_raw(transaction);
    assert_eq!(result.gas_used(), 100_011);
}

#[test]
fn test_account_not_exist_with_fee_payer_without_create_account() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::GAS_PAYER_ENABLED], vec![
        FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
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
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
        ],
        vec![],
    );

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
