// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    combinatorial_tests::{
        baseline::BaselineOutput,
        mock_executor::{MockEvent, MockOutput, MockTask},
        types::{KeyType, MockTransaction, TransactionGen, TransactionGenParams, MAX_GAS_PER_TXN},
    },
    executor::BlockExecutor,
    task::ExecutorTask,
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::{default::DefaultTxnProvider, TxnProvider},
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::{state_value::StateValue, MockStateView, TStateView},
    transaction::{BlockExecutableTransaction as Transaction, BlockOutput},
    vm::modules::AptosModuleExtension,
};
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::Module;
use move_vm_types::code::ModuleCode;
use proptest::{
    collection::vec,
    prelude::*,
    sample::Index,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use rand::Rng;
use std::{fmt::Debug, sync::Arc};
use test_case::test_matrix;

pub(crate) fn get_gas_limit_variants(
    use_gas_limit: bool,
    transaction_count: usize,
) -> Vec<Option<u64>> {
    if use_gas_limit {
        vec![
            Some(rand::thread_rng().gen_range(0, (transaction_count as u64) * MAX_GAS_PER_TXN / 2)),
            Some(0),
        ]
    } else {
        vec![None]
    }
}

pub(crate) fn create_executor_thread_pool() -> Arc<rayon::ThreadPool> {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    )
}

/// Populates a module cache manager guard with empty modules for testing.
/// This function creates empty modules for each ModuleId in the provided list and adds them to the guard's module cache.
///
/// # Arguments
/// * `guard` - The AptosModuleCacheManagerGuard to populate with empty modules
/// * `module_ids` - A slice of ModuleIds to create empty modules for
///
/// # Returns
/// The number of modules successfully added to the cache
pub(crate) fn populate_guard_with_modules(
    guard: &mut AptosModuleCacheManagerGuard<'_>,
    module_ids: &[ModuleId],
) {
    for module_id in module_ids {
        // Create an empty module for testing with Module::new_for_test
        let module = Module::new_for_test(module_id.clone());

        // Serialize the module
        let mut serialized_bytes = Vec::new();
        module
            .serialize(&mut serialized_bytes)
            .expect("Failed to serialize compiled module");

        // Create a ModuleCode::verified instance with the module
        let module_code = Arc::new(ModuleCode::from_arced_verified(
            Arc::new(module),
            Arc::new(AptosModuleExtension::new(StateValue::new_legacy(
                serialized_bytes.into(),
            ))),
        ));

        // Add the module to the cache
        guard
            .module_cache_mut()
            .insert(module_id.clone(), module_code);
    }
}

pub(crate) fn execute_block_parallel<TxnType, ViewType, Provider>(
    executor_thread_pool: Arc<rayon::ThreadPool>,
    block_gas_limit: Option<u64>,
    txn_provider: &Provider,
    data_view: &ViewType,
    all_module_ids: Option<&[ModuleId]>,
    block_stm_v2: bool,
) -> Result<BlockOutput<ViewType::Key, MockOutput<KeyType<[u8; 32]>, MockEvent>>, ()>
where
    TxnType: Transaction<Key = KeyType<[u8; 32]>> + Debug + Clone + Send + Sync + 'static,
    ViewType: TStateView<Key = TxnType::Key> + Sync + 'static,
    Provider: TxnProvider<TxnType> + Sync + 'static,
    MockTask<KeyType<[u8; 32]>, MockEvent>: ExecutorTask<Txn = TxnType>,
{
    let mut guard = AptosModuleCacheManagerGuard::none();

    // If all_module_ids is provided, populate the guard with empty modules
    if let Some(module_ids) = all_module_ids {
        populate_guard_with_modules(&mut guard, module_ids);
    }

    let config = BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), block_gas_limit);
    let block_executor = BlockExecutor::<
        TxnType,
        MockTask<KeyType<[u8; 32]>, MockEvent>,
        ViewType,
        NoOpTransactionCommitHook<MockOutput<KeyType<[u8; 32]>, MockEvent>, usize>,
        Provider,
    >::new(config, executor_thread_pool, None);

    if block_stm_v2 {
        block_executor.execute_transactions_parallel_v2(txn_provider, data_view, &mut guard)
    } else {
        block_executor.execute_transactions_parallel(
            txn_provider,
            data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        )
    }
}

pub(crate) fn generate_universe_and_transactions(
    runner: &mut TestRunner,
    universe_size: usize,
    transaction_count: usize,
    is_dynamic: bool,
) -> (Vec<[u8; 32]>, Vec<TransactionGen<[u8; 32]>>) {
    let universe = vec(any::<[u8; 32]>(), universe_size)
        .new_tree(runner)
        .expect("creating universe should succeed")
        .current();

    let transaction_strategy = if is_dynamic {
        vec(
            any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
            transaction_count,
        )
    } else {
        vec(any::<TransactionGen<[u8; 32]>>(), transaction_count)
    };

    let transaction_gen = transaction_strategy
        .new_tree(runner)
        .expect("creating transactions should succeed")
        .current();

    (universe, transaction_gen)
}

pub(crate) fn run_transactions_resources(
    universe_size: usize,
    transaction_count: usize,
    abort_count: usize,
    skip_rest_count: usize,
    use_gas_limit: bool,
    is_dynamic: bool,
    num_executions: usize,
    num_random_generations: usize,
) {
    let executor_thread_pool = create_executor_thread_pool();
    let mut runner = TestRunner::default();

    let gas_limits = get_gas_limit_variants(use_gas_limit, transaction_count);

    // Run the test cases directly
    for idx_generation in 0..num_random_generations {
        // Generate universe and transactions
        let (universe, transaction_gen) = generate_universe_and_transactions(
            &mut runner,
            universe_size,
            transaction_count,
            is_dynamic,
        );

        // Generate abort and skip_rest transaction indices
        let abort_strategy = vec(any::<Index>(), abort_count);
        let skip_rest_strategy = vec(any::<Index>(), skip_rest_count);

        let abort_transactions = abort_strategy
            .new_tree(&mut runner)
            .expect("creating abort transactions should succeed")
            .current();

        let skip_rest_transactions = skip_rest_strategy
            .new_tree(&mut runner)
            .expect("creating skip_rest transactions should succeed")
            .current();

        // Create transactions
        let mut transactions: Vec<MockTransaction<KeyType<[u8; 32]>, MockEvent>> = transaction_gen
            .into_iter()
            .map(|txn_gen| txn_gen.materialize(&universe))
            .collect();

        // Apply modifications to transactions
        let length = transactions.len();
        for i in abort_transactions {
            *transactions.get_mut(i.index(length)).unwrap() = MockTransaction::Abort;
        }
        for i in skip_rest_transactions {
            *transactions.get_mut(i.index(length)).unwrap() = MockTransaction::SkipRest(0);
        }

        let txn_provider = DefaultTxnProvider::new_without_info(transactions);
        let state_view = MockStateView::empty();
        for idx_execution in 0..num_executions {
            for maybe_block_gas_limit in &gas_limits {
                if maybe_block_gas_limit.is_some_and(|v| v == 0)
                    && (idx_execution > 0 || idx_generation > 0)
                {
                    // For 0 gas limit tests, run fewer configurations.
                    continue;
                }
                for block_stm_v2 in [false, true] {
                    let output = execute_block_parallel::<
                        MockTransaction<KeyType<[u8; 32]>, MockEvent>,
                        MockStateView<KeyType<[u8; 32]>>,
                        DefaultTxnProvider<MockTransaction<KeyType<[u8; 32]>, MockEvent>>,
                    >(
                        executor_thread_pool.clone(),
                        *maybe_block_gas_limit,
                        &txn_provider,
                        &state_view,
                        None,
                        block_stm_v2,
                    );

                    BaselineOutput::generate(txn_provider.get_txns(), *maybe_block_gas_limit)
                        .assert_parallel_output(&output);
                }
            }
        }
    }
}

#[test_matrix(
    100, 3000, 0, 0, [false, true], [false, true], 6, 5; "varying_incarnation_behavior_gas_limit"
)]
#[test_matrix(
    50, 500, [0, 3, 200], [0, 3, 50], [false, true], [false, true], 5, 3; "with_mixed_abort_skip_rest"
)]
#[test_matrix(
    [10, 20], 1000, 0, 0, [false, true], [false, true], 10, 3; "contended"
)]
fn resource_transaction_tests(
    universe_size: usize,
    transaction_count: usize,
    abort_count: usize,
    skip_rest_count: usize,
    use_gas_limit: bool,
    is_dynamic: bool,
    num_random_generations: usize,
    num_executions: usize,
) {
    run_transactions_resources(
        universe_size,
        transaction_count,
        abort_count,
        skip_rest_count,
        use_gas_limit,
        is_dynamic,
        num_executions,
        num_random_generations,
    );
}
