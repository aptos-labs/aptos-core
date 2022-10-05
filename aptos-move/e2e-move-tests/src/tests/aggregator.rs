// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator::{
        add, add_and_materialize, check, destroy, initialize, materialize, materialize_and_add,
        materialize_and_sub, new, sub, sub_add, sub_and_materialize,
    },
    assert_abort, assert_success,
    tests::common,
    MoveHarness,
};
use aptos_types::transaction::SignedTransaction;
use language_e2e_tests::account::Account;

fn setup() -> (MoveHarness, Account) {
    initialize(common::test_dir_path("aggregator.data/pack"))
}

#[test]
fn test_aggregators_e2e() {
    let (mut h, acc) = setup();
    let block_size = 1000;

    // Create many aggregators with deterministic limit.
    let txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| new(&mut h, &acc, i, (i as u128) * 100000))
        .collect();
    h.run_block(txns);

    // All transactions in block must fail, so values of aggregators are still 0.
    let failed_txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| match i % 2 {
            0 => materialize_and_add(&mut h, &acc, i, (i as u128) * 100000 + 1),
            _ => materialize_and_sub(&mut h, &acc, i, (i as u128) * 100000 + 1),
        })
        .collect();
    h.run_block(failed_txns);

    // Now test all operations. To do that, make sure aggregator have values large enough.
    let txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| add(&mut h, &acc, i, (i as u128) * 1000))
        .collect();
    h.run_block(txns);

    // TODO: proptests with random transaction generator might be useful here.
    let txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| match i % 4 {
            0 => sub_add(&mut h, &acc, i, (i as u128) * 1000, (i as u128) * 3000),
            1 => materialize_and_add(&mut h, &acc, i, (i as u128) * 1000),
            2 => sub_and_materialize(&mut h, &acc, i, (i as u128) * 1000),
            _ => add(&mut h, &acc, i, i as u128),
        })
        .collect();
    h.run_block(txns);

    // Finally, check values.
    let txns: Vec<SignedTransaction> = (0..block_size)
        .map(|i| match i % 4 {
            0 => check(&mut h, &acc, i, (i as u128) * 3000),
            1 => check(&mut h, &acc, i, (i as u128) * 2000),
            2 => check(&mut h, &acc, i, 0),
            _ => check(&mut h, &acc, i, (i as u128) * 1000 + (i as u128)),
        })
        .collect();
    let outputs = h.run_block(txns);
    for status in outputs {
        assert_success!(status);
    }
}

#[test]
fn test_aggregator_lifetime() {
    let (mut h, acc) = setup();

    let txns = vec![
        new(&mut h, &acc, 0, 1500),
        add(&mut h, &acc, 0, 400),
        materialize(&mut h, &acc, 0),
        add(&mut h, &acc, 0, 500),
        check(&mut h, &acc, 0, 900),
        materialize_and_add(&mut h, &acc, 0, 600),
        materialize_and_sub(&mut h, &acc, 0, 600),
        check(&mut h, &acc, 0, 900),
        sub_add(&mut h, &acc, 0, 200, 300),
        check(&mut h, &acc, 0, 1000),
        // These 2 transactions fail, and should have no side-effects.
        add_and_materialize(&mut h, &acc, 0, 501),
        sub_and_materialize(&mut h, &acc, 0, 1001),
        check(&mut h, &acc, 0, 1000),
        destroy(&mut h, &acc, 0),
        // Aggregator has been destroyed and we cannot add this delta.
        add(&mut h, &acc, 0, 1),
    ];
    let outputs = h.run_block(txns);
    // 2 materializations should have failed.
    assert_abort!(outputs[10], 131073);
    assert_abort!(outputs[11], 131074);

    // All checks must succeed.
    assert_success!(outputs[4]);
    assert_success!(outputs[7]);
    assert_success!(outputs[9]);
    assert_success!(outputs[12]);

    // Aggregator is destroyed (abort code from `table::borrow` failure).
    assert_success!(outputs[13]);
    assert_abort!(outputs[14], 25863);
}

#[test]
#[should_panic]
fn test_aggregator_underflow() {
    let (mut h, acc) = setup();

    let txn1 = new(&mut h, &acc, 0, 600);
    let txn2 = add(&mut h, &acc, 0, 400);
    let txn3 = sub(&mut h, &acc, 0, 500);

    // Currently we panic on going below zero.
    assert_success!(h.run(txn1));
    assert_success!(h.run(txn2));
    h.run(txn3);
}

#[test]
fn test_aggregator_materialize_underflow() {
    let (mut h, acc) = setup();

    let txn1 = new(&mut h, &acc, 0, 600);
    let txn2 = materialize_and_sub(&mut h, &acc, 0, 400);

    // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
    assert_success!(h.run(txn1));
    assert_abort!(h.run(txn2), 131074);
}

#[test]
#[should_panic]
fn test_aggregator_overflow() {
    let (mut h, acc) = setup();

    let txn1 = new(&mut h, &acc, 0, 600);
    let txn2 = add(&mut h, &acc, 0, 400);
    let txn3 = add(&mut h, &acc, 0, 201);

    // Currently we panic on exceeding the limit.
    assert_success!(h.run(txn1));
    assert_success!(h.run(txn2));
    h.run(txn3);
}

#[test]
fn test_aggregator_materialize_overflow() {
    let (mut h, acc) = setup();

    let txn1 = new(&mut h, &acc, 0, 399);
    let txn2 = materialize_and_add(&mut h, &acc, 0, 400);

    // Underflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
    assert_success!(h.run(txn1));
    assert_abort!(h.run(txn2), 131073);
}
