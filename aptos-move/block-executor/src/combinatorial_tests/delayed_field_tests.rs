// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    combinatorial_tests::{
        group_tests::run_tests_with_groups,
        mock_executor::{MockEvent, MockTask},
        resource_tests::{
            create_executor_thread_pool, generate_test_data, get_gas_limit_variants,
        },
        types::{
            DeltaHoldingTag, DeltaTestKind, KeyType, MockTransaction, MockTransactionBuilder,
            NonEmptyGroupDataView, PerGroupConfig, TransactionGenData, TransactionGenParams,
            RESERVED_TAG, STORAGE_AGGREGATOR_VALUE,
        },
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

    let params = TransactionGenParams::new_dynamic().with_no_deletions();
    let (key_universe, transaction_gen) =
        generate_test_data(&mut local_runner, universe_size, transaction_count, params);

    // Fixed group size percentages and delta threshold.
    let group_size_pcts = [Some(30), Some(50), None];
    let group_keys = &key_universe[(universe_size - 3)..universe_size];
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

    let transactions = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &key_universe)
                .with_groups(group_config.clone())
                .with_deltas(DeltaTestKind::DelayedFields)
                .build()
        })
        .collect();

    let data_view = NonEmptyGroupDataView {
        group_keys_with_delta_tags: group_config
            .into_iter()
            .map(|cfg| (KeyType(cfg.key), cfg.tags_with_deltas))
            .collect(),
        delayed_field_testing: true,
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

    // Tear down the failpoint scenario
    scenario.teardown();
}

#[test]
fn delayed_field_delta_failure_test() {
    let universe_size = 20;
    let transaction_count = 50;

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
        any_with::<TransactionGenData<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        transaction_count,
    )
    .new_tree(&mut local_runner)
    .expect("creating a new value should succeed")
    .current();

    // Configure a group with a base value of u128::MAX to trigger overflow.
    let group_keys = &key_universe[(universe_size - 1)..universe_size];
    let group_config: Vec<_> = group_keys
        .iter()
        .map(|key| PerGroupConfig {
            key: *key,
            query_percentage: None,
            tags_with_deltas: vec![DeltaHoldingTag {
                tag: RESERVED_TAG,
                base_value: u128::MAX,
            }],
        })
        .collect();

    let transactions = transaction_gen
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &key_universe)
                .with_groups(group_config.clone())
                .with_deltas(DeltaTestKind::DelayedFields)
                .build()
        })
        .collect();

    let data_view = NonEmptyGroupDataView {
        group_keys_with_delta_tags: group_config
            .into_iter()
            .map(|cfg| (KeyType(cfg.key), cfg.tags_with_deltas))
            .collect(),
        delayed_field_testing: true,
    };

    let executor_thread_pool = create_executor_thread_pool();
    let gas_limits = vec![None]; // No gas limit for this test.

    run_tests_with_groups(
        executor_thread_pool,
        gas_limits,
        transactions,
        &data_view,
        1, // num_executions_parallel
        1, // num_executions_sequential
    );

    // Tear down the failpoint scenario
    scenario.teardown();
}

#[test]
fn delayed_field_lifecycle_test() {
    let universe_size = 1;
    let transaction_count = 3;

    let scenario = FailScenario::setup();
    fail::cfg("delayed_field_test", "return").expect("Failed to configure failpoint");

    let mut runner = TestRunner::default();
    let (key_universe, _) = generate_test_data(
        &mut runner,
        universe_size,
        0,
        TransactionGenParams::default(),
    );

    // Manually construct transaction generations to test the lifecycle.
    let index = Index::from(0usize);
    let value = [1u8; 32];
    let gen_data = vec![
        // 1. Create the resource
        TransactionGenData {
            reads: vec![vec![]],
            modifications: vec![vec![Modification::Write(index, value)]],
            gas: vec![index],
            metadata_seeds: vec![vec![index, index, index]],
            group_size_indicators: vec![vec![index, index, index]],
        },
        // 2. Delete the resource
        TransactionGenData {
            reads: vec![vec![]],
            modifications: vec![vec![Modification::Deletion(index)]],
            gas: vec![index],
            metadata_seeds: vec![vec![index, index, index]],
            group_size_indicators: vec![vec![index, index, index]],
        },
        // 3. Re-create the resource
        TransactionGenData {
            reads: vec![vec![]],
            modifications: vec![vec![Modification::Write(index, value)]],
            gas: vec![index],
            metadata_seeds: vec![vec![index, index, index]],
            group_size_indicators: vec![vec![index, index, index]],
        },
    ];

    let transactions = gen_data
        .into_iter()
        .map(|txn_gen| {
            MockTransactionBuilder::new(txn_gen, &key_universe)
                .with_deltas(DeltaTestKind::DelayedFields)
                .build()
        })
        .collect();

    let data_view = NonEmptyGroupDataView {
        group_keys_with_delta_tags: HashMap::new(),
        delayed_field_testing: true,
    };

    let executor_thread_pool = create_executor_thread_pool();
    let gas_limits = vec![None];

    run_tests_with_groups(
        executor_thread_pool,
        gas_limits,
        transactions,
        &data_view,
        1, // num_executions_parallel
        1, // num_executions_sequential
    );

    scenario.teardown();
}
