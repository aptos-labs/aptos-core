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
            DeltaDataView, DeltaTestKind, KeyType, MockEvent, MockOutput, MockTask,
            MockTransaction, MockTransactionBuilder, NonEmptyGroupDataView, PerGroupConfig,
            TransactionGenData, TransactionGenParams, MAX_GAS_PER_TXN, RESERVED_TAG,
            DeltaHoldingTag, STORAGE_AGGREGATOR_VALUE,
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
    transaction_gens: Vec<TransactionGenData<V>>,
    abort_transactions: Vec<Index>,
    skip_rest_transactions: Vec<Index>,
    num_repeat: usize,
    maybe_block_gas_limit: Option<u64>,
) where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + 'static,
    V: Clone + Eq + Send + Sync + Arbitrary + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
    Vec<u8>: From<V>,
{
    let mut transactions: Vec<_> = transaction_gens
        .into_iter()
        .map(|txn_gen| MockTransactionBuilder::new(txn_gen, key_universe).build())
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

    let txn_provider = DefaultTxnProvider::new_without_info(transactions);
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
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 4000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, None);
    }

    #[test]
    fn abort_only(
        universe in vec(any::<[u8; 32]>(), 80),
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 300).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, None);
    }

    #[test]
    fn skip_rest_only(
        universe in vec(any::<[u8; 32]>(), 80),
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 300).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, None);
    }

    #[test]
    fn mixed_transactions(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, None);
    }

    #[test]
    fn dynamic_read_writes_mixed(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGenData<[u8;32]>>(TransactionGenParams::new_dynamic()), 3000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, None);
    }
}

fn dynamic_read_writes_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

    let (universe, transaction_gen) = generate_test_data(
        &mut runner,
        100,
        num_txns,
        TransactionGenParams::new_dynamic(),
    );

    run_transactions::<[u8; 32], [u8; 32], MockEvent>(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        maybe_block_gas_limit,
    );
}

fn deltas_writes_mixed_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

    let params = TransactionGenParams::new_dynamic().with_no_deletions();
    let (universe, transaction_gen) =
        generate_test_data(&mut runner, 50, num_txns, params);

    // Do not allow deletions as resolver can't apply delta to a deleted aggregator.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &universe)
                .with_deltas(DeltaTestKind::AggregatorV1)
                .build()
        })
        .collect();
    let txn_provider = DefaultTxnProvider::new_without_info(transactions);

    let data_view = DeltaDataView::<KeyType<[u8; 32]>> {
        initial_values: HashMap::new(),
        default_base_value: STORAGE_AGGREGATOR_VALUE,
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

    let params = TransactionGenParams::new_dynamic().with_no_deletions();
    let (universe, transaction_gen) =
        generate_test_data(&mut runner, 50, num_txns, params);

    let data_view = DeltaDataView::<KeyType<[u8; 32]>> {
        initial_values: HashMap::new(),
        default_base_value: STORAGE_AGGREGATOR_VALUE,
        phantom: PhantomData,
    };

    // Do not allow deletes as that would panic in resolver.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &universe)
                .with_deltas(DeltaTestKind::AggregatorV1)
                .build()
        })
        .collect();
    let txn_provider = DefaultTxnProvider::new_without_info(transactions);

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

    let (universe, transaction_gen) = generate_test_data(
        &mut runner,
        10,
        num_txns,
        TransactionGenParams::new_dynamic(),
    );

    run_transactions::<[u8; 32], [u8; 32], MockEvent>(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        maybe_block_gas_limit,
    );
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

    let (key_universe, transaction_gen) = generate_test_data(
        &mut runner,
        key_universe_len,
        num_txns,
        TransactionGenParams::new_dynamic(),
    );

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

    let group_keys = &key_universe[(key_universe_len - 3)..key_universe_len];
    let group_config: Vec<_> = group_keys
        .iter()
        .zip(group_size_pcts)
        .map(|(key, percentage)| PerGroupConfig {
            key: *key,
            query_percentage: percentage,
            tags_with_deltas: vec![DeltaHoldingTag {
                tag: RESERVED_TAG,
                base_value: STORAGE_AGGREGATOR_VALUE,
            }],
        })
        .collect();

    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &key_universe)
                .with_groups(group_config.clone())
                .with_delayed_fields_testing()
                .build()
        })
        .collect();
    let txn_provider = DefaultTxnProvider::new_without_info(transactions);

    let data_view = NonEmptyGroupDataView::<KeyType<[u8; 32]>> {
        group_keys_with_delta_tags: group_config
            .into_iter()
            .map(|cfg| (KeyType(cfg.key), cfg.tags_with_deltas))
            .collect(),
        delayed_field_testing: true,
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
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn abort_only_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 10).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, Some(rand::thread_rng().gen_range(0, 10 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn skip_rest_only_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn mixed_transactions_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGenData<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
    }

    #[test]
    fn dynamic_read_writes_mixed_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGenData<[u8;32]>>(TransactionGenParams::new_dynamic()), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions::<[u8; 32], [u8; 32], MockEvent>(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, Some(rand::thread_rng().gen_range(0, 5000 * MAX_GAS_PER_TXN / 2)));
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
