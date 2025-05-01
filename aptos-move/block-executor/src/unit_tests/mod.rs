// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod code_cache_tests;

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    errors::SequentialBlockExecutionError,
    executor::BlockExecutor,
    proptest_types::{
        baseline::BaselineOutput,
        types::{
            DeltaDataView, KeyType, MockEvent, MockIncarnation, MockOutput, MockTask,
            MockTransaction, NonEmptyGroupDataView, ValueType,
        },
    },
    scheduler::{
        DependencyResult, ExecutionTaskType, Scheduler, SchedulerTask, TWaitForDependency,
    },
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_aggregator::{
    bounded_math::SignedU128,
    delta_change_set::{delta_add, delta_sub, DeltaOp},
    delta_math::DeltaHistory,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::config::BlockExecutorConfig,
    contract_event::TransactionEvent,
    state_store::{state_key::PathInfo, state_value::StateValueMetadata},
    write_set::WriteOpKind,
};
use claims::{assert_matches, assert_ok};
use fail::FailScenario;
use rand::{prelude::*, random};
use std::{
    cmp::min,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
};

#[test]
fn test_resource_group_deletion() {
    let mut group_creation: MockIncarnation<KeyType<u32>, MockEvent> =
        MockIncarnation::new(vec![KeyType::<u32>::new(1)], vec![], vec![], vec![], 10);
    group_creation.group_writes.push((
        KeyType::<u32>::new(100),
        StateValueMetadata::none(),
        HashMap::from([(101, ValueType::from_value(vec![5], true))]),
    ));
    let mut group_deletion: MockIncarnation<KeyType<u32>, MockEvent> =
        MockIncarnation::new(vec![KeyType::<u32>::new(1)], vec![], vec![], vec![], 10);
    group_deletion.group_writes.push((
        KeyType::<u32>::new(100),
        StateValueMetadata::none(),
        HashMap::from([(
            101,
            ValueType::new(None, StateValueMetadata::none(), WriteOpKind::Deletion),
        )]),
    ));
    let t_0 = MockTransaction::from_behavior(group_creation);
    let t_1 = MockTransaction::from_behavior(group_deletion);

    let transactions = Vec::from([t_0, t_1]);

    let data_view = NonEmptyGroupDataView::<KeyType<u32>>::new();
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );
    let block_executor = BlockExecutor::<
        MockTransaction<KeyType<u32>, MockEvent>,
        MockTask<KeyType<u32>, MockEvent>,
        NonEmptyGroupDataView<KeyType<u32>>,
        NoOpTransactionCommitHook<MockOutput<KeyType<u32>, MockEvent>, usize>,
        DefaultTxnProvider<MockTransaction<KeyType<u32>, MockEvent>>,
    >::new(
        BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
        executor_thread_pool,
        None,
    );

    let mut guard = AptosModuleCacheManagerGuard::none();
    let txn_provider = DefaultTxnProvider::new(transactions);
    assert_ok!(block_executor.execute_transactions_sequential(
        &txn_provider,
        &data_view,
        &mut guard,
        false
    ));

    let mut guard = AptosModuleCacheManagerGuard::none();
    assert_ok!(block_executor.execute_transactions_parallel(&txn_provider, &data_view, &mut guard));
}

#[test]
fn resource_group_bcs_fallback() {
    let no_group_incarnation_1: MockIncarnation<KeyType<u32>, MockEvent> = MockIncarnation::new(
        vec![KeyType::<u32>::new(1)],
        vec![(KeyType::<u32>::new(2), ValueType::from_value(vec![5], true))],
        vec![],
        vec![],
        10,
    );
    let no_group_incarnation_2: MockIncarnation<KeyType<u32>, MockEvent> = MockIncarnation::new(
        vec![KeyType::<u32>::new(3), KeyType::<u32>::new(4)],
        vec![(KeyType::<u32>::new(1), ValueType::from_value(vec![5], true))],
        vec![],
        vec![],
        10,
    );
    let t_1 = MockTransaction::from_behavior(no_group_incarnation_1);
    let t_3 = MockTransaction::from_behavior(no_group_incarnation_2);

    let mut group_incarnation: MockIncarnation<KeyType<u32>, MockEvent> =
        MockIncarnation::new(vec![KeyType::<u32>::new(1)], vec![], vec![], vec![], 10);
    group_incarnation.group_writes.push((
        KeyType::<u32>::new(100),
        StateValueMetadata::none(),
        HashMap::from([(101, ValueType::from_value(vec![5], true))]),
    ));
    let t_2 = MockTransaction::from_behavior(group_incarnation);
    let transactions = Vec::from([t_1, t_2, t_3]);

    let data_view = NonEmptyGroupDataView::<KeyType<u32>>::new();
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );
    let block_executor = BlockExecutor::<
        MockTransaction<KeyType<u32>, MockEvent>,
        MockTask<KeyType<u32>, MockEvent>,
        NonEmptyGroupDataView<KeyType<u32>>,
        NoOpTransactionCommitHook<MockOutput<KeyType<u32>, MockEvent>, usize>,
        DefaultTxnProvider<MockTransaction<KeyType<u32>, MockEvent>>,
    >::new(
        BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
        executor_thread_pool,
        None,
    );

    let txn_provider = DefaultTxnProvider::new(transactions);
    // Execute the block normally.
    let mut guard = AptosModuleCacheManagerGuard::none();
    let output =
        block_executor.execute_transactions_parallel(&txn_provider, &data_view, &mut guard);
    match output {
        Ok(block_output) => {
            let txn_outputs = block_output.into_transaction_outputs_forced();
            assert_eq!(txn_outputs.len(), 3);
            assert!(!txn_outputs[0].writes.is_empty());
            assert!(!txn_outputs[2].writes.is_empty());
            assert!(!txn_outputs[1].group_writes.is_empty());
        },
        Err(e) => unreachable!("Must succeed, but {:?}: failpoint not yet set up", e),
    };

    // Set up and sanity check failpoint.
    let scenario = FailScenario::setup();
    assert!(fail::has_failpoints());
    fail::cfg("fail-point-resource-group-serialization", "return()").unwrap();
    assert!(!fail::list().is_empty());

    let mut guard = AptosModuleCacheManagerGuard::none();
    let par_output =
        block_executor.execute_transactions_parallel(&txn_provider, &data_view, &mut guard);
    assert_matches!(par_output, Err(()));

    let mut guard = AptosModuleCacheManagerGuard::none();
    let seq_output = block_executor.execute_transactions_sequential(
        &txn_provider,
        &data_view,
        &mut guard,
        false,
    );
    assert_matches!(
        seq_output,
        Err(SequentialBlockExecutionError::ResourceGroupSerializationError)
    );

    // Now execute with fallback handling for resource group serialization error:
    let mut guard = AptosModuleCacheManagerGuard::none();
    let fallback_output = block_executor
        .execute_transactions_sequential(&txn_provider, &data_view, &mut guard, true)
        .map_err(|e| match e {
            SequentialBlockExecutionError::ResourceGroupSerializationError => {
                panic!("Unexpected error")
            },
            SequentialBlockExecutionError::ErrorToReturn(err) => err,
        });

    let mut guard = AptosModuleCacheManagerGuard::none();
    let fallback_output_block = block_executor.execute_block(&txn_provider, &data_view, &mut guard);
    for output in [fallback_output, fallback_output_block] {
        match output {
            Ok(block_output) => {
                let txn_outputs = block_output.into_transaction_outputs_forced();
                assert_eq!(txn_outputs.len(), 3);
                assert!(!txn_outputs[0].writes.is_empty());
                assert!(!txn_outputs[2].writes.is_empty());

                // But now transaction 1 must be skipped.
                assert!(txn_outputs[1].skipped);
            },
            Err(_) => unreachable!("Must succeed: fallback"),
        };
    }

    scenario.teardown();
}

#[test]
fn interrupt_requested() {
    let transactions = Vec::from([MockTransaction::Abort, MockTransaction::InterruptRequested]);
    let txn_provider = DefaultTxnProvider::new(transactions);
    let mut guard = AptosModuleCacheManagerGuard::none();

    let data_view = DeltaDataView::<KeyType<u32>> {
        phantom: PhantomData,
    };
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );
    let block_executor = BlockExecutor::<
        MockTransaction<KeyType<u32>, MockEvent>,
        MockTask<KeyType<u32>, MockEvent>,
        DeltaDataView<KeyType<u32>>,
        NoOpTransactionCommitHook<MockOutput<KeyType<u32>, MockEvent>, usize>,
        DefaultTxnProvider<MockTransaction<KeyType<u32>, MockEvent>>,
    >::new(
        BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
        executor_thread_pool,
        None,
    );

    // MockTransaction::InterruptRequested will only return if interrupt is requested (here, due
    // to abort from the first transaction). O.w. the test will hang.
    let _ = block_executor.execute_transactions_parallel(&txn_provider, &data_view, &mut guard);
}

#[test]
fn block_output_err_precedence() {
    let incarnation: MockIncarnation<KeyType<u32>, MockEvent> = MockIncarnation::new(
        vec![KeyType::<u32>::new(1)],
        vec![(KeyType::<u32>::new(2), ValueType::from_value(vec![5], true))],
        vec![],
        vec![],
        10,
    );
    let txn = MockTransaction::from_behavior(incarnation);
    let transactions = Vec::from([txn.clone(), txn]);
    let txn_provider = DefaultTxnProvider::new(transactions);

    let data_view = DeltaDataView::<KeyType<u32>> {
        phantom: PhantomData,
    };
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );
    let block_executor = BlockExecutor::<
        MockTransaction<KeyType<u32>, MockEvent>,
        MockTask<KeyType<u32>, MockEvent>,
        DeltaDataView<KeyType<u32>>,
        NoOpTransactionCommitHook<MockOutput<KeyType<u32>, MockEvent>, usize>,
        DefaultTxnProvider<MockTransaction<KeyType<u32>, MockEvent>>,
    >::new(
        BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
        executor_thread_pool,
        None,
    );

    let scenario = FailScenario::setup();
    assert!(fail::has_failpoints());
    fail::cfg("commit-all-halt-err", "return()").unwrap();
    assert!(!fail::list().is_empty());
    // Pause the thread that processes the aborting txn1, so txn2 can halt the scheduler first.
    // Confirm that the fatal VM error is still detected and sequential fallback triggered.
    let mut guard = AptosModuleCacheManagerGuard::none();
    let output =
        block_executor.execute_transactions_parallel(&txn_provider, &data_view, &mut guard);
    assert_matches!(output, Err(()));
    scenario.teardown();
}

#[test]
fn skip_rest_gas_limit() {
    // The contents of the second txn does not matter, as the first should hit the gas limit and
    // also skip. But it ensures block is not finished at the first txn (different processing).
    let transactions = Vec::from([MockTransaction::SkipRest(10), MockTransaction::SkipRest(10)]);
    let txn_provider = DefaultTxnProvider::new(transactions);

    let data_view = DeltaDataView::<KeyType<u32>> {
        phantom: PhantomData,
    };
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );
    let block_executor = BlockExecutor::<
        MockTransaction<KeyType<u32>, MockEvent>,
        MockTask<KeyType<u32>, MockEvent>,
        DeltaDataView<KeyType<u32>>,
        NoOpTransactionCommitHook<MockOutput<KeyType<u32>, MockEvent>, usize>,
        DefaultTxnProvider<MockTransaction<KeyType<u32>, MockEvent>>,
    >::new(
        BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), Some(5)),
        executor_thread_pool,
        None,
    );

    // Should hit block limit on the skip transaction.
    let mut guard = AptosModuleCacheManagerGuard::none();
    let _ = block_executor.execute_transactions_parallel(&txn_provider, &data_view, &mut guard);
}

// TODO: add unit test for block gas limit!
fn run_and_assert<K, E>(transactions: Vec<MockTransaction<K, E>>)
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + PathInfo + Debug + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    let data_view = DeltaDataView::<K> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    let mut guard = AptosModuleCacheManagerGuard::none();
    let txn_provider = DefaultTxnProvider::new(transactions);
    let output = BlockExecutor::<
        MockTransaction<K, E>,
        MockTask<K, E>,
        DeltaDataView<K>,
        NoOpTransactionCommitHook<MockOutput<K, E>, usize>,
        _,
    >::new(
        BlockExecutorConfig::new_no_block_limit(num_cpus::get()),
        executor_thread_pool,
        None,
    )
    .execute_transactions_parallel(&txn_provider, &data_view, &mut guard);

    let baseline = BaselineOutput::generate(txn_provider.get_txns(), None);
    baseline.assert_parallel_output(&output);
}

fn random_value(delete_value: bool) -> ValueType {
    ValueType::from_value(
        (0..32).map(|_| (random::<u8>())).collect::<Vec<u8>>(),
        !delete_value,
    )
}

#[test]
fn empty_block() {
    // This test checks that we do not trigger asserts due to an empty block, e.g. in the
    // scheduler. Instead, parallel execution should gracefully early return empty output.
    run_and_assert::<KeyType<[u8; 32]>, MockEvent>(vec![]);
}

#[test]
fn delta_counters() {
    let key = KeyType::new(random::<[u8; 32]>());
    let mut transactions = vec![MockTransaction::from_behavior(MockIncarnation::<
        KeyType<[u8; 32]>,
        MockEvent,
    >::new(
        vec![],
        vec![(key, random_value(false))], // writes
        vec![],
        vec![],
        1, // gas
    ))];

    for _ in 0..50 {
        transactions.push(MockTransaction::from_behavior(MockIncarnation::<
            KeyType<[u8; 32]>,
            MockEvent,
        >::new(
            vec![key], // reads
            vec![],
            vec![(key, delta_add(5, u128::MAX))], // deltas
            vec![],
            1, // gas
        )));
    }

    transactions.push(MockTransaction::from_behavior(MockIncarnation::<
        KeyType<[u8; 32]>,
        MockEvent,
    >::new(
        vec![],
        vec![(key, random_value(false))], // writes
        vec![],
        vec![],
        1, // gas
    )));

    for _ in 0..50 {
        transactions.push(MockTransaction::from_behavior(MockIncarnation::<
            KeyType<[u8; 32]>,
            MockEvent,
        >::new(
            vec![key], // reads
            vec![],
            vec![(key, delta_sub(2, u128::MAX))], // deltas
            vec![],
            1, // gas
        )));
    }

    run_and_assert(transactions)
}

#[test]
fn delta_chains() {
    let mut transactions = vec![];
    // Generate a series of transactions add and subtract from an aggregator.

    let keys: Vec<KeyType<[u8; 32]>> = (0..10)
        .map(|_| KeyType::new(random::<[u8; 32]>()))
        .collect();

    for i in 0..500 {
        transactions.push(
            MockTransaction::<KeyType<[u8; 32]>, MockEvent>::from_behavior(MockIncarnation::new(
                keys.clone(), // reads
                vec![],
                keys.iter()
                    .enumerate()
                    .filter_map(|(j, k)| match (i + j) % 2 == 0 {
                        true => Some((
                            *k,
                            // Deterministic pattern for adds/subtracts.
                            DeltaOp::new(
                                if (i % 2 == 0) == (j < 5) {
                                    SignedU128::Positive(10)
                                } else {
                                    SignedU128::Negative(1)
                                },
                                // below params irrelevant for this test.
                                u128::MAX,
                                DeltaHistory::new(),
                            ),
                        )),
                        false => None,
                    })
                    .collect(), // deltas
                vec![],
                1, // gas
            )),
        );
    }

    run_and_assert(transactions)
}

const TOTAL_KEY_NUM: u64 = 50;
const WRITES_PER_KEY: u64 = 100;

#[test]
fn cycle_transactions() {
    let mut transactions = vec![];
    // For every key in `TOTAL_KEY_NUM`, generate a series of transactions that will assign a
    // value to this key.
    for _ in 0..TOTAL_KEY_NUM {
        let key = random::<[u8; 32]>();
        for _ in 0..WRITES_PER_KEY {
            transactions.push(MockTransaction::from_behavior(MockIncarnation::<
                KeyType<[u8; 32]>,
                MockEvent,
            >::new(
                vec![KeyType::new(key)],                        // reads
                vec![(KeyType::new(key), random_value(false))], // writes
                vec![],
                vec![],
                1, // gas
            )));
        }
    }
    run_and_assert(transactions)
}

const NUM_BLOCKS: u64 = 10;
const TXN_PER_BLOCK: u64 = 100;

#[test]
fn one_reads_all_barrier() {
    let mut transactions = vec![];
    let keys: Vec<KeyType<_>> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType::new(random::<[u8; 32]>()))
        .collect();
    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(MockTransaction::from_behavior(MockIncarnation::<
                KeyType<[u8; 32]>,
                MockEvent,
            >::new(
                vec![*key],                        // reads
                vec![(*key, random_value(false))], // writes
                vec![],
                vec![],
                1, // gas
            )));
        }
        // One transaction reading the write results of every prior transactions in the block.
        transactions.push(MockTransaction::from_behavior(MockIncarnation::<
            KeyType<[u8; 32]>,
            MockEvent,
        >::new(
            keys.clone(), //reads
            vec![],
            vec![],
            vec![],
            1, //gas
        )));
    }
    run_and_assert(transactions)
}

#[test]
fn one_writes_all_barrier() {
    let mut transactions = vec![];
    let keys: Vec<KeyType<_>> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType::new(random::<[u8; 32]>()))
        .collect();
    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(MockTransaction::from_behavior(MockIncarnation::new(
                vec![*key],                        //reads
                vec![(*key, random_value(false))], //writes
                vec![],
                vec![],
                1, //gas
            )));
        }
        // One transaction writing to the write results of every prior transactions in the block.
        transactions.push(MockTransaction::from_behavior(MockIncarnation::<
            KeyType<[u8; 32]>,
            MockEvent,
        >::new(
            keys.clone(), // reads
            keys.iter()
                .map(|key| (*key, random_value(false)))
                .collect::<Vec<_>>(), //writes
            vec![],
            vec![],
            1, // gas
        )));
    }
    run_and_assert(transactions)
}

#[test]
fn early_aborts() {
    let mut transactions = vec![];
    let keys: Vec<_> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType::new(random::<[u8; 32]>()))
        .collect();

    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(MockTransaction::from_behavior(MockIncarnation::<
                KeyType<[u8; 32]>,
                MockEvent,
            >::new(
                vec![*key],                        // reads
                vec![(*key, random_value(false))], // writes
                vec![],
                vec![],
                1, // gas
            )));
        }
        // One transaction that triggers an abort
        transactions.push(MockTransaction::Abort)
    }
    run_and_assert(transactions)
}

#[test]
fn early_skips() {
    let mut transactions = vec![];
    let keys: Vec<_> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType::new(random::<[u8; 32]>()))
        .collect();

    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(MockTransaction::from_behavior(MockIncarnation::<
                KeyType<[u8; 32]>,
                MockEvent,
            >::new(
                vec![*key],                        // reads
                vec![(*key, random_value(false))], //writes
                vec![],
                vec![],
                1, // gas
            )));
        }
        // One transaction that triggers an abort
        transactions.push(MockTransaction::SkipRest(0))
    }
    run_and_assert(transactions)
}

#[test]
fn scheduler_tasks() {
    let s = Scheduler::new(5);

    for i in 0..5 {
        // No validation tasks.
        assert_matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask(j, 0, ExecutionTaskType::Execution) if i == j
        );
    }

    for i in 0..5 {
        // Validation index is at 0, so transactions will be validated and no
        // need to return a validation task to the caller.
        assert_matches!(s.finish_execution(i, 0, false), Ok(SchedulerTask::Retry));
    }

    for i in 0..5 {
        assert_matches!(
            s.next_task(),
            SchedulerTask::ValidationTask(j, 0, 0) if i == j
        );
    }

    // successful aborts.
    assert!(s.try_abort(3, 0));
    s.finish_validation(4, 0);
    assert!(s.try_abort(4, 0)); // can abort even after successful validation
    assert!(s.try_abort(1, 0));

    // unsuccessful aborts
    assert!(!s.try_abort(1, 0));
    assert!(!s.try_abort(3, 0));

    assert_matches!(
        s.finish_abort(4, 0),
        Ok(SchedulerTask::ExecutionTask(
            4,
            1,
            ExecutionTaskType::Execution
        ))
    );
    assert_matches!(
        s.finish_abort(1, 0),
        Ok(SchedulerTask::ExecutionTask(
            1,
            1,
            ExecutionTaskType::Execution
        ))
    );
    // Validation index = 2, wave = 1.
    assert_matches!(
        s.finish_abort(3, 0),
        Ok(SchedulerTask::ExecutionTask(
            3,
            1,
            ExecutionTaskType::Execution
        ))
    );

    assert_matches!(s.finish_execution(4, 1, true), Ok(SchedulerTask::Retry));
    assert_matches!(
        s.finish_execution(1, 1, false),
        Ok(SchedulerTask::ValidationTask(1, 1, 1))
    );

    // Another validation task for (2, 0).
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(2, 0, 1));
    // Now skip over txn 3 (status is Executing), and validate 4.
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(4, 1, 1));

    assert_matches!(
        s.finish_execution(3, 1, false),
        Ok(SchedulerTask::ValidationTask(3, 1, 1))
    );

    s.finish_validation(0, 0);
    s.finish_validation(1, 2);
    for i in 2..5 {
        s.finish_validation(i, 2)
    }

    // Make sure everything can be committed.
    for i in 0..5 {
        assert_matches!(s.try_commit(), Some((v, _)) if v == i);
    }

    assert_matches!(s.next_task(), SchedulerTask::Done);
}

#[test]
fn scheduler_first_wave() {
    let s = Scheduler::new(6);

    for i in 0..5 {
        // Nothing to validate.
        assert_matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask(j, 0, ExecutionTaskType::Execution) if j == i
        );
    }

    // validation index will not increase for the first execution wave
    // until the status becomes executed.
    assert_matches!(s.finish_execution(0, 0, false), Ok(SchedulerTask::Retry));

    // Now we can validate version (0, 0).
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(0, 0, 0));
    assert_matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask(5, 0, ExecutionTaskType::Execution)
    );
    // Since (1, 0) is not EXECUTED, no validation tasks, and execution index
    // is already at the limit, so no tasks immediately available.
    assert_matches!(s.next_task(), SchedulerTask::Retry);

    assert_matches!(s.finish_execution(2, 0, false), Ok(SchedulerTask::Retry));
    // There should be no tasks, but finishing (1,0) should enable validating
    // (1, 0) then (2,0).
    assert_matches!(s.next_task(), SchedulerTask::Retry);

    assert_matches!(s.finish_execution(1, 0, false), Ok(SchedulerTask::Retry));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(1, 0, 0));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(2, 0, 0));
    assert_matches!(s.next_task(), SchedulerTask::Retry);
}

#[test]
fn scheduler_dependency() {
    let s = Scheduler::new(10);

    for i in 0..5 {
        // Nothing to validate.
        assert_matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask(j, 0, ExecutionTaskType::Execution) if j == i
        );
    }

    // validation index will not increase for the first execution wave
    // until the status becomes executed.
    assert_matches!(s.finish_execution(0, 0, false), Ok(SchedulerTask::Retry));
    // Now we can validate version (0, 0).
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(0, 0, 0));
    // Current status of 0 is executed - hence, no dependency added.
    assert_matches!(s.wait_for_dependency(3, 0), Ok(DependencyResult::Resolved));
    // Dependency added for transaction 4 on transaction 2.
    assert_matches!(
        s.wait_for_dependency(4, 2),
        Ok(DependencyResult::Dependency(_))
    );

    assert_matches!(s.finish_execution(2, 0, false), Ok(SchedulerTask::Retry));

    // resumed task doesn't bump incarnation
    assert_matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask(4, 0, ExecutionTaskType::Wakeup(_))
    );
}

// Will return a scheduler in a state where all transactions are scheduled for
// for execution, validation index = num_txns, and wave = 0.
fn incarnation_one_scheduler(num_txns: TxnIndex) -> Scheduler {
    let s = Scheduler::new(num_txns);

    for i in 0..num_txns {
        // Get the first executions out of the way.
        assert_matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask(j, 0, ExecutionTaskType::Execution) if j == i
        );
        assert_matches!(s.finish_execution(i, 0, false), Ok(SchedulerTask::Retry));
        assert_matches!(
            s.next_task(),
            SchedulerTask::ValidationTask(j, 0, 0) if i == j
        );
        assert!(s.try_abort(i, 0));
        assert_matches!(
            s.finish_abort(i, 0),
            Ok(SchedulerTask::ExecutionTask(j, 1, ExecutionTaskType::Execution)) if i == j
        );
    }
    s
}

#[test]
fn scheduler_incarnation() {
    let s = incarnation_one_scheduler(5);

    // execution/validation index = 5, wave = 0.
    assert_matches!(
        s.wait_for_dependency(1, 0),
        Ok(DependencyResult::Dependency(_))
    );
    assert_matches!(
        s.wait_for_dependency(3, 0),
        Ok(DependencyResult::Dependency(_))
    );

    // Because validation index is higher, return validation task to caller (even with
    // revalidate_suffix = true) - because now we always decrease validation idx to txn_idx + 1
    // here validation wave increases to 1, and index is reduced to 3.
    assert_matches!(
        s.finish_execution(2, 1, true),
        Ok(SchedulerTask::ValidationTask(2, 1, 1))
    );
    // Here since validation index is lower, wave doesn't increase and no task returned.
    assert_matches!(s.finish_execution(4, 1, true), Ok(SchedulerTask::Retry));

    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(4, 1, 1));

    assert!(s.try_abort(2, 1));
    assert!(s.try_abort(4, 1));
    assert!(!s.try_abort(2, 1));

    assert_matches!(
        s.finish_abort(2, 1),
        Ok(SchedulerTask::ExecutionTask(
            2,
            2,
            ExecutionTaskType::Execution
        ))
    );
    // wave = 2, validation index = 2.
    assert_matches!(
        s.finish_execution(0, 1, false),
        Ok(SchedulerTask::ValidationTask(0, 1, 2))
    );
    // execution index =  1

    assert_matches!(s.finish_abort(4, 1), Ok(SchedulerTask::Retry));

    assert_matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask(1, 1, ExecutionTaskType::Wakeup(_))
    );
    assert_matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask(3, 1, ExecutionTaskType::Wakeup(_))
    );
    assert_matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask(4, 2, ExecutionTaskType::Execution)
    );
    // execution index = 5

    assert_matches!(
        s.finish_execution(1, 1, false),
        Ok(SchedulerTask::ValidationTask(1, 1, 2))
    );
    assert_matches!(
        s.finish_execution(2, 2, false),
        Ok(SchedulerTask::ValidationTask(2, 2, 2))
    );
    assert_matches!(
        s.finish_execution(3, 1, false),
        Ok(SchedulerTask::ValidationTask(3, 1, 2))
    );

    // validation index is 4, so finish execution doesn't return validation task, next task does.
    assert_matches!(s.finish_execution(4, 2, false), Ok(SchedulerTask::Retry));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(4, 2, 2));
}

#[test]
fn scheduler_basic() {
    let s = Scheduler::new(3);

    for i in 0..3 {
        // Nothing to validate.
        assert_matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask(j, 0, ExecutionTaskType::Execution) if j == i
        );
    }

    // Finish executions & dispatch validation tasks.
    assert_matches!(s.finish_execution(0, 0, true), Ok(SchedulerTask::Retry));
    assert_matches!(s.finish_execution(1, 0, true), Ok(SchedulerTask::Retry));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(0, 0, 0));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(1, 0, 0));
    assert_matches!(s.finish_execution(2, 0, true), Ok(SchedulerTask::Retry));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(2, 0, 0));

    for i in 0..3 {
        s.finish_validation(i, 1)
    }

    // make sure everything can be committed.
    for i in 0..3 {
        assert_matches!(s.try_commit(), Some((v, _)) if v == i);
    }

    assert_matches!(s.next_task(), SchedulerTask::Done);
}

#[test]
fn scheduler_drain_idx() {
    let s = Scheduler::new(3);

    for i in 0..3 {
        // Nothing to validate.
        assert_matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask(j, 0, ExecutionTaskType::Execution) if j == i
        );
    }

    // Finish executions & dispatch validation tasks.
    assert_matches!(s.finish_execution(0, 0, true), Ok(SchedulerTask::Retry));
    assert_matches!(s.finish_execution(1, 0, true), Ok(SchedulerTask::Retry));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(0, 0, 0));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(1, 0, 0));
    assert_matches!(s.finish_execution(2, 0, true), Ok(SchedulerTask::Retry));
    assert_matches!(s.next_task(), SchedulerTask::ValidationTask(2, 0, 0));

    for i in 0..3 {
        s.finish_validation(i, 1)
    }

    // make sure everything can be committed.
    for i in 0..3 {
        assert_matches!(s.try_commit(), Some((v, _)) if v == i);
    }

    assert_matches!(s.next_task(), SchedulerTask::Done);
}

#[test]
fn finish_execution_wave() {
    // Wave won't be increased, because validation index is already 2, and finish_execution
    // tries to reduce it to 2.
    let s = incarnation_one_scheduler(2);
    assert_matches!(
        s.finish_execution(1, 1, true),
        Ok(SchedulerTask::ValidationTask(1, 1, 0))
    );

    // Here wave will increase, because validation index is reduced from 3 to 2.
    let s = incarnation_one_scheduler(3);
    assert_matches!(
        s.finish_execution(1, 1, true),
        Ok(SchedulerTask::ValidationTask(1, 1, 1))
    );

    // Here wave won't be increased, because we pass revalidate_suffix = false.
    let s = incarnation_one_scheduler(3);
    assert_matches!(
        s.finish_execution(1, 1, false),
        Ok(SchedulerTask::ValidationTask(1, 1, 0))
    );
}

#[test]
fn rolling_commit_wave() {
    let s = incarnation_one_scheduler(3);

    // Finish execution for txn 0 without validate_suffix and because
    // validation index is higher will return validation task to the caller.
    assert_matches!(
        s.finish_execution(0, 1, false),
        Ok(SchedulerTask::ValidationTask(0, 1, 0))
    );
    // finish validating txn 0 with proper wave
    s.finish_validation(0, 1);
    // txn 0 can be committed
    assert_matches!(s.try_commit(), Some((0, _)));
    assert_eq!(s.commit_state(), (1, 0));

    // This increases the wave, but only sets max_triggered_wave for transaction 2.
    // sets validation_index to 2.
    assert_matches!(
        s.finish_execution(1, 1, true),
        Ok(SchedulerTask::ValidationTask(1, 1, 1))
    );

    // finish validating txn 1 with lower wave
    s.finish_validation(1, 0);
    // txn 1 cannot be committed
    assert!(s.try_commit().is_none());
    assert_eq!(s.commit_state(), (1, 0));

    // finish validating txn 1 with proper wave
    s.finish_validation(1, 1);
    // txn 1 can be committed
    assert_matches!(s.try_commit(), Some((1, _)));
    assert_eq!(s.commit_state(), (2, 0));

    // No validation task because index is already 2.
    assert_matches!(s.finish_execution(2, 1, false), Ok(SchedulerTask::Retry,));
    // finish validating with a lower wave.
    s.finish_validation(2, 0);
    assert!(s.try_commit().is_none());
    assert_eq!(s.commit_state(), (2, 1));
    // Finish validation with appropriate wave.
    s.finish_validation(2, 1);
    assert_matches!(s.try_commit(), Some((2, _)));
    assert_eq!(s.commit_state(), (3, 1));

    // All txns have been committed.
    assert_matches!(s.next_task(), SchedulerTask::Done);
}

#[test]
fn no_conflict_task_count() {
    // When there are no conflicts and transactions do not abort, the number of
    // execution and validation tasks should be the same, no matter in which order
    // the concurrent tasks are performed. We can simulate different order by
    // assigning a virtual duration to each task, and using it as a priority for
    // calling finish_ on the corresponding task to the scheduler. We should also
    // keep calling next_task to keep the total number of tasks being worked on
    // somewhat constant.
    //
    // invariants:
    // 1. should return same number of validation and execution tasks, = num_txns;
    // 2. all incarnations should be 0.
    // 3. current wave should always be 0.

    let num_txns: TxnIndex = 1000;
    for num_concurrent_tasks in [1, 5, 10, 20] {
        let s = Scheduler::new(num_txns);

        let mut tasks = BTreeMap::new();

        let mut rng = rand::thread_rng();
        let mut num_exec_tasks = 0;
        let mut num_val_tasks = 0;

        loop {
            while tasks.len() < num_concurrent_tasks {
                match s.next_task() {
                    SchedulerTask::ExecutionTask(txn_idx, incarnation, _) => {
                        assert_eq!(incarnation, 0);
                        // true means an execution task.
                        tasks.insert(rng.gen::<u32>(), (true, txn_idx));
                    },
                    SchedulerTask::ValidationTask(txn_idx, incarnation, cur_wave) => {
                        assert_eq!(incarnation, 0);
                        assert_eq!(cur_wave, 0);
                        // false means a validation task.
                        tasks.insert(rng.gen::<u32>(), (false, txn_idx));
                    },
                    SchedulerTask::Retry => break,
                    // Unreachable because we never call try_commit.
                    SchedulerTask::Done => unreachable!(),
                }
            }

            if tasks.is_empty() {
                break;
            }

            // Do a few tasks.
            let num_tasks_to_perform = rng.gen_range(1, min(tasks.len(), 4) + 1);
            for _ in 0..num_tasks_to_perform {
                match tasks.pop_first().unwrap() {
                    (_, (true, txn_idx)) => {
                        let task_res = s.finish_execution(txn_idx, 0, true);
                        num_exec_tasks += 1;

                        // Process a task that may have been returned.
                        if let Ok(SchedulerTask::ValidationTask(idx, incarnation, wave)) = task_res
                        {
                            assert_eq!(idx, txn_idx);
                            assert_eq!(incarnation, 0);
                            assert_eq!(wave, 0);
                            tasks.insert(rng.gen::<u32>(), (false, txn_idx));
                        } else {
                            assert_matches!(task_res, Ok(SchedulerTask::Retry));
                        }
                    },
                    (_, (false, txn_idx)) => {
                        s.finish_validation(txn_idx, 0);
                        num_val_tasks += 1;
                    },
                }
            }
        }

        assert_eq!(num_exec_tasks, num_txns);
        assert_eq!(num_val_tasks, num_txns);

        for i in 0..num_txns {
            assert_matches!(s.try_commit(), Some((v, _)) if v == i);
            assert_eq!(s.commit_state(), (i + 1, 0));
        }
        assert_matches!(s.next_task(), SchedulerTask::Done);
    }
}
