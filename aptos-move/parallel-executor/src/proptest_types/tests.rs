// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::ParallelTransactionExecutor,
    proptest_types::types::{
        ExpectedOutput, Task, Transaction, TransactionGen, TransactionGenParams,
    },
};
use num_cpus;
use proptest::{
    collection::vec,
    prelude::*,
    sample::Index,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use std::{fmt::Debug, hash::Hash};

fn run_transactions<K, V>(
    key_universe: Vec<K>,
    transaction_gens: Vec<TransactionGen<V>>,
    abort_transactions: Vec<Index>,
    skip_rest_transactions: Vec<Index>,
    num_repeat: usize,
) -> bool
where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + 'static,
    V: Clone + Eq + Send + Sync + Arbitrary + 'static,
{
    let mut transactions: Vec<_> = transaction_gens
        .into_iter()
        .map(|txn_gen| txn_gen.materialize(&key_universe))
        .collect();

    let length = transactions.len();

    for i in abort_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = Transaction::Abort;
    }

    for i in skip_rest_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = Transaction::SkipRest;
    }

    let mut ret = true;
    for _ in 0..num_repeat {
        let output =
            ParallelTransactionExecutor::<Transaction<K, V>, Task<K, V>>::new(num_cpus::get())
                .execute_transactions_parallel((), transactions.clone());

        let baseline = ExpectedOutput::generate_baseline(&transactions);

        ret = ret && baseline.check_output(&output);
    }
    ret
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn no_early_termination(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        prop_assert!(run_transactions(universe, transaction_gen, abort_transactions, skip_rest_transactions, 1));
    }

    #[test]
    fn abort_only(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        prop_assert!(run_transactions(universe, transaction_gen, abort_transactions, skip_rest_transactions, 1));
    }

    #[test]
    fn skip_rest_only(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        prop_assert!(run_transactions(universe, transaction_gen, abort_transactions, skip_rest_transactions, 1));
    }

    #[test]
    fn mixed_transactions(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        prop_assert!(run_transactions(universe, transaction_gen, abort_transactions, skip_rest_transactions, 1));
    }

    #[test]
    fn dynamic_read_writes_mixed(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGen<[u8;32]>>(TransactionGenParams::new_dynamic()), 3000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        prop_assert!(run_transactions(universe, transaction_gen, abort_transactions, skip_rest_transactions, 1));
    }
}

#[test]
fn dynamic_read_writes() {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 100)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        3000,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    assert!(run_transactions(
        universe,
        transaction_gen,
        vec![],
        vec![],
        100
    ));
}

#[test]
fn dynamic_read_writes_contended() {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 10)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();

    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        1000,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    assert!(run_transactions(
        universe,
        transaction_gen,
        vec![],
        vec![],
        100
    ));
}
