// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        scheduled_txn::ScheduledTransactionInfoWithKey, AbortInfo, ExecutionStatus,
        TransactionOutput, TransactionStatus,
    },
};
use move_core_types::{value::MoveValue, vm_status::AbortLocation};

fn setup_test_env() -> (MoveHarness, Account, u64) {
    let build_options = BuildOptions::move_2().set_latest_language();
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("scheduled_txns.data"),
        build_options
    ));

    // Get current timestamp in milliseconds
    let current_time_ms = h.executor.get_block_time() / 1000;

    (h, acc, current_time_ms)
}

fn get_scheduled_txns(h: &mut MoveHarness, acc: &Account) -> Vec<ScheduledTransactionInfoWithKey> {
    let result = h.execute_view_function(
        str::parse("0xcafe::scheduled_txns_usage::get_stored_sched_txns").unwrap(),
        vec![],
        vec![MoveValue::Address(*acc.address())
            .simple_serialize()
            .unwrap()],
    );

    bcs::from_bytes::<Vec<ScheduledTransactionInfoWithKey>>(
        &result.values.expect("Getting keys failed!!")[0],
    )
    .unwrap()
}

fn execute_scheduled_txns(
    h: &mut MoveHarness,
    scheduled_txns: &[ScheduledTransactionInfoWithKey],
) -> Vec<TransactionOutput> {
    use aptos_types::transaction::Transaction;
    let txns: Vec<Transaction> = scheduled_txns
        .iter()
        .cloned()
        .map(Transaction::ScheduledTransaction)
        .collect();

    let outputs = h.executor.execute_transaction_block(txns).unwrap();

    // Apply write sets to update harness state
    for output in &outputs {
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            h.executor.apply_write_set(output.write_set());
            h.executor.append_events(output.events().to_vec());
        }
    }

    outputs
}

#[test]
fn test_basic_execute() {
    let (mut h, acc, current_time_ms) = setup_test_env();

    let initial_balance = h.read_aptos_balance(acc.address());

    // Define the parameters for scheduled transactions
    let values: Vec<u64> = vec![1 /*, 2, 3*/];
    let gas_amounts: Vec<u64> = vec![10000 /*, 10000, 10000*/];
    let gas_prices: Vec<u64> = vec![300 /*, 300, 200*/];
    let num_txns = values.len();

    let signed_txn = h.create_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::create_and_add_transactions").unwrap(),
        vec![],
        vec![
            MoveValue::U64(current_time_ms).simple_serialize().unwrap(),
            MoveValue::Vector(values.into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(
                gas_amounts
                    .clone()
                    .into_iter()
                    .map(MoveValue::U64)
                    .collect(),
            )
            .simple_serialize()
            .unwrap(),
            MoveValue::Vector(gas_prices.clone().into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
        ],
    );
    let output = h.run_raw(signed_txn);
    assert_success!(output.status().to_owned());
    let expected_gas_deposit: u64 = gas_amounts
        .iter()
        .zip(gas_prices.iter())
        .map(|(a, b)| a * b)
        .sum();

    // Check if gas deposit is collected correctly
    let curr_balance = h.read_aptos_balance(acc.address());
    let gas_fees_to_schedule = h.default_gas_unit_price * output.gas_used();

    let expected_deduction = gas_fees_to_schedule + expected_gas_deposit;
    let actual_deduction = initial_balance - curr_balance;

    assert_eq!(
        actual_deduction, expected_deduction,
        "Actual deduction {} should equal expected deduction {}",
        actual_deduction, expected_deduction
    );

    let scheduled_txns = get_scheduled_txns(&mut h, &acc);
    assert_eq!(scheduled_txns.len(), num_txns);
    let outputs = execute_scheduled_txns(&mut h, &scheduled_txns);
    assert!(outputs
        .iter()
        .all(|output| output.status().status().unwrap().is_success()));

    // Verify refunds are applied correctly
    let final_balance = h.read_aptos_balance(acc.address());

    // Calculate expected final balance after scheduled transactions execute
    let gas_fees_to_run: u64 = outputs
        .iter()
        .map(|output| h.default_gas_unit_price * output.gas_used())
        .sum();

    // Expected final balance should be approximately:
    // curr_balance + deposits_refunded - execution_costs + storage_fee_refunds
    // We use a buffer to account for storage_fee_refunds
    // todo: check if we can measure storage_fee_refunds accurately
    let execution_buffer_per_txn = 50000;
    let total_execution_buffer = execution_buffer_per_txn * outputs.len() as u64;

    // With gas deposit refunds, total_gas_costs ~ total_balance_change
    let total_balance_change = initial_balance as i64 - final_balance as i64;
    let total_gas_costs = gas_fees_to_run as i64 + gas_fees_to_schedule as i64;

    // Allow for positive variance due to storage fee refunds
    assert!(
        total_gas_costs >= total_balance_change
            && total_gas_costs <= total_balance_change + total_execution_buffer as i64,
        "total_gas_costs {}; total_balance_change {}",
        total_gas_costs,
        total_balance_change,
    );
}

#[test]
fn test_user_func_abort() {
    let (mut h, acc, current_time_ms) = setup_test_env();

    // Schedule some transactions
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::add_txn_with_user_func_abort").unwrap(),
        vec![],
        vec![MoveValue::U64(current_time_ms).simple_serialize().unwrap()],
    );
    assert_success!(result);

    let scheduled_txns = get_scheduled_txns(&mut h, &acc);
    assert_eq!(scheduled_txns.len(), 1);
    let output = execute_scheduled_txns(&mut h, &scheduled_txns);
    assert_eq!(
        output[0].status().status().unwrap(),
        ExecutionStatus::MoveAbort {
            location: AbortLocation::Module(str::parse("0xcafe::scheduled_txns_usage").unwrap(),),
            code: 1,
            info: None
        }
    );
}

#[test]
fn test_run_and_cancel_race_condition() {
    let (mut h, acc, current_time_ms) = setup_test_env();

    // Schedule one transaction
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::create_and_add_transactions").unwrap(),
        vec![],
        vec![
            MoveValue::U64(current_time_ms).simple_serialize().unwrap(),
            MoveValue::Vector(vec![1].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(vec![10000].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(vec![300].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
        ],
    );
    assert_success!(result);

    let scheduled_txns = get_scheduled_txns(&mut h, &acc);
    assert_eq!(scheduled_txns.len(), 1);
    let outputs = execute_scheduled_txns(&mut h, &scheduled_txns);
    assert!(outputs
        .iter()
        .all(|output| output.status().status().unwrap().is_success()));

    // Cancel the scheduled transaction
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::cancel_txn").unwrap(),
        vec![],
        vec![],
    );
    assert_eq!(result.status().unwrap(), ExecutionStatus::MoveAbort {
        location: AbortLocation::Module(str::parse("0x1::scheduled_txns").unwrap(),),
        code: 65549,
        info: Some(AbortInfo {
            reason_name: "ECANCEL_TOO_LATE".to_string(),
            description:
                "Cannot cancel a transaction that is about to be run or has already been run"
                    .to_string(),
        })
    });
}

#[test]
fn test_cancel_and_run_race_condition() {
    let (mut h, acc, _) = setup_test_env();
    // Use a large timestamp to ensure the transaction is scheduled in the future
    let current_time_ms = u32::MAX as u64;

    // Schedule one transaction
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::create_and_add_transactions").unwrap(),
        vec![],
        vec![
            MoveValue::U64(current_time_ms).simple_serialize().unwrap(),
            MoveValue::Vector(vec![1].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(vec![10000].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(vec![300].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
        ],
    );
    assert_success!(result);

    let scheduled_txns = get_scheduled_txns(&mut h, &acc);
    assert_eq!(scheduled_txns.len(), 1);

    let txn_to_cancel = &scheduled_txns[0];
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::cancel_txn").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    // Run the cancelled transaction
    let output = execute_scheduled_txns(&mut h, &[txn_to_cancel.clone()]);
    assert!(
        output[0].status().is_discarded(),
        "Expected the cancelled transaction to be discarded, but it was not. Output: {:?}",
        output[0].status()
    );
}

#[test]
fn test_cancel_without_execution() {
    let (mut h, acc, _) = setup_test_env();
    // Use a large timestamp to ensure the transaction is scheduled in the future
    let current_time_ms = u32::MAX as u64;

    let initial_balance = h.read_aptos_balance(acc.address());

    // Schedule one transaction
    let signed_txn = h.create_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::create_and_add_transactions").unwrap(),
        vec![],
        vec![
            MoveValue::U64(current_time_ms).simple_serialize().unwrap(),
            MoveValue::Vector(vec![1].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(vec![10000].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
            MoveValue::Vector(vec![300].into_iter().map(MoveValue::U64).collect())
                .simple_serialize()
                .unwrap(),
        ],
    );
    let result = h.run_raw(signed_txn);
    assert_success!(result.status().to_owned());

    // Balance after scheduling (should be reduced by deposit)
    let balance_after_scheduling = h.read_aptos_balance(acc.address());

    let expected_deposit = 10000 * 300; // max_gas_amount * gas_unit_price
    let setup_gas_cost = h.default_gas_unit_price * result.gas_used();
    let expected_balance_after_scheduling = initial_balance - expected_deposit - setup_gas_cost;
    assert_eq!(
        balance_after_scheduling, expected_balance_after_scheduling,
        "Balance after scheduling should equal initial balance minus deposit and setup gas costs"
    );

    // Verify the transaction was scheduled
    let scheduled_txns = get_scheduled_txns(&mut h, &acc);
    assert_eq!(scheduled_txns.len(), 1);

    // Cancel the first scheduled transaction (no args needed, function picks first from BigOrderedMap)
    let cancel_signed_txn = h.create_entry_function(
        &acc,
        str::parse("0xcafe::scheduled_txns_usage::cancel_txn").unwrap(),
        vec![],
        vec![],
    );
    let cancel_result = h.run_raw(cancel_signed_txn);
    assert_success!(cancel_result.status().to_owned());

    // Balance after cancelling (should be back to initial minus gas costs)
    let balance_after_cancelling = h.read_aptos_balance(acc.address());

    let cancel_gas_cost = h.default_gas_unit_price * cancel_result.gas_used();

    // Verify the deposit was refunded with buffer for storage fee interactions
    let total_gas_costs = setup_gas_cost + cancel_gas_cost;

    // Buffer per transaction to account for storage fee refunds during cancellation
    let cancel_buffer_per_txn = 50000; // Buffer for storage fee interactions per cancelled transaction
    let total_cancel_buffer = cancel_buffer_per_txn; // Only 1 transaction in this test

    let expected_balance = initial_balance - total_gas_costs;
    let balance_difference = balance_after_cancelling as i64 - expected_balance as i64;

    // Allow for positive variance due to storage fee refunds during cancellation
    assert!(
        balance_difference >= 0 && balance_difference <= total_cancel_buffer as i64,
        "Balance difference {} should be between 0 and {} (buffer for storage fee refunds)",
        balance_difference,
        total_cancel_buffer
    );
}
