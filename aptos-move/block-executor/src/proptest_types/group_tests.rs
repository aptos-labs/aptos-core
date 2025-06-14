// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    errors::SequentialBlockExecutionError,
    executor::BlockExecutor,
    proptest_types::{
        baseline::BaselineOutput,
        mock_executor::{MockEvent, MockOutput, MockTask},
        resource_tests::{
            create_executor_thread_pool, execute_block_parallel, get_gas_limit_variants,
        },
        types::{
            KeyType, MockTransaction, NonEmptyGroupDataView, TransactionGen, TransactionGenParams,
        },
    },
    task::ExecutorTask,
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_types::block_executor::{
    config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
};
use num_cpus;
use proptest::{collection::vec, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use std::sync::Arc;
use test_case::test_case;

/// Create a data view for testing with non-empty groups
pub(crate) fn create_non_empty_group_data_view(
    key_universe: &[[u8; 32]],
    universe_size: usize,
    delayed_field_testing: bool,
) -> NonEmptyGroupDataView<KeyType<[u8; 32]>> {
    NonEmptyGroupDataView::<KeyType<[u8; 32]>> {
        group_keys: key_universe[(universe_size - 3)..universe_size]
            .iter()
            .map(|k| KeyType(*k))
            .collect(),
        delayed_field_testing,
    }
}

/// Run both parallel and sequential execution tests for a transaction provider
pub(crate) fn run_tests_with_groups(
    executor_thread_pool: Arc<rayon::ThreadPool>,
    gas_limits: Vec<Option<u64>>,
    transactions: Vec<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
    data_view: &NonEmptyGroupDataView<KeyType<[u8; 32]>>,
    num_executions_parallel: usize,
    num_executions_sequential: usize,
) {
    let txn_provider = DefaultTxnProvider::new(transactions);

    // Run parallel execution tests
    for maybe_block_gas_limit in gas_limits {
        for _ in 0..num_executions_parallel {
            let output = execute_block_parallel::<
                MockTransaction<KeyType<[u8; 32]>, MockEvent>,
                NonEmptyGroupDataView<KeyType<[u8; 32]>>,
                DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
            >(
                executor_thread_pool.clone(),
                maybe_block_gas_limit,
                &txn_provider,
                data_view,
                None,
            );

            BaselineOutput::generate(txn_provider.get_txns(), maybe_block_gas_limit)
                .assert_parallel_output(&output);
        }
    }

    // Run sequential execution tests
    for _ in 0..num_executions_sequential {
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
            data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
            false,
        );

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

// TODO: Change some tests (e.g. second and fifth) to use gas limit: needs to handle error in mock executor.
#[test_case(50, 100, None, None, None, false, 30, 15 ; "basic group test")]
#[test_case(50, 1000, None, None, None, false, 20, 10 ; "basic group test 2")]
#[test_case(50, 1000, None, None, None, true, 20, 10 ; "basic group test 2 with gas limit")]
#[test_case(15, 1000, None, None, None, false, 5, 5 ; "small universe group test")]
#[test_case(20, 1000, Some(30), None, None, false, 10, 5 ; "group size pct1=30%")]
#[test_case(20, 1000, Some(80), None, None, false, 10, 5 ; "group size pct1=80%")]
#[test_case(20, 1000, Some(80), None, None, true, 10, 5 ; "group size pct1=80% with gas limit")]
#[test_case(20, 1000, Some(30), Some(80), None, false, 10, 5 ; "group size pct1=30%, pct2=80%")]
#[test_case(20, 1000, Some(30), Some(50), Some(70), false, 10, 5 ; "group size pct1=30%, pct2=50%, pct3=70%")]
fn non_empty_group_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    group_size_pct1: Option<u8>,
    group_size_pct2: Option<u8>,
    group_size_pct3: Option<u8>,
    use_gas_limit: bool,
    num_executions_parallel: usize,
    num_executions_sequential: usize,
) where
    MockTask<KeyType<[u8; 32]>, MockEvent>:
        ExecutorTask<Txn = MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
{
    let mut local_runner = TestRunner::default();

    let key_universe = vec(any::<[u8; 32]>(), universe_size)
        .new_tree(&mut local_runner)
        .expect("creating a new value should succeed")
        .current();

    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        transaction_count,
    )
    .new_tree(&mut local_runner)
    .expect("creating a new value should succeed")
    .current();

    // Group size percentages for 3 groups
    let group_size_pcts = [group_size_pct1, group_size_pct2, group_size_pct3];
    let transactions = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            txn_gen.materialize_groups::<[u8; 32], MockEvent>(&key_universe, group_size_pcts, None)
        })
        .collect();

    let data_view = create_non_empty_group_data_view(&key_universe, universe_size, false);
    let executor_thread_pool = create_executor_thread_pool();
    let gas_limits = get_gas_limit_variants(use_gas_limit, transaction_count);

    run_tests_with_groups(
        executor_thread_pool,
        gas_limits,
        transactions,
        &data_view,
        num_executions_parallel,
        num_executions_sequential,
    );
}
