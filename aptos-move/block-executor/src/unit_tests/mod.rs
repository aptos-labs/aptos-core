// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::BlockExecutor,
    proptest_types::types::{DeltaDataView, ExpectedOutput, KeyType, Task, Transaction, ValueType},
    scheduler::{DependencyResult, Scheduler, SchedulerTask},
    IndexMapping,
};
use aptos_aggregator::delta_change_set::{delta_add, delta_sub, DeltaOp, DeltaUpdate};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::partitioner::ExecutableTransactions,
    executable::{ExecutableTestType, ModulePath},
    write_set::TransactionWrite,
};
use claims::{assert_matches, assert_some_eq};
use rand::{prelude::*, random};
use std::{
    cmp::min,
    collections::BTreeMap,
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    sync::{atomic::AtomicUsize, Arc},
};

fn run_and_assert<K, V>(transactions: Vec<Transaction<K, V>>)
where
    K: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug + 'static,
    V: Send + Sync + Debug + Clone + Eq + TransactionWrite + 'static,
{
    let data_view = DeltaDataView::<K, V> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    let executable_transactions = ExecutableTransactions::Unsharded(transactions);
    let num_txns = executable_transactions.num_transactions();
    let output = BlockExecutor::<
        Transaction<K, V>,
        Task<K, V>,
        DeltaDataView<K, V>,
        ExecutableTestType,
    >::new(num_cpus::get(), executor_thread_pool, None)
    .execute_transactions_parallel(
        (),
        num_txns,
        (0..num_txns).map(|x|x as TxnIndex).collect(),
        &executable_transactions,
        &data_view,
    );

    let baseline = ExpectedOutput::generate_baseline(
        executable_transactions
            .get_unsharded_transactions()
            .unwrap(),
        None,
        None,
    );
    baseline.assert_output(&output);
}

fn random_value(delete_value: bool) -> ValueType<Vec<u8>> {
    ValueType((0..4).map(|_| (random::<u8>())).collect(), !delete_value)
}

#[test]
fn empty_block() {
    // This test checks that we do not trigger asserts due to an empty block, e.g. in the
    // scheduler. Instead, parallel execution should gracefully early return empty output.
    run_and_assert::<KeyType<[u8; 32]>, ValueType<[u8; 32]>>(vec![]);
}

#[test]
fn delta_counters() {
    let key = KeyType(random::<[u8; 32]>(), false);
    let mut transactions = vec![Transaction::Write {
        incarnation: Arc::new(AtomicUsize::new(0)),
        reads: vec![vec![]],
        writes_and_deltas: vec![(vec![(key, random_value(false))], vec![])],
    }];

    for _ in 0..50 {
        transactions.push(Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            reads: vec![vec![key]],
            writes_and_deltas: vec![(vec![], vec![(key, delta_add(5, u128::MAX))])],
        });
    }

    transactions.push(Transaction::Write {
        incarnation: Arc::new(AtomicUsize::new(0)),
        reads: vec![vec![]],
        writes_and_deltas: vec![(vec![(key, random_value(false))], vec![])],
    });

    for _ in 0..50 {
        transactions.push(Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            reads: vec![vec![key]],
            writes_and_deltas: vec![(vec![], vec![(key, delta_sub(2, u128::MAX))])],
        });
    }

    run_and_assert(transactions)
}

#[test]
fn delta_chains() {
    let mut transactions = vec![];
    // Generate a series of transactions add and subtract from an aggregator.

    let keys: Vec<KeyType<[u8; 32]>> = (0..10)
        .map(|_| KeyType(random::<[u8; 32]>(), false))
        .collect();

    for i in 0..500 {
        transactions.push(
            Transaction::Write::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
                incarnation: Arc::new(AtomicUsize::new(0)),
                reads: vec![keys.clone()],
                writes_and_deltas: vec![(
                    vec![],
                    keys.iter()
                        .enumerate()
                        .filter_map(|(j, k)| match (i + j) % 2 == 0 {
                            true => Some((
                                *k,
                                // Deterministic pattern for adds/subtracts.
                                DeltaOp::new(
                                    if (i % 2 == 0) == (j < 5) {
                                        DeltaUpdate::Plus(10)
                                    } else {
                                        DeltaUpdate::Minus(1)
                                    },
                                    // below params irrelevant for this test.
                                    u128::MAX,
                                    0,
                                    0,
                                ),
                            )),
                            false => None,
                        })
                        .collect(),
                )],
            },
        )
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
            transactions.push(Transaction::Write {
                incarnation: Arc::new(AtomicUsize::new(0)),
                reads: vec![vec![KeyType(key, false)]],
                writes_and_deltas: vec![(vec![(KeyType(key, false), random_value(false))], vec![])],
            })
        }
    }
    run_and_assert(transactions)
}

const NUM_BLOCKS: u64 = 10;
const TXN_PER_BLOCK: u64 = 100;

/// Make a list of indices of pattern `[5, 15, 25, ...]`.
fn make_non_contiguous_indices(num: usize) -> Vec<TxnIndex> {
    (0..num as TxnIndex).map(|i| i * 10 + 5).collect()
}

/// Make an IndexMapping of a sub-block, assuming there are `num_txns*10` txns in the block,
/// and the global indices of the txns in the sub-block is `[5, 15, ..., num_txns*10-5]`.
fn make_non_contiguous_index_mapping(num_txns: usize) -> IndexMapping {
    let indices = make_non_contiguous_indices(num_txns);
    IndexMapping::new(indices, num_txns * 10)
}

#[test]
fn one_reads_all_barrier() {
    let mut transactions = vec![];
    let keys: Vec<KeyType<_>> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType(random::<[u8; 32]>(), false))
        .collect();
    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                incarnation: Arc::new(AtomicUsize::new(0)),
                reads: vec![vec![*key]],
                writes_and_deltas: vec![(vec![(*key, random_value(false))], vec![])],
            })
        }
        // One transaction reading the write results of every prior transactions in the block.
        transactions.push(Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            reads: vec![keys.clone()],
            writes_and_deltas: vec![(vec![], vec![])],
        })
    }
    run_and_assert(transactions)
}

#[test]
fn one_writes_all_barrier() {
    let mut transactions = vec![];
    let keys: Vec<KeyType<_>> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType(random::<[u8; 32]>(), false))
        .collect();
    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                incarnation: Arc::new(AtomicUsize::new(0)),
                reads: vec![vec![*key]],
                writes_and_deltas: vec![(vec![(*key, random_value(false))], vec![])],
            })
        }
        // One transaction writing to the write results of every prior transactions in the block.
        transactions.push(Transaction::Write {
            incarnation: Arc::new(AtomicUsize::new(0)),
            reads: vec![keys.clone()],
            writes_and_deltas: vec![(
                keys.iter()
                    .map(|key| (*key, random_value(false)))
                    .collect::<Vec<_>>(),
                vec![],
            )],
        })
    }
    run_and_assert(transactions)
}

#[test]
fn early_aborts() {
    let mut transactions = vec![];
    let keys: Vec<_> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType(random::<[u8; 32]>(), false))
        .collect();

    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                incarnation: Arc::new(AtomicUsize::new(0)),
                reads: vec![vec![*key]],
                writes_and_deltas: vec![(vec![(*key, random_value(false))], vec![])],
            })
        }
        // One transaction that triggers an abort
        transactions.push(Transaction::Abort)
    }
    run_and_assert(transactions)
}

#[test]
fn early_skips() {
    let mut transactions = vec![];
    let keys: Vec<_> = (0..TXN_PER_BLOCK)
        .map(|_| KeyType(random::<[u8; 32]>(), false))
        .collect();

    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                incarnation: Arc::new(AtomicUsize::new(0)),
                reads: vec![vec![*key]],
                writes_and_deltas: vec![(vec![(*key, random_value(false))], vec![])],
            })
        }
        // One transaction that triggers an abort
        transactions.push(Transaction::SkipRest)
    }
    run_and_assert(transactions)
}

#[test]
fn scheduler_tasks() {
    scheduler_tasks_main(IndexMapping::new_unsharded(5));
}

#[test]
fn scheduler_tasks_with_non_contiguous_indices() {
    let index_mapping = make_non_contiguous_index_mapping(5);
    scheduler_tasks_main(index_mapping);
}

fn scheduler_tasks_main(index_mapping: IndexMapping) {
    let s = Scheduler::new(index_mapping.clone());

    for &i in index_mapping.indices.iter() {
        // No validation tasks.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if i == j
        ));
    }

    for &i in index_mapping.indices.iter() {
        // Validation index is at 0, so transactions will be validated and no
        // need to return a validation task to the caller.
        assert!(matches!(
            s.finish_execution(i, 0, false),
            SchedulerTask::NoTask
        ));
    }

    for &i in index_mapping.indices.iter() {
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ValidationTask((j, 0), 0) if i == j
        ));
    }

    // successful aborts.
    assert!(s.try_abort(index_mapping.indices[3], 0));
    s.finish_validation(index_mapping.indices[4], 0);
    assert!(s.try_abort(index_mapping.indices[4], 0)); // can abort even after successful validation
    assert!(s.try_abort(index_mapping.indices[1], 0));

    // unsuccessful aborts
    assert!(!s.try_abort(index_mapping.indices[1], 0));
    assert!(!s.try_abort(index_mapping.indices[3], 0));

    assert!(matches!(
        s.finish_abort(index_mapping.indices[4], 0),
        SchedulerTask::ExecutionTask((index, 1), None) if index == index_mapping.indices[4]
    ));
    assert!(matches!(
        s.finish_abort(index_mapping.indices[1], 0),
        SchedulerTask::ExecutionTask((index, 1), None) if index == index_mapping.indices[1]
    ));
    // Validation index = 2, wave = 1.
    assert!(matches!(
        s.finish_abort(index_mapping.indices[3], 0),
        SchedulerTask::ExecutionTask((index, 1), None) if index == index_mapping.indices[3]
    ));

    assert!(matches!(
        s.finish_execution(index_mapping.indices[4], 1, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 1, false),
        SchedulerTask::ValidationTask((index, 1), 1) if index == index_mapping.indices[1]
    ));

    // Another validation task for (2, 0).
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((index, 0), 1) if index == index_mapping.indices[2]
    ));
    // Now skip over txn 3 (status is Executing), and validate 4.
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((index, 1), 1) if index == index_mapping.indices[4]
    ));

    assert!(matches!(
        s.finish_execution(index_mapping.indices[3], 1, false),
        SchedulerTask::ValidationTask((index, 1), 1) if index == index_mapping.indices[3]
    ));

    s.finish_validation(index_mapping.indices[0], 0);
    s.finish_validation(index_mapping.indices[1], 2);
    for i in 2..5 {
        s.finish_validation(index_mapping.indices[i], 2)
    }

    // Make sure everything can be committed.
    for i in 0..5 {
        assert_some_eq!(s.try_commit(), index_mapping.indices[i]);
    }

    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn scheduler_first_wave() {
    scheduler_first_wave_main(IndexMapping::new_unsharded(6));
}

#[test]
fn scheduler_first_wave_with_non_contiguous_indices() {
    let index_mapping = make_non_contiguous_index_mapping(6);
    scheduler_first_wave_main(index_mapping);
}

fn scheduler_first_wave_main(index_mapping: IndexMapping) {
    let s = Scheduler::new(index_mapping.clone());

    for &i in index_mapping
        .indices
        .iter()
        .take(index_mapping.num_txns() - 1)
    {
        // Nothing to validate.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    // validation index will not increase for the first execution wave
    // until the status becomes executed.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[0], 0, false),
        SchedulerTask::NoTask
    ));

    // Now we can validate version (0, 0).
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((index, 0), 0) if index == index_mapping.indices[0]
    ));
    let x = s.next_task(false);
    assert!(matches!(
        x,
        SchedulerTask::ExecutionTask((index, 0), None) if index == index_mapping.indices[5]
    ));
    // Since (1, 0) is not EXECUTED, no validation tasks, and execution index
    // is already at the limit, so no tasks immediately available.
    assert!(matches!(s.next_task(false), SchedulerTask::NoTask));

    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 0, false),
        SchedulerTask::NoTask
    ));
    // There should be no tasks, but finishing (1,0) should enable validating
    // (1, 0) then (2,0).
    assert!(matches!(s.next_task(false), SchedulerTask::NoTask));

    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 0, false),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if i == index_mapping.indices[1]
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if i == index_mapping.indices[2]
    ));
    assert!(matches!(s.next_task(false), SchedulerTask::NoTask));
}

#[test]
fn scheduler_dependency() {
    scheduler_dependency_main(IndexMapping::new_unsharded(10));
}

#[test]
fn scheduler_dependency_with_non_contiguous_indices() {
    let index_mapping = make_non_contiguous_index_mapping(10);
    scheduler_dependency_main(index_mapping);
}

fn scheduler_dependency_main(index_mapping: IndexMapping) {
    let s = Scheduler::new(index_mapping.clone());

    for &i in index_mapping.indices.iter() {
        // Nothing to validate.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    // validation index will not increase for the first execution wave
    // until the status becomes executed.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[0], 0, false),
        SchedulerTask::NoTask
    ));
    // Now we can validate version (0, 0).
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if i == index_mapping.indices[0]
    ));
    // Current status of 0 is executed - hence, no dependency added.
    assert!(matches!(
        s.wait_for_dependency(index_mapping.indices[3], index_mapping.indices[0]),
        DependencyResult::Resolved
    ));
    // Dependency added for transaction 4 on transaction 2.
    assert!(matches!(
        s.wait_for_dependency(index_mapping.indices[4], index_mapping.indices[2]),
        DependencyResult::Dependency(_)
    ));

    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 0, false),
        SchedulerTask::NoTask
    ));

    // resumed task doesn't bump incarnation
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((i, 0), Some(_)) if i == index_mapping.indices[4]
    ));
}

// Will return a scheduler in a state where all transactions are scheduled for
// for execution, validation index = num_txns, and wave = 0.
fn incarnation_one_scheduler(index_mapping: &IndexMapping) -> Scheduler {
    let s = Scheduler::new(index_mapping.clone());

    for &i in index_mapping.indices.iter() {
        // Get the first executions out of the way.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
        assert!(matches!(
            s.finish_execution(i, 0, false),
            SchedulerTask::NoTask
        ));
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ValidationTask((j, 0), 0) if i == j
        ));
        assert!(s.try_abort(i, 0));
        assert!(matches!(
            s.finish_abort(i, 0),
            SchedulerTask::ExecutionTask((j, 1), None) if i == j
        ));
    }
    s
}

#[test]
fn scheduler_incarnation() {
    scheduler_incarnation_main(IndexMapping::new_unsharded(5));
}

#[test]
fn scheduler_incarnation_with_non_contiguous_indices() {
    scheduler_incarnation_main(make_non_contiguous_index_mapping(5));
}

fn scheduler_incarnation_main(index_mapping: IndexMapping) {
    let s = incarnation_one_scheduler(&index_mapping);

    // execution/validation index = 5, wave = 0.
    assert!(matches!(
        s.wait_for_dependency(index_mapping.indices[1], index_mapping.indices[0]),
        DependencyResult::Dependency(_)
    ));
    assert!(matches!(
        s.wait_for_dependency(index_mapping.indices[3], index_mapping.indices[0]),
        DependencyResult::Dependency(_)
    ));

    // Because validation index is higher, return validation task to caller (even with
    // revalidate_suffix = true) - because now we always decrease validation idx to txn_idx + 1
    // here validation wave increases to 1, and index is reduced to 3.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 1, true),
        SchedulerTask::ValidationTask((i, 1), 1) if i == index_mapping.indices[2]
    ));
    // Here since validation index is lower, wave doesn't increase and no task returned.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[4], 1, true),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 1), 1) if i == index_mapping.indices[4],
    ));

    assert!(s.try_abort(index_mapping.indices[2], 1));
    assert!(s.try_abort(index_mapping.indices[4], 1));
    assert!(!s.try_abort(index_mapping.indices[2], 1));

    assert!(matches!(
        s.finish_abort(index_mapping.indices[2], 1),
        SchedulerTask::ExecutionTask((i, 2), None) if i == index_mapping.indices[2]
    ));
    // wave = 2, validation index = 2.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[0], 1, false),
        SchedulerTask::ValidationTask((i, 1), 2) if i == index_mapping.indices[0]
    ));
    // execution index =  1

    assert!(matches!(
        s.finish_abort(index_mapping.indices[4], 1),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((i, 1), Some(_)) if i == index_mapping.indices[1]
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((i, 1), Some(_)) if i == index_mapping.indices[3]
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((i, 2), None) if i == index_mapping.indices[4]
    ));
    // execution index = 5

    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 1, false),
        SchedulerTask::ValidationTask((i, 1), 2) if i == index_mapping.indices[1]
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 2, false),
        SchedulerTask::ValidationTask((i, 2), 2) if i == index_mapping.indices[2]
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[3], 1, false),
        SchedulerTask::ValidationTask((i, 1), 2) if i == index_mapping.indices[3]
    ));

    // validation index is 4, so finish execution doesn't return validation task, next task does.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[4], 2, false),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 2), 2) if i == index_mapping.indices[4]
    ));
}

#[test]
fn scheduler_basic() {
    scheduler_basic_main(IndexMapping::new_unsharded(3));
}

#[test]
fn scheduler_basic_with_non_contiguous_indices() {
    scheduler_basic_main(make_non_contiguous_index_mapping(3));
}

fn scheduler_basic_main(index_mapping: IndexMapping) {
    let s = Scheduler::new(index_mapping.clone());

    for &i in index_mapping.indices.iter() {
        // Nothing to validate.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    // Finish executions & dispatch validation tasks.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[0], 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if index_mapping.indices[0] == i
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if index_mapping.indices[1] == i
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if index_mapping.indices[2] == i
    ));

    for &i in index_mapping.indices.iter() {
        s.finish_validation(i, 1)
    }

    // make sure everything can be committed.
    for &i in index_mapping.indices.iter() {
        assert_some_eq!(s.try_commit(), i);
    }

    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn scheduler_drain_idx() {
    scheduler_drain_idx_main(IndexMapping::new_unsharded(3));
}

#[test]
fn scheduler_drain_idx_with_non_contiguous_indices() {
    scheduler_drain_idx_main(make_non_contiguous_index_mapping(3));
}

fn scheduler_drain_idx_main(index_mapping: IndexMapping) {
    let s = Scheduler::new(index_mapping.clone());

    for &i in index_mapping.indices.iter() {
        // Nothing to validate.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    // Finish executions & dispatch validation tasks.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[0], 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if i == index_mapping.indices[0]
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if i == index_mapping.indices[1]
    ));
    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((i, 0), 0) if i == index_mapping.indices[2]
    ));

    for &i in index_mapping.indices.iter() {
        s.finish_validation(i, 1)
    }

    // make sure everything can be committed.
    for &i in index_mapping.indices.iter() {
        assert_some_eq!(s.try_commit(), i);
    }

    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn finish_execution_wave() {
    // Wave won't be increased, because validation index is already 2, and finish_execution
    // tries to reduce it to 2.
    let index_mapping = IndexMapping::new_unsharded(2);
    let s = incarnation_one_scheduler(&index_mapping);
    assert!(matches!(
        s.finish_execution(1, 1, true),
        SchedulerTask::ValidationTask((1, 1), 0),
    ));

    // Here wave will increase, because validation index is reduced from 3 to 2.
    let index_mapping = IndexMapping::new_unsharded(3);
    let s = incarnation_one_scheduler(&index_mapping);
    assert!(matches!(
        s.finish_execution(1, 1, true),
        SchedulerTask::ValidationTask((1, 1), 1),
    ));

    // Here wave won't be increased, because we pass revalidate_suffix = false.
    let index_mapping = IndexMapping::new_unsharded(3);
    let s = incarnation_one_scheduler(&index_mapping);
    assert!(matches!(
        s.finish_execution(1, 1, false),
        SchedulerTask::ValidationTask((1, 1), 0),
    ));
}

#[test]
fn finish_execution_wave_with_non_contiguous_indices() {
    // Wave won't be increased, because validation index is already 2, and finish_execution
    // tries to reduce it to 2.
    let index_mapping = make_non_contiguous_index_mapping(2);
    let s = incarnation_one_scheduler(&index_mapping);
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 1, true),
        SchedulerTask::ValidationTask((i, 1), 0) if i == index_mapping.indices[1],
    ));

    // Here wave will increase, because validation index is reduced from 3 to 2.
    let index_mapping = make_non_contiguous_index_mapping(3);
    let s = incarnation_one_scheduler(&index_mapping);
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 1, true),
        SchedulerTask::ValidationTask((i, 1), 1) if i == index_mapping.indices[1],
    ));

    // Here wave won't be increased, because we pass revalidate_suffix = false.
    let index_mapping = make_non_contiguous_index_mapping(3);
    let s = incarnation_one_scheduler(&index_mapping);
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 1, false),
        SchedulerTask::ValidationTask((i, 1), 0) if i == index_mapping.indices[1],
    ));
}

#[test]
fn rolling_commit_wave() {
    rolling_commit_wave_main(IndexMapping::new_unsharded(3));
}

#[test]
fn rolling_commit_wave_with_non_contiguous_indices() {
    rolling_commit_wave_main(make_non_contiguous_index_mapping(3));
}

fn rolling_commit_wave_main(index_mapping: IndexMapping) {
    let s = incarnation_one_scheduler(&index_mapping);

    // Finish execution for txn 0 without validate_suffix and because
    // validation index is higher will return validation task to the caller.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[0], 1, false),
        SchedulerTask::ValidationTask((i, 1), 0) if i == index_mapping.indices[0]
    ));
    // finish validating txn 0 with proper wave
    s.finish_validation(index_mapping.indices[0], 1);
    // txn 0 can be committed
    assert_some_eq!(s.try_commit(), index_mapping.indices[0]);
    assert_eq!(s.commit_state(), (index_mapping.indices[1], 0));

    // This increases the wave, but only sets max_triggered_wave for transaction 2.
    // sets validation_index to 2.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[1], 1, true),
        SchedulerTask::ValidationTask((i, 1), 1) if i == index_mapping.indices[1],
    ));

    // finish validating txn 1 with lower wave
    s.finish_validation(index_mapping.indices[1], 0);
    // txn 1 cannot be committed
    assert!(s.try_commit().is_none());
    assert_eq!(s.commit_state(), (index_mapping.indices[1], 0));

    // finish validating txn 1 with proper wave
    s.finish_validation(index_mapping.indices[1], 1);
    // txn 1 can be committed
    assert_some_eq!(s.try_commit(), index_mapping.indices[1]);
    assert_eq!(s.commit_state(), (index_mapping.indices[2], 0));

    // No validation task because index is already 2.
    assert!(matches!(
        s.finish_execution(index_mapping.indices[2], 1, false),
        SchedulerTask::NoTask,
    ));
    // finish validating with a lower wave.
    s.finish_validation(index_mapping.indices[2], 0);
    assert!(s.try_commit().is_none());
    assert_eq!(s.commit_state(), (index_mapping.indices[2], 1));
    // Finish validation with appropriate wave.
    s.finish_validation(index_mapping.indices[2], 1);
    assert_some_eq!(s.try_commit(), index_mapping.indices[2]);
    assert_eq!(s.commit_state(), (index_mapping.end_index, 1));

    // All txns have been committed.
    assert!(matches!(s.next_task(false), SchedulerTask::Done));
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

    let num_txns = 1000;
    for num_concurrent_tasks in [1, 5, 10, 20] {
        let index_mappings = [
            IndexMapping::new_unsharded(num_txns),
            make_non_contiguous_index_mapping(num_txns),
        ];
        for index_mapping in index_mappings {
            let s = Scheduler::new(index_mapping.clone());

            let mut tasks = BTreeMap::new();

            let mut rng = rand::thread_rng();
            let mut num_exec_tasks = 0;
            let mut num_val_tasks = 0;

            loop {
                while tasks.len() < num_concurrent_tasks {
                    match s.next_task(false) {
                        SchedulerTask::ExecutionTask((txn_idx, incarnation), _) => {
                            assert_eq!(incarnation, 0);
                            // true means an execution task.
                            tasks.insert(rng.gen::<u32>(), (true, txn_idx));
                        },
                        SchedulerTask::ValidationTask((txn_idx, incarnation), cur_wave) => {
                            assert_eq!(incarnation, 0);
                            assert_eq!(cur_wave, 0);
                            // false means a validation task.
                            tasks.insert(rng.gen::<u32>(), (false, txn_idx));
                        },
                        SchedulerTask::NoTask => break,
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
                            if let SchedulerTask::ValidationTask((idx, incarnation), wave) =
                                task_res
                            {
                                assert_eq!(idx, txn_idx);
                                assert_eq!(incarnation, 0);
                                assert_eq!(wave, 0);
                                tasks.insert(rng.gen::<u32>(), (false, txn_idx));
                            } else {
                                assert_matches!(task_res, SchedulerTask::NoTask);
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

            for &i in index_mapping.indices.iter() {
                assert_some_eq!(s.try_commit(), i);
                let expected = if i == index_mapping.indices[index_mapping.num_txns() - 1] {
                    index_mapping.end_index
                } else {
                    index_mapping.next_index(i)
                };
                assert_eq!(s.commit_state(), (expected, 0));
            }

            assert!(matches!(s.next_task(false), SchedulerTask::Done));
        }
    }
}
