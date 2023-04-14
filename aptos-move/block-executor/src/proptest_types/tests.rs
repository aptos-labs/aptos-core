// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::Error,
    executor::BlockExecutor,
    proptest_types::types::{
        DeltaDataView, EmptyDataView, ExpectedOutput, KeyType, Task, Transaction, TransactionGen,
        TransactionGenParams, ValueType,
    },
};
use aptos_executable_store::ExecutableStore;
use aptos_types::executable::ExecutableTestType;
use claims::assert_ok;
use num_cpus;
use proptest::{
    collection::vec,
    prelude::*,
    sample::Index,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

fn run_transactions<K, V>(
    key_universe: &[K],
    transaction_gens: Vec<TransactionGen<V>>,
    abort_transactions: Vec<Index>,
    skip_rest_transactions: Vec<Index>,
    num_repeat: usize,
    module_access: (bool, bool),
) where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + 'static,
    V: Clone + Eq + Send + Sync + Arbitrary + 'static,
    Vec<u8>: From<V>,
{
    let mut transactions: Vec<_> = transaction_gens
        .into_iter()
        .map(|txn_gen| txn_gen.materialize(key_universe, module_access))
        .collect();

    let length = transactions.len();
    for i in abort_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = Transaction::Abort;
    }
    for i in skip_rest_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = Transaction::SkipRest;
    }

    let data_view = EmptyDataView::<KeyType<K>, ValueType<V>> {
        phantom: PhantomData,
    };

    for _ in 0..num_repeat {
        let output = BlockExecutor::<
            Transaction<KeyType<K>, ValueType<V>>,
            Task<KeyType<K>, ValueType<V>>,
            EmptyDataView<KeyType<K>, ValueType<V>>,
            ExecutableTestType,
        >::new(num_cpus::get())
        .execute_transactions_parallel(
            (),
            &transactions,
            &data_view,
            Arc::new(ExecutableStore::<KeyType<K>, ExecutableTestType>::default()),
        );

        if module_access.0 && module_access.1 {
            assert_eq!(output.unwrap_err(), Error::ModulePathReadWrite);
            continue;
        }

        let baseline = ExpectedOutput::generate_baseline(&transactions, None);
        baseline.assert_output(&output);
    }
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
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false));
    }

    #[test]
    fn abort_only(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false));
    }

    #[test]
    fn skip_rest_only(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false));
    }

    #[test]
    fn mixed_transactions(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false));
    }

    #[test]
    fn dynamic_read_writes_mixed(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGen<[u8;32]>>(TransactionGenParams::new_dynamic()), 3000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false));
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

    run_transactions(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        (false, false),
    );
}

#[test]
fn deltas_writes_mixed() {
    let mut runner = TestRunner::default();
    let num_txns = 1000;

    let universe = vec(any::<[u8; 32]>(), 50)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    // Do not allow deletions as resolver can't apply delta to a deleted aggregator.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_with_deltas(&universe, 15, false))
        .collect();

    let data_view = DeltaDataView::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
        phantom: PhantomData,
    };

    for _ in 0..20 {
        let output = BlockExecutor::<
            Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            ExecutableTestType,
        >::new(num_cpus::get())
        .execute_transactions_parallel(
            (),
            &transactions,
            &data_view,
            Arc::new(ExecutableStore::<KeyType<[u8; 32]>, ExecutableTestType>::default()),
        );

        let baseline = ExpectedOutput::generate_baseline(&transactions, None);
        baseline.assert_output(&output);
    }
}

#[test]
fn deltas_resolver() {
    let mut runner = TestRunner::default();
    let num_txns = 1000;

    let universe = vec(any::<[u8; 32]>(), 50)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    let data_view = DeltaDataView::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
        phantom: PhantomData,
    };

    // Do not allow deletes as that would panic in resolver.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_with_deltas(&universe, 15, false))
        .collect();

    for _ in 0..20 {
        let output = BlockExecutor::<
            Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            ExecutableTestType,
        >::new(num_cpus::get())
        .execute_transactions_parallel(
            (),
            &transactions,
            &data_view,
            Arc::new(ExecutableStore::<KeyType<[u8; 32]>, ExecutableTestType>::default()),
        );

        let delta_writes = output
            .as_ref()
            .expect("Must be success")
            .iter()
            .map(|out| out.delta_writes())
            .collect();

        let baseline = ExpectedOutput::generate_baseline(&transactions, Some(delta_writes));
        baseline.assert_output(&output);
    }
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

    run_transactions(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        (false, false),
    );
}

#[test]
fn module_publishing_fallback() {
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

    run_transactions(
        &universe,
        transaction_gen.clone(),
        vec![],
        vec![],
        2,
        (false, true),
    );
    run_transactions(
        &universe,
        transaction_gen.clone(),
        vec![],
        vec![],
        2,
        (false, true),
    );
    run_transactions(&universe, transaction_gen, vec![], vec![], 2, (true, true));
}

fn publishing_fixed_params() {
    let mut runner = TestRunner::default();
    let num_txns = 300;

    let universe = vec(any::<[u8; 32]>(), 50)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();
    let indices = vec(any::<Index>(), 4)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();

    // First 12 keys are normal paths, next 14 are module reads, then writes.
    let mut transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_disjoint_module_rw(&universe[0..40], 12, 26))
        .collect();

    // Adjust the writes of txn indices[0] to contain module write to key 42.
    let w_index = indices[0].index(num_txns);
    *transactions.get_mut(w_index).unwrap() = match transactions.get_mut(w_index).unwrap() {
        Transaction::Write {
            incarnation,
            reads,
            writes_and_deltas,
        } => {
            let mut new_writes_and_deltas = vec![];
            for (incarnation_writes, incarnation_deltas) in writes_and_deltas {
                assert!(!incarnation_writes.is_empty());
                let val = incarnation_writes[0].1.clone();
                let insert_idx = indices[1].index(incarnation_writes.len());
                incarnation_writes.insert(insert_idx, (KeyType(universe[42], true), val));
                new_writes_and_deltas
                    .push((incarnation_writes.clone(), incarnation_deltas.clone()));
            }

            Transaction::Write {
                incarnation: incarnation.clone(),
                reads: reads.clone(),
                writes_and_deltas: new_writes_and_deltas,
            }
        },
        _ => {
            unreachable!();
        },
    };

    let data_view = DeltaDataView::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
        phantom: PhantomData,
    };

    // Confirm still no intersection
    let output = BlockExecutor::<
        Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
        Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
        DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
        ExecutableTestType,
    >::new(num_cpus::get())
    .execute_transactions_parallel(
        (),
        &transactions,
        &data_view,
        Arc::new(ExecutableStore::<KeyType<[u8; 32]>, ExecutableTestType>::default()),
    );
    assert_ok!(output);

    // Adjust the reads of txn indices[2] to contain module read to key 42.
    let r_index = indices[2].index(num_txns);
    *transactions.get_mut(r_index).unwrap() = match transactions.get_mut(r_index).unwrap() {
        Transaction::Write {
            incarnation,
            reads,
            writes_and_deltas,
        } => {
            let mut new_reads = vec![];
            for incarnation_reads in reads {
                assert!(!incarnation_reads.is_empty());
                let insert_idx = indices[3].index(incarnation_reads.len());
                incarnation_reads.insert(insert_idx, KeyType(universe[42], true));
                new_reads.push(incarnation_reads.clone());
            }

            Transaction::Write {
                incarnation: incarnation.clone(),
                reads: new_reads,
                writes_and_deltas: writes_and_deltas.clone(),
            }
        },
        _ => {
            unreachable!();
        },
    };

    for _ in 0..200 {
        let output = BlockExecutor::<
            Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            ExecutableTestType,
        >::new(num_cpus::get())
        .execute_transactions_parallel(
            (),
            &transactions,
            &data_view,
            Arc::new(ExecutableStore::<KeyType<[u8; 32]>, ExecutableTestType>::default()),
        );

        assert_eq!(output.unwrap_err(), Error::ModulePathReadWrite);
    }
}

#[test]
// Test a single transaction intersection interleaves with a lot of dependencies and
// not overlapping module r/w keys.
fn module_publishing_races() {
    for _ in 0..5 {
        publishing_fixed_params();
    }
}
