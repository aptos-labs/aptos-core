// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::ParallelTransactionExecutor,
    proptest_types::types::{ExpectedOutput, Task, Transaction},
    scheduler::{Scheduler, SchedulerTask, TaskGuard},
};
use rand::random;
use std::{fmt::Debug, hash::Hash, sync::atomic::AtomicUsize};

fn run_and_assert<K, V>(transactions: Vec<Transaction<K, V>>)
where
    K: PartialOrd + Send + Sync + Clone + Hash + Eq + 'static,
    V: Send + Sync + Debug + Clone + Eq + 'static,
{
    let baseline = ExpectedOutput::generate_baseline(&transactions);

    let output = ParallelTransactionExecutor::<Transaction<K, V>, Task<K, V>>::new()
        .execute_transactions_parallel((), transactions);

    assert!(baseline.check_output(&output))
}

const TOTAL_KEY_NUM: u64 = 50;
const WRITES_PER_KEY: u64 = 100;

#[test]
fn cycle_transactions() {
    let mut transactions = vec![];
    // For every key in `TOTAL_KEY_NUM`, generate a series transaction that will assign a value to
    // this key.
    for _ in 0..TOTAL_KEY_NUM {
        let key = random::<[u8; 32]>();
        for _ in 0..WRITES_PER_KEY {
            transactions.push(Transaction::Write {
                reads: vec![key],
                actual_writes: vec![(key, random::<u64>())],
                skipped_writes: vec![],
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
    let keys: Vec<_> = (0..TXN_PER_BLOCK).map(|_| random::<[u8; 32]>()).collect();
    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                reads: vec![*key],
                actual_writes: vec![(*key, random::<u64>())],
                skipped_writes: vec![],
            })
        }
        // One transaction reading the write results of every prior transactions in the block.
        transactions.push(Transaction::Write {
            reads: keys.clone(),
            actual_writes: vec![],
            skipped_writes: vec![],
        })
    }
    run_and_assert(transactions)
}

#[test]
fn one_writes_all_barrier() {
    let mut transactions = vec![];
    let keys: Vec<_> = (0..TXN_PER_BLOCK).map(|_| random::<[u8; 32]>()).collect();
    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                reads: vec![*key],
                actual_writes: vec![(*key, random::<u64>())],
                skipped_writes: vec![],
            })
        }
        // One transaction writing to the write results of every prior transactions in the block.
        transactions.push(Transaction::Write {
            reads: keys.clone(),
            actual_writes: keys
                .iter()
                .map(|key| (*key, random::<u64>()))
                .collect::<Vec<_>>(),
            skipped_writes: vec![],
        })
    }
    run_and_assert(transactions)
}

#[test]
fn early_aborts() {
    let mut transactions = vec![];
    let keys: Vec<_> = (0..TXN_PER_BLOCK).map(|_| random::<[u8; 32]>()).collect();

    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                reads: vec![*key],
                actual_writes: vec![(*key, random::<u64>())],
                skipped_writes: vec![],
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
    let keys: Vec<_> = (0..TXN_PER_BLOCK).map(|_| random::<[u8; 32]>()).collect();

    for _ in 0..NUM_BLOCKS {
        for key in &keys {
            transactions.push(Transaction::Write {
                reads: vec![*key],
                actual_writes: vec![(*key, random::<u64>())],
                skipped_writes: vec![],
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
    let fake_counter = AtomicUsize::new(0);

    for i in 0..5 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask((j, 0), _) if i == j
        ));
    }

    // Finish execution for txns 0, 2, 4. txn 0 without validate_suffix and because
    // validation index is higher will return validation task to the caller.
    assert!(matches!(
        s.finish_execution(0, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((0, 0), _)
    ));
    // Requires revalidation suffix, so validation index will be decreased to 2,
    // and txn 4 will not need to return a validation task.
    assert!(matches!(
        s.finish_execution(2, 0, true, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));
    // txn 2's finish validation pulled back validation index, so 4 will get validated
    // and no need to return a validation task.
    assert!(matches!(
        s.finish_execution(4, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((2, 0), _)
    ));
    // txn 3 hasn't finished execution, so no validation task for it.
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((4, 0), _)
    ));

    // Validation index is decreased and no task returned to caller.
    assert!(matches!(
        s.finish_execution(3, 0, true, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((3, 0), _)
    ));
    // txn 4 dispatched for validation again because it the previous validation
    // hasn't finished.
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((4, 0), _)
    ));

    // successful abort.
    assert!(s.try_abort(3, 0));
    assert!(matches!(
        s.finish_execution(1, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((1, 0), _)
    ));

    // unsuccessful abort.
    assert!(!s.try_abort(3, 0));
    assert!(matches!(
        s.finish_abort(3, 0, TaskGuard::new(&fake_counter)),
        SchedulerTask::ExecutionTask((3, 1), _)
    ));

    // can abort even after succesful validation
    assert!(s.try_abort(4, 0));
    assert!(matches!(
        s.finish_abort(4, 0, TaskGuard::new(&fake_counter)),
        SchedulerTask::ExecutionTask((4, 1), _)
    ));

    // txn 4 is aborted, so there won't be a validation task.
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask((5, 0), _)
    ));
    // Wrap up all outstanding tasks.
    assert!(matches!(
        s.finish_execution(4, 1, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((4, 1), _)
    ));
    assert!(matches!(
        s.finish_execution(3, 1, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((3, 1), _)
    ));

    assert!(matches!(
        s.finish_execution(5, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((5, 0), _)
    ));

    assert!(matches!(s.next_task(), SchedulerTask::Done));
}

#[test]
fn scheduler_dependency() {
    let s = Scheduler::new(10);
    let fake_counter = AtomicUsize::new(0);

    for i in 0..5 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask((j, 0), _) if j == i
        ));
    }

    assert!(matches!(
        s.finish_execution(0, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((0, 0), _)
    ));
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask((5, 0), _)
    ));

    assert!(!s.try_add_dependency(3, 0));
    assert!(s.try_add_dependency(4, 2));

    assert!(matches!(
        s.finish_execution(2, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((2, 0), _)
    ));
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask((4, 1), _)
    ));
}

#[test]
fn scheduler_incarnation() {
    let s = Scheduler::new(5);
    let fake_counter = AtomicUsize::new(0);

    for i in 0..5 {
        // not calling finish execution, so validation tasks not dispatched.
        assert!(matches!(
            s.next_task(),
            SchedulerTask::ExecutionTask((j, 0), _) if j == i
        ));
    }
    // execution index = 5
    assert!(s.try_add_dependency(1, 0));
    assert!(s.try_add_dependency(3, 0));

    assert!(matches!(
        s.finish_execution(2, 0, true, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.finish_execution(4, 0, true, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((2, 0), _)
    ));
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((4, 0), _)
    ));

    assert!(s.try_abort(2, 0));
    assert!(s.try_abort(4, 0));
    assert!(!s.try_abort(2, 0));

    assert!(matches!(
        s.finish_abort(2, 0, TaskGuard::new(&fake_counter)),
        SchedulerTask::ExecutionTask((2, 1), _)
    ));

    assert!(matches!(
        s.finish_execution(0, 0, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((0, 0), _)
    ));
    // execution index =  1

    assert!(matches!(
        s.finish_abort(4, 0, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));

    assert!(matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask((1, 1), _)
    ));
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask((3, 1), _)
    ));
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ExecutionTask((4, 1), _)
    ));
    // execution index = 5

    assert!(matches!(
        s.finish_execution(1, 1, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((1, 1), _)
    ));
    assert!(matches!(
        s.finish_execution(2, 1, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((2, 1), _)
    ));
    assert!(matches!(
        s.finish_execution(3, 1, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::ValidationTask((3, 1), _)
    ));

    // validation index is 4, so finish execution doesn't return validation task, next task does.
    assert!(matches!(
        s.finish_execution(4, 1, false, TaskGuard::new(&fake_counter)),
        SchedulerTask::NoTask
    ));
    assert!(matches!(
        s.next_task(),
        SchedulerTask::ValidationTask((4, 1), _)
    ));

    assert!(matches!(s.next_task(), SchedulerTask::Done));
}
