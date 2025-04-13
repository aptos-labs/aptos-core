// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    proptest_types::{
        group_tests::{
            create_mock_transactions, create_non_empty_group_data_view, run_tests_with_groups,
        },
        resource_tests::{create_executor_thread_pool, get_gas_limit_variants},
        types::{KeyType, MockEvent, MockTask, MockTransaction, TransactionGen, TransactionGenParams},
    },
    task::ExecutorTask,
};
use fail::FailScenario;
use proptest::{collection::vec, prelude::*, test_runner::TestRunner, strategy::ValueTree};
use std::cmp::min;
use test_case::test_case;

#[test_case(50, 100, false, false, 30, 15 ; "basic delayed field test_v1")]
#[test_case(50, 1000, false, false, 20, 10 ; "longer delayed field test_v1")]
#[test_case(50, 1000, true, false, 20, 10 ; "delayed field test with gas limit_v1")]
#[test_case(15, 1000, false, false, 5, 5 ; "small universe delayed field test_v1")]
#[test_case(50, 100, false, true, 30, 15 ; "basic delayed field test_v2")]
#[test_case(50, 1000, false, true, 20, 10 ; "longer delayed field test_v2")]
#[test_case(50, 1000, true, true, 20, 10 ; "delayed field test with gas limit_v2")]
#[test_case(15, 1000, false, true, 5, 5 ; "small universe delayed field test_v2")]
fn delayed_field_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    block_stm_v2: bool,
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
    let transactions = create_mock_transactions::<KeyType<[u8; 32]>>(
        transaction_gen,
        &key_universe,
        group_size_pcts,
        Some(delta_threshold),
    );

    let data_view = create_non_empty_group_data_view(&key_universe, universe_size);

    let executor_thread_pool = create_executor_thread_pool();

    let gas_limits = get_gas_limit_variants(use_gas_limit, transaction_count);
    
    run_tests_with_groups(
        executor_thread_pool,
        gas_limits,
        block_stm_v2,
        transactions,
        &data_view,
        num_executions_parallel,
        num_executions_sequential,
    );

    // Tear down the failpoint scenario
    scenario.teardown();
} 