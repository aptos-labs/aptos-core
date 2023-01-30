// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::BlockExecutor,
    proptest_types::types::{DeltaDataView, ExpectedOutput, KeyType, Task, Transaction, ValueType},
    scheduler::{Scheduler, SchedulerTask},
    task::ModulePath,
};
use aptos_aggregator::delta_change_set::{delta_add, delta_sub, DeltaOp, DeltaUpdate};
use aptos_types::write_set::TransactionWrite;
use rand::random;
use std::{
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

    let output =
        BlockExecutor::<Transaction<K, V>, Task<K, V>, DeltaDataView<K, V>>::new(num_cpus::get())
            .execute_transactions_parallel((), &transactions, &data_view)
            .map(|zipped| zipped.into_iter().map(|(res, _)| res).collect());

    let baseline = ExpectedOutput::generate_baseline(&transactions, None);

    baseline.assert_output(&output);
}

fn random_value(delete_value: bool) -> ValueType<Vec<u8>> {
    ValueType((0..4).map(|_| (random::<u8>())).collect(), !delete_value)
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
    let s = Scheduler::new(6);

    for i in 0..5 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if i == j
        ));
    }

    // Finish execution for txns 0, 2, 4. txn 0 without validate_suffix and because
    // validation index is higher will return validation task to the caller.
    assert!(matches!(
        s.finish_execution(0, 0, false),
        SchedulerTask::ValidationTask((0, 0), 0)
    ));
    // Requires revalidation suffix, so validation index will be decreased to 2,
    // and txn 4 will not need to return a validation task.
    assert!(matches!(
        s.finish_execution(2, 0, true),
        SchedulerTask::NoTask
    ));
    // txn 2's finish validation pulled back validation index, so 4 will get validated
    // and no need to return a validation task.
    assert!(matches!(
        s.finish_execution(4, 0, false),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((2, 0), 1)
    ));
    // txn 3 hasn't finished execution, so no validation task for it.
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((4, 0), 1)
    ));

    // Validation index is decreased and no task returned to caller.
    assert!(matches!(
        s.finish_execution(3, 0, true),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((3, 0), 2)
    ));
    // txn 4 dispatched for validation again because it the previous validation
    // hasn't finished.
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((4, 0), 2)
    ));

    // successful abort.
    assert!(s.try_abort(3, 0));
    assert!(matches!(
        s.finish_execution(1, 0, false),
        // wave is 2 since validation index is decreased twice
        SchedulerTask::ValidationTask((1, 0), 2)
    ));

    // unsuccessful abort.
    assert!(!s.try_abort(3, 0));
    assert!(matches!(
        s.finish_abort(3, 0),
        SchedulerTask::ExecutionTask((3, 1), None)
    ));

    // can abort even after succesful validation
    assert!(s.try_abort(4, 0));
    assert!(matches!(
        s.finish_abort(4, 0),
        SchedulerTask::ExecutionTask((4, 1), None)
    ));

    // txn 4 is aborted, so there won't be a validation task.
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((5, 0), None)
    ));
    // Wrap up all outstanding tasks.
    assert!(matches!(
        s.finish_execution(4, 1, false),
        SchedulerTask::ValidationTask((4, 1), 3)
    ));
    assert!(matches!(
        s.finish_execution(3, 1, false),
        SchedulerTask::ValidationTask((3, 1), 3)
    ));

    assert!(matches!(
        s.finish_execution(5, 0, false),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((5, 0), 3)
    ));

    s.finish_validation(0, 0);
    s.finish_validation(1, 2);
    for i in 2..6 {
        s.finish_validation(i, 3)
    }

    while s.try_commit().is_some() {}

    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn scheduler_dependency() {
    let s = Scheduler::new(10);

    for i in 0..5 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    assert!(matches!(
        s.finish_execution(0, 0, false),
        SchedulerTask::ValidationTask((0, 0), _)
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((5, 0), None)
    ));

    assert!(s.wait_for_dependency(3, 0).is_none());
    assert!(s.wait_for_dependency(4, 2).is_some());

    assert!(matches!(
        s.finish_execution(2, 0, false),
        SchedulerTask::ValidationTask((2, 0), _)
    ));
    // resumed task doesn't bump incarnation
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((4, 0), Some(_))
    ));
}

#[test]
fn scheduler_incarnation() {
    let s = Scheduler::new(5);

    for i in 0..5 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    // execution index = 5
    assert!(s.wait_for_dependency(1, 0).is_some());
    assert!(s.wait_for_dependency(3, 0).is_some());

    assert!(matches!(
        s.finish_execution(2, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(4, 0, true),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((2, 0), _)
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((4, 0), _)
    ));

    assert!(s.try_abort(2, 0));
    assert!(s.try_abort(4, 0));
    assert!(!s.try_abort(2, 0));

    assert!(matches!(
        s.finish_abort(2, 0),
        SchedulerTask::ExecutionTask((2, 1), None)
    ));

    assert!(matches!(
        s.finish_execution(0, 0, false),
        SchedulerTask::ValidationTask((0, 0), _)
    ));
    // execution index =  1

    assert!(matches!(s.finish_abort(4, 0), SchedulerTask::NoTask));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((1, 0), Some(_))
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((3, 0), Some(_))
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((4, 1), None)
    ));
    // execution index = 5

    assert!(matches!(
        s.finish_execution(1, 0, false),
        SchedulerTask::ValidationTask((1, 0), _)
    ));
    assert!(matches!(
        s.finish_execution(2, 1, false),
        SchedulerTask::ValidationTask((2, 1), _)
    ));
    assert!(matches!(
        s.finish_execution(3, 0, false),
        SchedulerTask::ValidationTask((3, 0), _)
    ));

    // validation index is 4, so finish execution doesn't return validation task, next task does.
    assert!(matches!(
        s.finish_execution(4, 1, false),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((4, 1), _)
    ));
}

#[test]
fn scheduler_stop_idx() {
    let s = Scheduler::new(3);

    for i in 0..2 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((2, 0), None)
    ));

    // Finish executions & dispatch validation tasks.
    assert!(matches!(
        s.finish_execution(0, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(1, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((0, 0), 1)
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((1, 0), 1)
    ));
    assert!(matches!(
        s.finish_execution(2, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((2, 0), 1)
    ));

    for i in 0..3 {
        s.finish_validation(i, 1)
    }

    while s.try_commit().is_some() {}

    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn scheduler_drain_idx() {
    let s = Scheduler::new(3);

    for i in 0..3 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(false),
            SchedulerTask::ExecutionTask((j, 0), None) if j == i
        ));
    }

    // Finish executions & dispatch validation tasks.
    assert!(matches!(
        s.finish_execution(0, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(1, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((0, 0), 1)
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((1, 0), 1)
    ));
    assert!(matches!(
        s.finish_execution(2, 0, true),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ValidationTask((2, 0), 1)
    ));

    for i in 0..3 {
        s.finish_validation(i, 1)
    }

    while s.try_commit().is_some() {}

    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn test_rolling_commit_wave() {
    let s = Scheduler::new(2);

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((0, 0), None)
    ));

    // Finish execution for txns 0 without validate_suffix and because
    // validation index is higher will return validation task to the caller.
    assert!(matches!(
        s.finish_execution(0, 0, false),
        SchedulerTask::NoTask
    ));
    // finish validating txn 0 with proper wave
    s.finish_validation(0, 0);
    // txn 0 can be committed
    assert!(s.try_commit().is_some());
    assert!(matches!(s.commit_state(), (1, 0)));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((1, 0), None)
    ));

    // Increase validation_index
    assert!(matches!(s.next_task(false), SchedulerTask::NoTask));

    // Requires revalidation suffix, so validation index will be decreased to 1
    assert!(matches!(
        s.finish_execution(1, 0, true),
        SchedulerTask::NoTask
    ));

    // finish validating txn 1 with lower wave
    s.finish_validation(1, 0);
    // txn 1 cannot be committed
    assert!(s.try_commit().is_none());
    assert!(matches!(s.commit_state(), (1, 1)));

    // finish validating txn 1 with proper wave
    s.finish_validation(1, 1);
    // txn 1 can be committed
    assert!(s.try_commit().is_some());
    // commit_state wave is updated
    assert!(matches!(s.commit_state(), (2, 1)));

    // All txns have been committed.
    assert!(s.try_commit().is_none());
    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}

#[test]
fn test_rolling_commit_wave_update() {
    let s = Scheduler::new(2);
    // create txn 0 with max_triggered_wave = 0 and required_wave = 1
    // create txn 1 with max_triggered_wave = 1 and required_wave = 1

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((0, 0), None)
    ));

    assert!(matches!(
        s.next_task(false),
        SchedulerTask::ExecutionTask((1, 0), None)
    ));

    // Increase validation_index
    assert!(matches!(s.next_task(false), SchedulerTask::NoTask));

    // Requires revalidation suffix, so validation index will be decreased to 1
    // validation_index wave = 1
    // txn 1 max_triggered_wave = 1
    assert!(matches!(
        s.finish_execution(1, 0, true),
        SchedulerTask::NoTask
    ));

    // The required_wave of txn 0 is 1
    assert!(matches!(
        s.finish_execution(0, 0, false),
        SchedulerTask::ValidationTask((0, 0), 1)
    ));

    // finish validating txn 0 with lower wave
    s.finish_validation(0, 0);
    // txn 0 cannot be committed since the required_wave of txn 0 is 1
    assert!(s.try_commit().is_none());
    assert!(matches!(s.commit_state(), (0, 0)));

    // finish validating txn 0 with proper wave
    s.finish_validation(0, 1);
    // txn 0 can be committed
    assert!(s.try_commit().is_some());
    assert!(matches!(s.commit_state(), (1, 0)));

    // finish validating txn 1 with lower wave
    s.finish_validation(1, 0);
    // txn 1 cannot be committed
    assert!(s.try_commit().is_none());
    assert!(matches!(s.commit_state(), (1, 1)));

    // finish validating txn 1 with proper wave
    s.finish_validation(1, 1);
    // txn 1 can be committed
    assert!(s.try_commit().is_some());
    assert!(matches!(s.commit_state(), (2, 1)));

    // All txns have been committed.
    assert!(s.try_commit().is_none());
    assert!(matches!(s.next_task(false), SchedulerTask::Done));
}
