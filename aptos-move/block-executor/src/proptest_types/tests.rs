// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    errors::SequentialBlockExecutionError,
    executor::BlockExecutor,
    proptest_types::{
        baseline::BaselineOutput,
        types::{
            DeltaDataView, KeyType, MockEvent, MockOutput, MockTask, MockTransaction,
            NonEmptyGroupDataView, TransactionGen, TransactionGenParams, MAX_GAS_PER_TXN,
        },
    },
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::TransactionEvent,
    state_store::MockStateView,
};
use claims::{assert_matches, assert_ok};
use num_cpus;
use proptest::{
    collection::vec,
    prelude::*,
    sample::Index,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use rand::Rng;
use std::{cmp::max, fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};
use test_case::test_case;

fn run_transactions<K, V, E>(
    key_universe: &[K],
    transaction_gens: Vec<TransactionGen<V>>,
    abort_transactions: Vec<Index>,
    skip_rest_transactions: Vec<Index>,
    num_repeat: usize,
    module_access: (bool, bool),
    maybe_block_gas_limit: Option<u64>,
) where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + 'static,
    V: Clone + Eq + Send + Sync + Arbitrary + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
    Vec<u8>: From<V>,
{
    let mut transactions: Vec<_> = transaction_gens
        .into_iter()
        .map(|txn_gen| txn_gen.materialize(key_universe, module_access))
        .collect();

    let length = transactions.len();
    for i in abort_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = MockTransaction::Abort;
    }
    for i in skip_rest_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = MockTransaction::SkipRest(0);
    }

    let state_view = MockStateView::empty();

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    let txn_provider = DefaultTxnProvider::new(transactions);
    for _ in 0..num_repeat {
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<K>, E>,
            MockTask<KeyType<K>, E>,
            MockStateView<KeyType<K>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<K>, E>, usize>,
            DefaultTxnProvider<MockTransaction<KeyType<K>, E>>,
        >::new(
            BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), maybe_block_gas_limit),
            executor_thread_pool.clone(),
            None,
        )
        .execute_transactions_parallel(
            &txn_provider,
            &state_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        );

        if module_access.0 && module_access.1 {
            assert_matches!(output, Err(()));
            continue;
        }

        BaselineOutput::generate(txn_provider.get_txns(), maybe_block_gas_limit)
            .assert_parallel_output(&output);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn no_early_termination(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 4000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn abort_only(
        universe in vec(any::<[u8; 32]>(), 80),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 300).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn skip_rest_only(
        universe in vec(any::<[u8; 32]>(), 80),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 300).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn mixed_transactions(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn dynamic_read_writes_mixed(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGen<[u8;32]>>(TransactionGenParams::new_dynamic()), 3000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }
}

fn dynamic_read_writes_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 100)
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

    run_transactions::<[u8; 32], [u8; 32], MockEvent>(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        (false, false),
        maybe_block_gas_limit,
    );
}

fn deltas_writes_mixed_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

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
    let txn_provider = DefaultTxnProvider::new(transactions);

    let data_view = DeltaDataView::<KeyType<[u8; 32]>> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..20 {
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<[u8; 32]>, MockEvent>,
            MockTask<KeyType<[u8; 32]>, MockEvent>,
            DeltaDataView<KeyType<[u8; 32]>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
            DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
        >::new(
            BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), maybe_block_gas_limit),
            executor_thread_pool.clone(),
            None,
        )
        .execute_transactions_parallel(
            &txn_provider,
            &data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        );

        BaselineOutput::generate(txn_provider.get_txns(), maybe_block_gas_limit)
            .assert_parallel_output(&output);
    }
}

fn deltas_resolver_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

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

    let data_view = DeltaDataView::<KeyType<[u8; 32]>> {
        phantom: PhantomData,
    };

    // Do not allow deletes as that would panic in resolver.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_with_deltas(&universe, 15, false))
        .collect();
    let txn_provider = DefaultTxnProvider::new(transactions);

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..20 {
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<[u8; 32]>, MockEvent>,
            MockTask<KeyType<[u8; 32]>, MockEvent>,
            DeltaDataView<KeyType<[u8; 32]>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
            DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
        >::new(
            BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), maybe_block_gas_limit),
            executor_thread_pool.clone(),
            None,
        )
        .execute_transactions_parallel(
            &txn_provider,
            &data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        );

        BaselineOutput::generate(txn_provider.get_txns(), maybe_block_gas_limit)
            .assert_parallel_output(&output);
    }
}

fn dynamic_read_writes_contended_with_block_gas_limit(
    num_txns: usize,
    maybe_block_gas_limit: Option<u64>,
) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 10)
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

    run_transactions::<[u8; 32], [u8; 32], MockEvent>(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        (false, false),
        maybe_block_gas_limit,
    );
}

fn publishing_fixed_params_with_block_gas_limit(
    num_txns: usize,
    maybe_block_gas_limit: Option<u64>,
) {
    let mut runner = TestRunner::default();

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
    match transactions.get_mut(w_index).unwrap() {
        MockTransaction::Write {
            incarnation_counter: _,
            incarnation_behaviors,
        } => {
            incarnation_behaviors.iter_mut().for_each(|behavior| {
                assert!(!behavior.writes.is_empty());
                let insert_idx = indices[1].index(behavior.writes.len());
                let val = behavior.writes[0].1.clone();
                behavior
                    .writes
                    .insert(insert_idx, (KeyType(universe[42], true), val));
            });
        },
        _ => {
            unreachable!();
        },
    };

    let data_view = DeltaDataView::<KeyType<[u8; 32]>> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    let txn_provider = DefaultTxnProvider::new(transactions.clone());
    // Confirm still no intersection
    let mut guard = AptosModuleCacheManagerGuard::none();
    let output = BlockExecutor::<
        MockTransaction<KeyType<[u8; 32]>, MockEvent>,
        MockTask<KeyType<[u8; 32]>, MockEvent>,
        DeltaDataView<KeyType<[u8; 32]>>,
        NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
        DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
    >::new(
        BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), maybe_block_gas_limit),
        executor_thread_pool,
        None,
    )
    .execute_transactions_parallel(
        &txn_provider,
        &data_view,
        &TransactionSliceMetadata::unknown(),
        &mut guard,
    );
    assert_ok!(output);

    // Adjust the reads of txn indices[2] to contain module read to key 42.
    let r_index = indices[2].index(num_txns);
    match transactions.get_mut(r_index).unwrap() {
        MockTransaction::Write {
            incarnation_counter: _,
            incarnation_behaviors,
        } => {
            incarnation_behaviors.iter_mut().for_each(|behavior| {
                assert!(!behavior.reads.is_empty());
                let insert_idx = indices[3].index(behavior.reads.len());
                behavior
                    .reads
                    .insert(insert_idx, KeyType(universe[42], true));
            });
        },
        _ => {
            unreachable!();
        },
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    let txn_provider = DefaultTxnProvider::new(transactions);
    for _ in 0..200 {
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<[u8; 32]>, MockEvent>,
            MockTask<KeyType<[u8; 32]>, MockEvent>,
            DeltaDataView<KeyType<[u8; 32]>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
            DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
        >::new(
            BlockExecutorConfig::new_maybe_block_limit(
                num_cpus::get(),
                Some(max(w_index, r_index) as u64 * MAX_GAS_PER_TXN + 1),
            ),
            executor_thread_pool.clone(),
            None,
        ) // Ensure enough gas limit to commit the module txns (4 is maximum gas per txn)
        .execute_transactions_parallel(
            &txn_provider,
            &data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        );

        assert_matches!(output, Err(()));
    }
}

#[test_case(1000, 100, 30, 15, 0)]
#[test_case(1000, 50, 20, 10, 0)]
#[test_case(1000, 15, 5, 5, 0)]
#[test_case(1000, 20, 10, 5, 1)]
#[test_case(1000, 20, 10, 5, 2)]
#[test_case(1000, 20, 10, 5, 3)]
#[test_case(1000, 20, 10, 5, 4)]
fn non_empty_group(
    num_txns: usize,
    key_universe_len: usize,
    num_repeat_parallel: usize,
    num_repeat_sequential: usize,
    group_size_testing: usize,
) {
    let mut runner = TestRunner::default();

    let key_universe = vec(any::<[u8; 32]>(), key_universe_len)
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

    // Determines the probability that any given incarnation of an executed txn will query
    // the size of a given group (3 groups).
    let group_size_pcts = match group_size_testing {
        0 => [None, None, None],
        1 => [Some(30), None, None],
        2 => [Some(80), None, None],
        3 => [Some(30), Some(80), None],
        4 => [Some(30), Some(50), Some(70)],
        _ => unreachable!("Unexpected test configuration"),
    };

    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            txn_gen.materialize_groups::<[u8; 32], MockEvent>(&key_universe, group_size_pcts)
        })
        .collect();
    let txn_provider = DefaultTxnProvider::new(transactions);

    let data_view = NonEmptyGroupDataView::<KeyType<[u8; 32]>> {
        group_keys: key_universe[(key_universe_len - 3)..key_universe_len]
            .iter()
            .map(|k| KeyType(*k, false))
            .collect(),
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..num_repeat_parallel {
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<[u8; 32]>, MockEvent>,
            MockTask<KeyType<[u8; 32]>, MockEvent>,
            NonEmptyGroupDataView<KeyType<[u8; 32]>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
            DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
        >::new(
            BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
            executor_thread_pool.clone(),
            None,
        )
        .execute_transactions_parallel(
            &txn_provider,
            &data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        );

        BaselineOutput::generate(txn_provider.get_txns(), None).assert_parallel_output(&output);
    }

    for _ in 0..num_repeat_sequential {
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<[u8; 32]>, MockEvent>,
            MockTask<KeyType<[u8; 32]>, MockEvent>,
            NonEmptyGroupDataView<KeyType<[u8; 32]>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
            DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
        >::new(
            BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
            executor_thread_pool.clone(),
            None,
        )
        .execute_transactions_sequential(
            &txn_provider,
            &data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
            false,
        );
        // TODO: test dynamic disabled as well.

        BaselineOutput::generate(txn_provider.get_txns(), None).assert_output(&output.map_err(
            |e| match e {
                SequentialBlockExecutionError::ResourceGroupSerializationError => {
                    panic!("Unexpected error")
                },
                SequentialBlockExecutionError::ErrorToReturn(err) => err,
            },
        ));
    }
}

#[test]
fn dynamic_read_writes() {
    dynamic_read_writes_with_block_gas_limit(3000, None);
}

#[test]
fn deltas_writes_mixed() {
    deltas_writes_mixed_with_block_gas_limit(1000, None);
}

#[test]
fn deltas_resolver() {
    deltas_resolver_with_block_gas_limit(1000, None);
}

#[test]
fn dynamic_read_writes_contended() {
    dynamic_read_writes_contended_with_block_gas_limit(1000, None);
}

// TODO(loader_v2): Fix this test.
#[test]
#[ignore]
// Test a single transaction intersection interleaves with a lot of dependencies and
// not overlapping module r/w keys.
fn module_publishing_races() {
    for _ in 0..5 {
        publishing_fixed_params_with_block_gas_limit(300, None);
    }
}

// The following set of tests are the same tests as above with per-block gas limit.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn no_early_termination_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn abort_only_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 10).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 10 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn skip_rest_only_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn mixed_transactions_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn dynamic_read_writes_mixed_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGen<[u8;32]>>(TransactionGenParams::new_dynamic()), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }
}

#[test]
fn dynamic_read_writes_with_block_gas_limit_test() {
    dynamic_read_writes_with_block_gas_limit(
        3000,
        // TODO: here and below, use proptest randomness, not thread_rng.
        Some(rand::thread_rng().gen_range(0, 3000) as u64),
    );
    dynamic_read_writes_with_block_gas_limit(3000, Some(0));
}

#[test]
fn deltas_writes_mixed_with_block_gas_limit_test() {
    deltas_writes_mixed_with_block_gas_limit(
        1000,
        Some(rand::thread_rng().gen_range(0, 1000) as u64),
    );
    deltas_writes_mixed_with_block_gas_limit(1000, Some(0));
}

#[test]
fn deltas_resolver_with_block_gas_limit_test() {
    deltas_resolver_with_block_gas_limit(
        1000,
        Some(rand::thread_rng().gen_range(0, 1000 * MAX_GAS_PER_TXN / 2)),
    );
    deltas_resolver_with_block_gas_limit(1000, Some(0));
}

#[test]
fn dynamic_read_writes_contended_with_block_gas_limit_test() {
    dynamic_read_writes_contended_with_block_gas_limit(
        1000,
        Some(rand::thread_rng().gen_range(0, 1000) as u64),
    );
    dynamic_read_writes_contended_with_block_gas_limit(1000, Some(0));
}

// TODO(loader_v2): Fix this test.
#[test]
#[ignore]
// Test a single transaction intersection interleaves with a lot of dependencies and
// not overlapping module r/w keys.
fn module_publishing_races_with_block_gas_limit_test() {
    for _ in 0..5 {
        publishing_fixed_params_with_block_gas_limit(
            300,
            Some(rand::thread_rng().gen_range(0, 300 * MAX_GAS_PER_TXN / 2)),
        );
    }
}
