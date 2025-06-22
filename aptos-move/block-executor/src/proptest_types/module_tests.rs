// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    proptest_types::{
        baseline::BaselineOutput,
        mock_executor::{MockEvent, MockTask},
        resource_tests::{
            create_executor_thread_pool, execute_block_parallel, get_gas_limit_variants,
        },
        types::{
            key_to_mock_module_id, KeyType, MockTransaction, TransactionGen, TransactionGenParams,
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

enum ModuleTestType {
    // All transactions publish modules, and all accesses are module reads.
    AllTransactionsAndAccesses,
    // All transactions publish modules, but some accesses are not module reads.
    AllTransactionsMixedAccesses,
    // Some transactions publish modules and contain module reads. Other
    // transactions do not publish modules and do not contain module reads.
    MixedTransactionsMixedAccesses,
}

fn execute_module_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    modules_test_type: ModuleTestType,
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
            let transaction_strategy = match modules_test_type {
                ModuleTestType::AllTransactionsAndAccesses => vec(
                    any_with::<TransactionGen<[u8; 32]>>(
                        TransactionGenParams::new_dynamic_modules_only(),
                    ),
                    transaction_count,
                ),
                ModuleTestType::AllTransactionsMixedAccesses
                | ModuleTestType::MixedTransactionsMixedAccesses => vec(
                    any_with::<TransactionGen<[u8; 32]>>(
                        TransactionGenParams::new_dynamic_with_modules(),
                    ),
                    transaction_count,
                ),
            };

            let transaction_gen = transaction_strategy
                .new_tree(&mut runner)
                .expect("creating transactions should succeed")
                .current();

            // Convert transactions to use modules. For mixed transactions, we convert every
            // fifth transaction to use modules.
            let transactions: Vec<MockTransaction<KeyType<[u8; 32]>, MockEvent>> = transaction_gen
                .into_iter()
                .enumerate()
                .map(|(i, txn_gen)| {
                    if i % 5 == 0
                        || !matches!(
                            modules_test_type,
                            ModuleTestType::MixedTransactionsMixedAccesses
                        )
                    {
                        txn_gen.materialize_modules(&universe)
                    } else {
                        txn_gen.materialize(&universe)
                    }
                })
                .collect();

            let txn_provider = DefaultTxnProvider::new_without_info(transactions);
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
                    &txn_provider,
                    &state_view,
                    Some(&all_module_ids),
                    false,
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
        .map(|k| key_to_mock_module_id(&KeyType(*k), universe.len()))
        .collect()
}

// Test cases with various parameters
#[test_case(50, 100, false, ModuleTestType::AllTransactionsAndAccesses, 2, 3; "basic modules only test v1")]
#[test_case(50, 100, false, ModuleTestType::MixedTransactionsMixedAccesses, 2, 3; "basic mixed txn test with modules v1")]
#[test_case(50, 100, true, ModuleTestType::AllTransactionsAndAccesses, 2, 3; "modules only with gas limit")]
#[test_case(50, 100, false, ModuleTestType::AllTransactionsMixedAccesses, 2, 3; "mixed access with modules test")]
#[test_case(50, 100, true, ModuleTestType::AllTransactionsMixedAccesses, 2, 3; "mixed access with modules with gas limit")]
#[test_case(10, 1000, false, ModuleTestType::AllTransactionsAndAccesses, 2, 2; "small universe modules only")]
#[test_case(10, 1000, false, ModuleTestType::AllTransactionsMixedAccesses, 2, 2; "small universe mixed access with modules")]
fn module_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    use_gas_limit: bool,
    modules_test_type: ModuleTestType,
    num_executions: usize,
    num_random_generations: usize,
) {
    execute_module_tests(
        universe_size,
        transaction_count,
        use_gas_limit,
        modules_test_type,
        num_executions,
        num_random_generations,
    );
}
