// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    combinatorial_tests::{
        baseline::BaselineOutput,
        mock_executor::{MockEvent, MockOutput, MockTask},
        resource_tests::{
            create_executor_thread_pool, execute_block_parallel, generate_test_data,
            get_gas_limit_variants,
        },
        types::{
            KeyType, MockTransaction, MockTransactionBuilder, NonEmptyGroupDataView,
            PerGroupConfig, TransactionGenData, TransactionGenParams, RESERVED_TAG,
            DeltaHoldingTag, STORAGE_AGGREGATOR_VALUE,
        },
    },
    errors::SequentialBlockExecutionError,
    executor::BlockExecutor,
    task::ExecutorTask,
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_types::block_executor::{
    config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
};
use proptest::{collection::vec, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use std::sync::Arc;
use test_case::test_case;

/// Run both parallel and sequential execution tests for a transaction provider
pub(crate) fn run_tests_with_groups(
    executor_thread_pool: Arc<rayon::ThreadPool>,
    gas_limits: Vec<Option<u64>>,
    transactions: Vec<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
    data_view: &NonEmptyGroupDataView<KeyType<[u8; 32]>>,
    num_executions_parallel: usize,
    num_executions_sequential: usize,
) {
    let txn_provider = DefaultTxnProvider::new_without_info(transactions);

    // Run parallel execution tests
    for block_stm_v2 in [false, true] {
        for i in 0..num_executions_parallel {
            for maybe_block_gas_limit in &gas_limits {
                if *maybe_block_gas_limit == Some(0) && i > 0 {
                    continue;
                }

                let output = execute_block_parallel::<
                    MockTransaction<KeyType<[u8; 32]>, MockEvent>,
                    NonEmptyGroupDataView<KeyType<[u8; 32]>>,
                    DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
                >(
                    executor_thread_pool.clone(),
                    *maybe_block_gas_limit,
                    &txn_provider,
                    data_view,
                    None,
                    block_stm_v2,
                );

                BaselineOutput::generate(txn_provider.get_txns(), *maybe_block_gas_limit)
                    .assert_parallel_output(&output);
            }
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

// TODO(BlockSTMv2): split number of test runs based on number of random generations and
// number of executions per generated randomness.
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

    let (universe, transaction_gen) = generate_test_data(
        &mut local_runner,
        universe_size,
        transaction_count,
        TransactionGenParams::new_dynamic(),
    );

    // Group size percentages for 3 groups
    let group_size_pcts = [group_size_pct1, group_size_pct2, group_size_pct3];
    let group_keys = &universe[(universe_size - 3)..universe_size];
    let group_config: Vec<_> = group_keys
        .iter()
        .zip(group_size_pcts)
        .map(|(key, percentage)| PerGroupConfig {
            key: *key,
            query_percentage: percentage,
            // For these tests, deltas are not enabled, but we still need a default
            // tag in the group to ensure it's not empty.
            tags_with_deltas: vec![DeltaHoldingTag {
                tag: RESERVED_TAG,
                base_value: STORAGE_AGGREGATOR_VALUE,
            }],
        })
        .collect();

    let transactions = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &universe)
                .with_groups(group_config.clone())
                .build()
        })
        .collect();

    let data_view = NonEmptyGroupDataView {
        group_keys_with_delta_tags: group_config
            .into_iter()
            .map(|cfg| (KeyType(cfg.key), cfg.tags_with_deltas))
            .collect(),
        delayed_field_testing: false,
    };
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
