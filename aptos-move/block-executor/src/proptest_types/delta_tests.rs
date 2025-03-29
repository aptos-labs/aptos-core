// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    proptest_types::{
        baseline::BaselineOutput,
        resource_tests::{create_executor_thread_pool, execute_block_parallel, get_gas_limit_variants, generate_universe_and_transactions},
        types::{
            DeltaDataView, KeyType, MockEvent, MockTask, MockTransaction, 
        },
    },
    task::ExecutorTask,
    txn_provider::default::DefaultTxnProvider,
};
use proptest::test_runner::TestRunner;
use std::marker::PhantomData;
use test_case::test_case;

// TODO(BlockSTMv2): Unify with delayed field testing (mock value exchange w. failpoint).
fn run_transactions_deltas(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    block_stm_v2: bool,
    num_executions: usize,
    num_random_generations: usize,
) {
    let executor_thread_pool = create_executor_thread_pool();

    // The delta threshold controls how many keys / paths are guaranteed r/w resources even in the
    // presence of deltas.
    let delta_threshold = std::cmp::min(15, universe_size / 2);

    for _ in 0..num_random_generations {
        let mut local_runner = TestRunner::default();

        let (universe, transaction_gen) = generate_universe_and_transactions(
            &mut local_runner,
            universe_size,
            transaction_count,
            true,
        );

        // Do not allow deletions as resolver can't apply delta to a deleted aggregator.
        let transactions: Vec<MockTransaction<KeyType<[u8; 32]>, MockEvent>> = transaction_gen
            .into_iter()
            .map(|txn_gen| txn_gen.materialize_with_deltas(&universe, delta_threshold, false))
            .collect();
        let txn_provider = DefaultTxnProvider::new(transactions);

        let data_view = DeltaDataView::<KeyType<[u8; 32]>> {
            phantom: PhantomData,
        };

        let gas_limits = get_gas_limit_variants(use_gas_limit, transaction_count);

        for maybe_block_gas_limit in gas_limits {
            for _ in 0..num_executions {
                let output = execute_block_parallel::<
                    MockTransaction<KeyType<[u8; 32]>, MockEvent>,
                    DeltaDataView<KeyType<[u8; 32]>>,
                    DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
                >(
                    executor_thread_pool.clone(),
                    maybe_block_gas_limit,
                    block_stm_v2,
                    &txn_provider,
                    &data_view,
                );

                BaselineOutput::generate(txn_provider.get_txns(), maybe_block_gas_limit)
                    .assert_parallel_output(&output);
            }
        }
    }
}

#[test_case(50, 1000, false, false, 10, 2 ; "basic deltas")]
#[test_case(10, 1000, false, false, 10, 2 ; "deltas with small universe")]
#[test_case(50, 1000, true, false, 10, 2 ; "deltas with gas limit")]
#[test_case(10, 1000, true, false, 10, 2 ; "deltas with small universe with gas limit")]
#[test_case(50, 1000, false, true, 10, 2 ; "basic deltas with v2")]
#[test_case(10, 1000, false, true, 10, 2 ; "deltas with small universe with v2")]
#[test_case(50, 1000, true, true, 10, 2 ; "deltas with gas limit with v2")]
#[test_case(10, 1000, true, true, 10, 2 ; "deltas with small universe with gas limit with v2")]
fn deltas_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    block_stm_v2: bool,
    num_executions: usize,
    num_random_generations: usize,
) where
    MockTask<KeyType<[u8; 32]>, MockEvent>:
        ExecutorTask<Txn = MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
{
    run_transactions_deltas(
        universe_size,
        transaction_count,
        use_gas_limit,
        block_stm_v2,
        num_executions,
        num_random_generations,
    );
}