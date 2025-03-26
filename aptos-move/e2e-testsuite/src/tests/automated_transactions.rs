// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::tests::automation_registration::AutomationRegistrationTestContext;
use aptos_cached_packages::aptos_framework_sdk_builder;
use aptos_crypto::HashValue;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::automated_transaction::{
    AutomatedTransaction, AutomatedTransactionBuilder, BuilderResult,
};
use aptos_types::transaction::{ExecutionStatus, Transaction, TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn check_unregistered_automated_transaction() {
    let mut test_context = AutomationRegistrationTestContext::new();
    let dest_account = test_context.new_account_data(0, 0);
    let payload = aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100);
    let sequence_number = 0;

    let raw_transaction = test_context
        .sender_account_data()
        .account()
        .transaction()
        .payload(payload)
        .sequence_number(sequence_number)
        .ttl(4000)
        .raw();
    let parent_has = HashValue::new([42; HashValue::LENGTH]);
    let automated_txn = AutomatedTransaction::new(raw_transaction.clone(), parent_has, 1);
    let result =
        test_context.execute_tagged_transaction(Transaction::AutomatedTransaction(automated_txn));
    AutomationRegistrationTestContext::check_discarded_output(
        result,
        StatusCode::NO_ACTIVE_AUTOMATED_TASK,
    );
}

#[test]
fn check_expired_automated_transaction() {
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.advance_chain_time_in_secs(2500);
    let dest_account = test_context.new_account_data(0, 0);
    let payload = aptos_framework_sdk_builder::supra_coin_mint(dest_account.address().clone(), 100);
    let sequence_number = 0;

    let raw_transaction = test_context
        .sender_account_data()
        .account()
        .transaction()
        .payload(payload)
        .sequence_number(sequence_number)
        .ttl(1000)
        .raw();

    let parent_hash = HashValue::new([42; HashValue::LENGTH]);
    let automated_txn = AutomatedTransaction::new(raw_transaction.clone(), parent_hash, 1);
    let result =
        test_context.execute_tagged_transaction(Transaction::AutomatedTransaction(automated_txn));
    AutomationRegistrationTestContext::check_discarded_output(
        result,
        StatusCode::TRANSACTION_EXPIRED,
    );
}

#[test]
fn check_automated_transaction_with_insufficient_balance() {
    let mut test_context = AutomationRegistrationTestContext::new();
    let dest_account = test_context.new_account_data(0, 0);
    let payload =
        aptos_framework_sdk_builder::supra_account_transfer(dest_account.address().clone(), 100);
    let sequence_number = 0;

    let raw_transaction = test_context
        .sender_account_data()
        .account()
        .transaction()
        .payload(payload)
        .sequence_number(sequence_number)
        .gas_unit_price(1_000_000)
        .max_gas_amount(1_000_000)
        .ttl(1000)
        .raw();

    let parent_hash = HashValue::new([42; HashValue::LENGTH]);
    let automated_txn = AutomatedTransaction::new(raw_transaction.clone(), parent_hash, 1);
    let result =
        test_context.execute_tagged_transaction(Transaction::AutomatedTransaction(automated_txn));
    AutomationRegistrationTestContext::check_discarded_output(
        result,
        StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE,
    );
}

#[test]
fn check_automated_transaction_successful_execution() {
    let mut test_context = AutomationRegistrationTestContext::new();
    test_context.set_supra_native_automation(true);
    let dest_account = test_context.new_account_data(1_000_000, 0);
    let payload =
        aptos_framework_sdk_builder::supra_account_transfer(dest_account.address().clone(), 100);
    let gas_price = 100;
    let max_gas_amount = 100;
    let automation_fee_cap = 100_000;
    let aux_data = Vec::new();

    // Register automation task
    let inner_entry_function = payload.clone().into_entry_function();
    let expiration_time = test_context.chain_time_now() + 8000;
    let automation_txn = test_context.create_automation_txn(
        0,
        inner_entry_function.clone(),
        expiration_time,
        gas_price,
        max_gas_amount,
        automation_fee_cap,
        aux_data,
    );

    let output = test_context.execute_and_apply(automation_txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "{output:?}"
    );
    let next_task_index = test_context.get_next_task_index_from_registry();
    assert_eq!(next_task_index, 1);
    let automated_task_details = test_context.get_task_details(next_task_index - 1);
    let automated_txn_builder = AutomatedTransactionBuilder::try_from(automated_task_details)
        .expect("Successful builder creation");
    let maybe_automated_txn = automated_txn_builder
        .clone()
        .with_chain_id(ChainId::test())
        .with_block_height(1)
        .with_gas_unit_price(gas_price)
        .build();
    let BuilderResult::Success(automated_txn) = maybe_automated_txn else {
        panic!("Automated transaction should successfully build")
    };

    let result = test_context
        .execute_tagged_transaction(Transaction::AutomatedTransaction(automated_txn.clone()));
    AutomationRegistrationTestContext::check_discarded_output(
        result,
        StatusCode::NO_ACTIVE_AUTOMATED_TASK,
    );

    // Moving to the next epoch
    test_context.advance_chain_time_in_secs(7200);

    // Execute automated transaction one more time which should be success, as task is already become active after epoch change
    let sender_address = test_context.sender_account_address();
    let sender_seq_num = test_context.account_sequence_number(sender_address);
    let output = test_context
        .execute_and_apply_transaction(Transaction::AutomatedTransaction(automated_txn.clone()));
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "{output:?}"
    );
    let dest_account_balance = test_context.account_balance(dest_account.address().clone());
    assert_eq!(dest_account_balance, 1_000_100);
    // check that sequence number is not updated.
    assert_eq!(
        sender_seq_num,
        test_context.account_sequence_number(sender_address)
    );

    // try to submit automated transaction with incorrect sender
    let maybe_automated_txn = automated_txn_builder
        .with_sender(*dest_account.address())
        .with_chain_id(ChainId::test())
        .with_block_height(1)
        .with_gas_unit_price(gas_price)
        .build();
    let BuilderResult::Success(automated_txn) = maybe_automated_txn else {
        panic!("Automated transaction should successfully build")
    };
    let result = test_context
        .execute_tagged_transaction(Transaction::AutomatedTransaction(automated_txn.clone()));
    AutomationRegistrationTestContext::check_discarded_output(
        result,
        StatusCode::NO_ACTIVE_AUTOMATED_TASK,
    );
}
