// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    proptest_types::{
        baseline::BaselineOutput,
        resource_tests::{
            create_executor_thread_pool, execute_block_parallel, get_gas_limit_variants,
        },
        mock_executor::{MockEvent, MockTask},
        types::{
            key_to_module_id, KeyType, MockTransaction,
            TransactionGen, TransactionGenParams,
        },
    },
    task::ExecutorTask,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_types::state_store::MockStateView;
use fail::FailScenario;
use move_core_types::language_storage::ModuleId;
use proptest::{collection::vec, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use test_case::test_case;

pub(crate) fn execute_module_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    block_stm_v2: bool,
    modules_only: bool,
    num_executions: usize,
    num_random_generations: usize,
) where
    MockTask<KeyType<[u8; 32]>, MockEvent>:
        ExecutorTask<Txn = MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
{
    let scenario = FailScenario::setup();
    assert!(fail::has_failpoints());
    fail::cfg("module_test", "return").unwrap();

    let executor_thread_pool = create_executor_thread_pool();
    let mut runner = TestRunner::default();

    let gas_limits = get_gas_limit_variants(use_gas_limit, transaction_count);
    for maybe_block_gas_limit in gas_limits {
        // Run the test cases directly
        for _ in 0..num_random_generations {
            // Generate universe
            let universe = vec(any::<[u8; 32]>(), universe_size)
                .new_tree(&mut runner)
                .expect("creating universe should succeed")
                .current();

            // Generate transactions based on parameters
            let transaction_strategy = if modules_only {
                vec(
                    any_with::<TransactionGen<[u8; 32]>>(
                        TransactionGenParams::new_dynamic_modules_only(),
                    ),
                    transaction_count,
                )
            } else {
                vec(
                    any_with::<TransactionGen<[u8; 32]>>(
                        TransactionGenParams::new_dynamic_with_modules(),
                    ),
                    transaction_count,
                )
            };

            let transaction_gen = transaction_strategy
                .new_tree(&mut runner)
                .expect("creating transactions should succeed")
                .current();

            // Convert transactions to use modules
            let transactions: Vec<MockTransaction<KeyType<[u8; 32]>, MockEvent>> = transaction_gen
                .into_iter()
                .map(|txn_gen| txn_gen.materialize_modules(&universe))
                .collect();

            let txn_provider = DefaultTxnProvider::new(transactions);
            let state_view = MockStateView::empty();

            // Generate all potential module IDs that could be used in the tests
            let all_module_ids = generate_all_potential_module_ids(&universe);

            // Run tests with fail point enabled to test the version metadata
            for _ in 0..num_executions {
                let output = execute_block_parallel::<
                    MockTransaction<KeyType<[u8; 32]>, MockEvent>,
                    MockStateView<KeyType<[u8; 32]>>,
                    DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
                >(
                    executor_thread_pool.clone(),
                    maybe_block_gas_limit,
                    block_stm_v2,
                    &txn_provider,
                    &state_view,
                    Some(&all_module_ids),
                );

                BaselineOutput::generate(txn_provider.get_txns(), maybe_block_gas_limit)
                    .assert_parallel_output(&output);
            }
        }
    }
    scenario.teardown();
}

// Generate all potential module IDs that could be used in the tests
fn generate_all_potential_module_ids(universe: &[[u8; 32]]) -> Vec<ModuleId> {
    universe
        .iter()
        .map(|k| key_to_module_id(&KeyType(*k), universe.len()))
        .collect()
}

// Test cases with various parameters
#[test_case(50, 100, false, false, true, 2, 3; "basic modules only test v1")]
#[test_case(50, 100, false, true, true, 2, 3; "basic modules only test v2")]
#[test_case(50, 100, true, false, true, 2, 3; "modules only with gas limit v1")]
#[test_case(50, 100, true, true, true, 2, 3; "modules only with gas limit v2")]
#[test_case(50, 100, false, false, false, 2, 3; "mixed with modules test v1")]
#[test_case(50, 100, false, true, false, 2, 3; "mixed with modules test v2")]
#[test_case(50, 100, true, false, false, 2, 3; "mixed with modules with gas limit v1")]
#[test_case(50, 100, true, true, false, 2, 3; "mixed with modules with gas limit v2")]
#[test_case(10, 1000, false, false, true, 2, 2; "small universe modules only v1")]
#[test_case(10, 1000, false, true, true, 2, 2; "small universe modules only v2")]
#[test_case(10, 1000, false, false, false, 2, 2; "small universe mixed with modules v1")]
#[test_case(10, 1000, false, true, false, 2, 2; "small universe mixed with modules v2")]
fn module_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    block_stm_v2: bool,
    modules_only: bool,
    num_executions: usize,
    num_random_generations: usize,
) {
    execute_module_tests(
        universe_size,
        transaction_count,
        use_gas_limit,
        block_stm_v2,
        modules_only,
        num_executions,
        num_random_generations,
    );
}
