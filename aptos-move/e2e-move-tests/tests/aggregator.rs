// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    account_address::AccountAddress,
    transaction::{SignedTransaction, TransactionStatus},
};
use e2e_move_tests::{assert_success, MoveHarness};
use language_e2e_tests::account::Account;

mod common;

/// Initialize the test aggregator.
fn initialize(limit: u128) -> (MoveHarness, Account) {
    // TODO: once parallel execution supports deltas - change to `new_mainnet`.
    let mut harness = MoveHarness::new_no_parallel();
    let account = harness.new_account_at(AccountAddress::ONE);

    assert_success!(
        harness.publish_package(&account, &common::test_dir_path("aggregator.data/pack"))
    );
    assert_success!(harness.run_entry_function(
        &account,
        str::parse("0x1::aggregator_test::new").unwrap(),
        vec![],
        vec![bcs::to_bytes(&limit).unwrap()],
    ));
    (harness, account)
}

/// Returns a transaction which executes `aggregator_test::add` script.
fn create_add(h: &mut MoveHarness, account: &Account, value: u128) -> SignedTransaction {
    h.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::add").unwrap(),
        vec![],
        vec![bcs::to_bytes(&value).unwrap()],
    )
}

/// Returns a transaction which executes `aggregator_test::sub` script.
fn create_sub(h: &mut MoveHarness, account: &Account, value: u128) -> SignedTransaction {
    h.create_entry_function(
        account,
        str::parse("0x1::aggregator_test::sub").unwrap(),
        vec![],
        vec![bcs::to_bytes(&value).unwrap()],
    )
}

/// Runs `aggregator_test::assert_eq` script.
fn assert_eq(h: &mut MoveHarness, account: &Account, expected: u128) -> TransactionStatus {
    h.run_entry_function(
        account,
        str::parse("0x1::aggregator_test::assert_eq").unwrap(),
        vec![],
        vec![bcs::to_bytes(&expected).unwrap()],
    )
}

#[test]
fn test_aggregator_block_execution_success() {
    let (mut h, acc) = initialize(10000);

    // Create a block of transactions which add to aggregator and execute it.
    let block_size = 100;
    let txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| create_add(&mut h, &acc, i))
        .collect();
    h.run_block(txns);

    let expected: u128 = block_size * (block_size - 1) / 2;
    assert_success!(assert_eq(&mut h, &acc, expected));

    // Now aggregator stores a value high enough - mix in some subtractions.
    let txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| {
            if i % 2 == 0 {
                create_add(&mut h, &acc, i)
            } else {
                create_sub(&mut h, &acc, i)
            }
        })
        .collect();
    h.run_block(txns);
    assert_success!(assert_eq(&mut h, &acc, expected - block_size / 2));
}

#[test]
#[should_panic]
fn test_aggregator_underflow() {
    let (mut h, acc) = initialize(600);
    let txn1 = create_add(&mut h, &acc, 400);
    let txn2 = create_sub(&mut h, &acc, 500);

    // Currently we panic on going below zero.
    assert_success!(h.run(txn1));
    h.run(txn2);
}

#[test]
#[should_panic]
fn test_aggregator_overflow() {
    let (mut h, acc) = initialize(600);
    let txn1 = create_add(&mut h, &acc, 400);
    let txn2 = create_add(&mut h, &acc, 201);

    // Currently we panic on exceeding the limit.
    assert_success!(h.run(txn1));
    h.run(txn2);
}
