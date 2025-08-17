// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    combinatorial_tests::{
        group_tests::{create_non_empty_group_data_view, run_tests_with_groups},
        mock_executor::{MockEvent, MockTask},
        resource_tests::{create_executor_thread_pool, get_gas_limit_variants},
        types::{KeyType, MockTransaction, TransactionGen, TransactionGenParams},
    },
    task::ExecutorTask,
};
use fail::FailScenario;
use proptest::{collection::vec, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use std::cmp::min;
use test_case::test_case;

#[test_case(50, 100, false, 30, 15 ; "basic delayed field test_v1")]
#[test_case(50, 1000, false, 20, 10 ; "longer delayed field test_v1")]
#[test_case(50, 1000, true, 20, 10 ; "delayed field test with gas limit_v1")]
#[test_case(15, 1000, false, 5, 5 ; "small universe delayed field test_v1")]
fn delayed_field_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    num_executions_parallel: usize,
    num_executions_sequential: usize,
) where
    MockTask<KeyType<[u8; 32]>, MockEvent>:
        ExecutorTask<Txn = MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
{
    // Set up fail point for exchange testing
    let scenario = FailScenario::setup();
    assert!(fail::has_failpoints());
    fail::cfg("delayed_field_test", "return").expect("Failed to configure failpoint");

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

    // Fixed group size percentages and delta threshold.
    let group_size_pcts = [Some(30), Some(50), None];
    let delta_threshold = min(15, universe_size / 2);
    let transactions = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            txn_gen.materialize_groups::<[u8; 32], MockEvent>(
                &key_universe,
                group_size_pcts,
                Some(delta_threshold),
            )
        })
        .collect();

    let data_view = create_non_empty_group_data_view(&key_universe, universe_size, true);

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

    // Tear down the failpoint scenario
    scenario.teardown();
}
