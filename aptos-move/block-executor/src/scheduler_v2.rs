// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scheduler::{DependencyCondvar, DependencyStatus},
    scheduler_status::{DependencyInstruction, DependencyResolution, ExecutionStatus},
};
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use concurrent_queue::{ConcurrentQueue, PopError};
use crossbeam::utils::CachePadded;
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

/**
A transaction may be (re-)executed multiple times, each time with an incremented
incarnation number. Each (txn_idx, incarnation) pair defines a full version, each
represented by a node in an abstract graph that the scheduler maintains. The prior
incarnation aborting is a precondition for the next incarnation to exist.

PanicError returned from APIs indicates scheduler internal invariant failure.
 **/

// Execution priority determined based on the proximity to the committed prefix.
#[derive(Clone, Copy)]
enum ExecutionPriority {
    Highest,
    High,
    Medium,
    Low,
}

// Describes downstream dependencies for a transaction that have previously gotten aborted
// due to reading the transaction's output (that changed). Since such dependencies might be
// detected in the system concurrently to stalling and unstalling, this structure also tracks
// which dependencies it has stalled (to unstall later). The implementation must maintain
// the invariant that the intersection of stalled and not_stalled is always empty.
struct AbortedDependencies {
    is_stalled: bool,
    not_stalled_deps: BTreeSet<TxnIndex>,
    stalled_deps: BTreeSet<TxnIndex>,
}

macro_rules! test_assert {
    ($($tt: tt)*) => {
        #[cfg(test)]
        assert!($($tt)*)
    }
}

impl AbortedDependencies {
    fn new() -> Self {
        Self {
            is_stalled: false,
            not_stalled_deps: BTreeSet::new(),
            stalled_deps: BTreeSet::new(),
        }
    }

    fn record_dependencies(&mut self, dependencies: impl Iterator<Item = TxnIndex>) {
        for dep in dependencies {
            if !self.stalled_deps.contains(&dep) {
                self.not_stalled_deps.insert(dep);
            }
        }
    }

    // Calls stall on the status and adds all indices from not_stalled to stalled. Inserts
    // the subset of indices for which stall returned true into the propagation queue.
    fn stall(
        &mut self,
        statuses: &[CachePadded<ExecutionStatus>],
        propagation_queue: &mut BTreeSet<TxnIndex>,
    ) -> Result<(), PanicError> {
        for idx in &self.not_stalled_deps {
            // Assert the invariant in tests.
            test_assert!(!self.stalled_deps.contains(&idx));

            if statuses[*idx as usize].stall()? {
                // May require recursive stalling.
                propagation_queue.insert(*idx);
            }
        }

        self.stalled_deps.append(&mut self.not_stalled_deps);
        self.is_stalled = true;
        Ok(())
    }

    // Calls unstall on the status and adds all indices from stalled to not_stalled. Inserts
    // the subset of indices for which unstall returned true into the propagation queue.
    // Additionally if status requires execution, index is added to the scheduling queue.
    fn unstall(
        &mut self,
        statuses: &[CachePadded<ExecutionStatus>],
        propagation_queue: &mut BTreeSet<TxnIndex>,
    ) -> Result<(), PanicError> {
        for idx in &self.stalled_deps {
            // Assert the invariant in tests.
            test_assert!(!self.not_stalled_deps.contains(&idx));

            if statuses[*idx as usize].unstall()? {
                // May require recursive unstalling.
                propagation_queue.insert(*idx);
            }
        }

        self.not_stalled_deps.append(&mut self.stalled_deps);
        self.is_stalled = false;
        Ok(())
    }
}

pub(crate) enum TaskKind {
    Execute(TxnIndex, Incarnation),
    Commit(TxnIndex),
    NextTask,
    Done,
}

pub(crate) struct SchedulerProxy {
    /// First executed_idx many transctions have all finished their first incarnation (i.e. have
    /// been executed at least once).
    executed_idx: CachePadded<AtomicU32>,
    /// Queue for scheduling transactions for execution. TODO: alternative implementations
    /// (e.g. packed ints, intervals w. locks, CachePadded<ConcurrentQueue<TxnIndex>>).
    execution_queue: Mutex<BTreeSet<TxnIndex>>,
}

impl SchedulerProxy {
    fn new(num_txns: TxnIndex) -> Self {
        Self {
            executed_idx: CachePadded::new(AtomicU32::new(0)),
            execution_queue: Mutex::new((0..num_txns).collect()),
        }
    }

    fn pop_next(&self) -> Option<TxnIndex> {
        self.execution_queue.lock().pop_first()
    }

    // is_first_reexecution must be determined and the method must be performed while holding
    // the idx-th status lock.
    pub(crate) fn add_to_schedule(&self, is_first_reexecution: bool, txn_idx: TxnIndex) {
        // If incarnation one (first re-execution) was not scheduled because of the executed_idx
        // check, it will be when executed_idx catches up (try_increase_executed_idx function).
        if !is_first_reexecution || self.executed_idx.load(Ordering::Relaxed) >= txn_idx {
            self.execution_queue.lock().insert(txn_idx);
        }
    }

    pub(crate) fn remove_from_schedule(&self, txn_idx: TxnIndex) {
        self.execution_queue.lock().remove(&txn_idx);
    }
}

// Testing interfaces for SchedulerProxy.
impl SchedulerProxy {
    #[cfg(test)]
    pub(crate) fn new_for_test(executed_idx: u32) -> Self {
        Self {
            executed_idx: CachePadded::new(AtomicU32::new(executed_idx)),
            execution_queue: Mutex::new(BTreeSet::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn assert_execution_queue(&self, expected_indices: &Vec<TxnIndex>) {
        let queue = self.execution_queue.lock();
        assert_eq!(queue.len(), expected_indices.len());
        for scheduled_idx in expected_indices {
            assert!(queue.contains(scheduled_idx));
        }
    }
}

pub(crate) struct SchedulerV2 {
    /// Number of transactions to execute, and number of workers: immutable.
    num_txns: TxnIndex,
    num_workers: u32,

    /// Execution status of each transaction as far as the scheduler is concerned, as well
    /// as recorded outgoing edges for each transaction along which to propagate the stalls
    /// when the transaction is itself aborted or stalled. These edges correspond to higher
    /// transactions that have aborted in the past due to a write-set change on re-execution.
    txn_status: Vec<CachePadded<ExecutionStatus>>,
    aborted_dependencies: Vec<CachePadded<Mutex<AbortedDependencies>>>,

    /// Shared counter for tracking the next transaction to commit, as well as
    /// a shared flag, set when all txns are committed or execution is halted.
    next_to_commit_idx: CachePadded<AtomicU32>,
    is_done: CachePadded<AtomicBool>,
    /// Tasks queue for post commit tasks with a fixed capacity of number of transactions.
    /// committed_marker is for implementation details: breaking symmetry between two kinds of
    /// commit for index i (direct finish execution of i and try_commit traversal from i-1).
    post_commit_task_queue: CachePadded<ConcurrentQueue<TxnIndex>>,
    committed_marker: Vec<CachePadded<AtomicBool>>,

    proxy: Arc<SchedulerProxy>,
}

impl SchedulerV2 {
    pub(crate) fn new(num_txns: TxnIndex, num_workers: u32) -> Self {
        assert!(num_txns > 0, "No scheduler needed for 0 transactions");
        assert!(num_workers > 0, "Scheduler requires at least 1 worker");

        let proxy = Arc::new(SchedulerProxy::new(num_txns));

        Self {
            num_txns,
            num_workers,
            txn_status: (0..num_txns)
                .map(|txn_idx| CachePadded::new(ExecutionStatus::new(proxy.clone(), txn_idx)))
                .collect(),
            aborted_dependencies: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(AbortedDependencies::new())))
                .collect(),
            next_to_commit_idx: CachePadded::new(AtomicU32::new(0)),
            is_done: CachePadded::new(AtomicBool::new(false)),
            post_commit_task_queue: CachePadded::new(ConcurrentQueue::<TxnIndex>::bounded(
                num_txns as usize,
            )),
            committed_marker: (0..num_txns)
                .map(|_| CachePadded::new(AtomicBool::new(false)))
                .collect(),
            proxy,
        }
    }

    // TODO: take worker ID, dedicate some workers to scan high priority tasks (can use armed lock).
    // We can also have different versions (e.g. for testing) of next_task.
    pub(crate) fn next_task(&self) -> Result<TaskKind, PanicError> {
        if self.is_done() && self.post_commit_task_queue.is_empty() {
            return Ok(TaskKind::Done);
        }

        match self.post_commit_task_queue.pop() {
            Ok(txn_idx) => {
                return Ok(TaskKind::Commit(txn_idx));
            },
            Err(PopError::Empty) => {},
            Err(PopError::Closed) => {
                return Err(code_invariant_error("Commit queue should never be closed"));
            },
        };

        if let Some(txn_idx) = self.proxy.pop_next() {
            if let Some(incarnation) = self.try_start_executing(txn_idx) {
                return Ok(TaskKind::Execute(txn_idx, incarnation));
            }
        }

        Ok(TaskKind::NextTask)
    }

    // Called when a transaction observes a read-write dependency, i.e. reads a value written by
    // another transaction. This API provides a cheap happy-path. It calls resolve dependency on
    // the status w. default dependency instruction, and return Ok(true) if the resolution
    // is SafeToProceed, Ok(false) o.w. (PanicError for invariant violations). Implementation
    // is wait-free, so the caller can simultaneously hold locks on multi-versioned data-structure.
    pub(crate) fn resolve_dependency_happy_path(
        &self,
        txn_idx: TxnIndex,
        dep_txn_idx: TxnIndex,
    ) -> Result<bool, PanicError> {
        if txn_idx <= dep_txn_idx || txn_idx >= self.num_txns {
            return Err(code_invariant_error(format!(
                "In resolve_dependency: txn_idx = {}, dep_txn_idx = {}, num txns = {}",
                txn_idx, dep_txn_idx, self.num_txns,
            )));
        }

        match self.txn_status[dep_txn_idx as usize]
            .resolve_dependency(DependencyInstruction::Default)?
        {
            DependencyResolution::SafeToProceed => Ok(true),
            DependencyResolution::Wait(_) => {
                unreachable!("Wait resolution for Default instruction")
            },
            DependencyResolution::None => Ok(false),
        }
    }

    pub(crate) fn resolve_hint(&self, dep_txn_idx: TxnIndex) -> Result<(), PanicError> {
        if let DependencyResolution::Wait(dep_condition) =
            self.txn_status[dep_txn_idx as usize].resolve_dependency(DependencyInstruction::Wait)?
        {
            Self::wait(dep_condition);
        }
        Ok(())
    }

    // Called when a transaction observes a read-write dependency, i.e. reads a value written by
    // another transaction. resolve_dependency is to be called after resolve_dependency_happy_path
    // returns Ok(false). The caller must not hold locks on multi-versioning data-structure as the
    // implementation can internally wait for the dependency to resolve.
    //
    // Since the caller does not hold any locks, it must repeat the read after resolve_dependency
    // returns Ok(true), re-acquiring the locks in the process. If the result of the read changes,
    // the resolution process must be performed for the new read. O.w., the read record can be made.
    // Ok(false) is a recommendation for the caller to (speculatively) abort the ongoing execution
    // as a performance optimization. This can occur for lower-priority transactions (further
    // from the committed prefix) that have already executed once (hence, generated the first
    // approximation of their output, which is valuable), and are likely to be aborted (having
    // already aborted due to the same dependency).
    pub(crate) fn resolve_dependency(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        dep_txn_idx: TxnIndex,
    ) -> Result<bool, PanicError> {
        loop {
            use ExecutionPriority::*;

            let priority = self.priority(txn_idx)?;
            let abort_likely = matches!(priority, High | Medium | Low).then(|| {
                self.aborted_dependencies[dep_txn_idx as usize]
                    .lock()
                    .stalled_deps
                    .contains(&txn_idx)
            });

            let resolution = match (priority, abort_likely) {
                (Highest, None) | (High, Some(true)) => {
                    // For the highest priority transactions, and high priority transactions that
                    // have previously aborted due to the encountered dependency, we can be more
                    // liberal with waiting: it should be shorter (due to priority), and pipelining
                    // benefits may outweigh the risk of aborts close to the committed prefix.
                    self.txn_status[dep_txn_idx as usize]
                        .resolve_dependency(DependencyInstruction::Wait)?
                },
                (High, Some(false)) => {
                    // For high priority transactions without previous abort history due to the
                    // same dependency, allow waiting only when the dependency is Executing (other
                    // statuses imply that a re-execution, when required, has not even started and
                    // would likely require longer waiting for resolution).
                    self.txn_status[dep_txn_idx as usize]
                        .resolve_dependency(DependencyInstruction::WaitForExecuting)?
                },
                (Low, Some(true)) | (Medium, Some(true)) => {
                    let ret = if matches!(priority, Medium) {
                        self.txn_status[dep_txn_idx as usize]
                            .resolve_dependency(DependencyInstruction::WaitForExecuting)?
                    } else {
                        DependencyResolution::None
                    };

                    // The logic of scheduling first re-executions after all prior transactions
                    // have finished 0-th incarnations is useful if the writes of these 0-th
                    // incarnations are properly recorded and available for reading. Hence, we
                    // disallow speculatively aborting 0-th incarnation. An alternative would be
                    // to only allow increasing scheduler's executed_idx after an execution that
                    // is not speculatively aborted (such as here).
                    if incarnation > 0 && matches!(ret, DependencyResolution::None) {
                        let to_propagate = self.try_abort(txn_idx, incarnation)?;
                        self.propagate(to_propagate)?;

                        // The current logic is to just ask caller to speculatively abort.
                        // TODO: complex handling (e.g. scheduler maintenance), also revisit Wait.
                        return Ok(false);
                    }

                    ret
                },
                (Low, Some(false)) | (Medium, Some(false)) => {
                    // There is value in finishing the speculative execution, as the transaction
                    // has not been previously aborted due to this dependency: proceeding allows
                    // an optimistic approach, and recording the aborted dependency upon failure.
                    DependencyResolution::None
                },
                (High | Medium | Low, None) | (Highest, Some(_)) => {
                    unreachable!("abort_likely checked for high / medium / low priority")
                },
            };

            match resolution {
                DependencyResolution::SafeToProceed | DependencyResolution::None => {
                    return Ok(true);
                },
                DependencyResolution::Wait(dep_condition) => {
                    Self::wait(dep_condition);
                    // Next iteration of the loop will try to resolve again.
                },
            }
        }
    }

    pub(crate) fn finish_execution(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        invalidated_versions: BTreeSet<(TxnIndex, Incarnation)>,
    ) -> Result<(), PanicError> {
        if incarnation > 0 {
            // TODO: make sure we don't kill switch 0-th incarnations.
            // Record aborted dependencies. Only recording for incarnations > 0 is in line with the
            // optimistic value validation principle of Block-STMv2. 0-th incarnation might invalidate
            // due to the first write, but later incarnations could make the same writes - in which case
            // there is no need to record (and stall, etc) the corresponding dependency.
            self.aborted_dependencies[txn_idx as usize]
                .lock()
                .record_dependencies(invalidated_versions.iter().map(|(idx, _)| *idx));
        }

        let mut propagation_queue = BTreeSet::from([txn_idx]);
        for (invalidated_idx, invalidated_incarnation) in invalidated_versions {
            let mut to_propagate = self.try_abort(invalidated_idx, invalidated_incarnation)?;
            propagation_queue.append(&mut to_propagate);
        }
        if self.txn_status[txn_idx as usize].finish_execution(incarnation)? {
            // After updating the status, check if more transactions can be committed. Important to
            // be called after finish execution.
            self.try_commit(txn_idx)?;
        }

        if incarnation == 0 {
            self.try_increase_executed_idx(txn_idx);
        }

        // Handle recursive propagation of stall / unstall.
        self.propagate(propagation_queue)
    }
}

/// Private interfaces
impl SchedulerV2 {
    fn wait(dep_condition: DependencyCondvar) {
        let (lock, cvar) = &*dep_condition;
        let mut dep_resolved = lock.lock();
        while matches!(*dep_resolved, DependencyStatus::Unresolved) {
            dep_resolved = cvar.wait(dep_resolved).unwrap();
        }
    }

    fn propagate(&self, mut propagation_queue: BTreeSet<TxnIndex>) -> Result<(), PanicError> {
        while let Some(task_idx) = propagation_queue.pop_first() {
            // checks the current status to determine whether to propagate 'stall' (or 'unstall'),
            // calling which only affects its currently not_stalled (or stalled) dependencies.
            // Allows to store indices in propagation queue (not stall or unstall commands) & avoids
            // handling corner cases such as merging commands (as propagation process is not atomic).
            if self.txn_status[task_idx as usize].shortcut_executed_and_not_stalled() {
                // Still makes sense to propagate 'unstall'
                self.aborted_dependencies[task_idx as usize]
                    .lock()
                    .unstall(&self.txn_status, &mut propagation_queue)?;
            } else {
                // Not executed or stalled - still makes sense to propagate 'stall'.
                self.aborted_dependencies[task_idx as usize]
                    .lock()
                    .stall(&self.txn_status, &mut propagation_queue)?;
            }
        }
        Ok(())
    }

    fn try_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<BTreeSet<TxnIndex>, PanicError> {
        let mut ret = BTreeSet::new();
        if self.txn_status[txn_idx as usize].try_abort(incarnation)? {
            self.aborted_dependencies[txn_idx as usize]
                .lock()
                .stall(&self.txn_status, &mut ret)?;
        }
        Ok(ret)
    }

    fn try_start_executing(&self, txn_idx: TxnIndex) -> Option<Incarnation> {
        self.txn_status[txn_idx as usize].try_start_executing()
    }

    fn try_commit(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        // Synchronization occurs on the transaction status locks:
        // (a) finish_execution sets the status before try_commit is called for the index,
        // followed by the next_to_commit_idx check, and
        // (b) is_executed call below internally also acquires the lock, and follows the
        // increment to next_to_commit_idx in the prior loop iteration.
        // Hence (by classic flags principle), in case is_executed check fails, the
        // check after finish_execution is guarateed to succeed. This allows relaxed reads
        // on next_to_commit_idx in each try_commit call.
        if self.next_to_commit_idx.load(Ordering::Relaxed) == txn_idx {
            let mut idx = txn_idx;
            while idx < self.num_txns && self.txn_status[idx as usize].is_executed() {
                if self.committed_marker[idx as usize].swap(true, Ordering::Relaxed) {
                    break;
                }

                if self.post_commit_task_queue.push(idx).is_err() {
                    return Err(code_invariant_error(format!(
                        "Error adding {idx} to commit queue, len {}",
                        self.post_commit_task_queue.len()
                    )));
                }

                // Increments cause cache invalidations, but finish_execution / try_commit
                // performs only a relaxed read to check (see above).
                self.next_to_commit_idx.store(idx + 1, Ordering::Relaxed);
                idx += 1;
            }
            if idx == self.num_txns {
                self.is_done.store(true, Ordering::SeqCst);
            }
        }

        Ok(())
    }

    fn try_increase_executed_idx(&self, txn_idx: TxnIndex) {
        // Similar to try_commit, synchronization is provided by the ordering of finish
        // execution updating the transaction inner status (under lock), and the ever_executed
        // check below, which also acquires the lock. In particular, ordering is as follows:
        // (a) finish_execution(idx) with idx lock -> executed_idx == txn_idx check
        // (b) increment executed_idx to txn_idx -> ever_executed check under lock
        // Note that (classic flags principle), in case when ever_executed check fails,
        // executed_idx == txn_idx check is guaranteed to succeed.
        if self.proxy.executed_idx.load(Ordering::Relaxed) == txn_idx {
            let mut idx = txn_idx;
            while idx < self.num_txns && self.txn_status[idx as usize].ever_executed() {
                // TODO: acquire the status lock once instead of twice.
                if self.txn_status[idx as usize].requires_execution() {
                    self.proxy.execution_queue.lock().insert(idx);
                }
                // A successful check of ever_executed holds idx-th status lock and follows an
                // increment of executed_idx to idx in the prior loop iteration. A stall can
                // only remove idx from the execution queue while holding the idx-th status
                // lock, which would have to be after ever_executed, and the corresponding
                // unstall would hence acquire the same lock even later, and hence be guaranteed
                // to observe executed_idx >= idx. TODO: confirm carefully.

                self.proxy.executed_idx.store(idx + 1, Ordering::Relaxed);
                idx += 1;
            }
        }
    }

    // Returns the priority of the transaction based on its proximity with the committed prefix.
    // PanicError is returned if txn_idx is already committed.
    // TODO: adjust the threshold, make priority more fine grained.
    fn priority(&self, txn_idx: TxnIndex) -> Result<ExecutionPriority, PanicError> {
        // TODO: less occasional updates for more cores.
        let next_to_commit_idx = self.next_to_commit_idx.load(Ordering::Relaxed);

        if txn_idx < next_to_commit_idx {
            return Err(code_invariant_error(format!(
                "has_high_priority called for already committed idx {txn_idx}"
            )));
        }

        let interval = self.num_workers >> 2;
        Ok(if txn_idx < next_to_commit_idx + interval {
            ExecutionPriority::Highest
        } else if txn_idx < next_to_commit_idx + 2 * interval {
            ExecutionPriority::High
        } else if txn_idx < next_to_commit_idx + 3 * interval {
            ExecutionPriority::Medium
        } else {
            ExecutionPriority::Low
        })
    }

    fn is_done(&self) -> bool {
        self.is_done.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler_status::InnerStatus;
    use arc_swap::ArcSwapOption;
    use claims::{assert_err, assert_lt, assert_ok, assert_ok_eq, assert_some_eq};
    use dashmap::DashMap;
    use num_cpus;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::{
        cmp::min,
        collections::{btree_map::Entry, BTreeMap},
        thread,
        time::{Duration, Instant},
    };
    use test_case::test_case;

    #[test]
    fn record_aborted_dependencies() {
        let mut deps = AbortedDependencies::new();
        assert!(!deps.is_stalled);
        assert!(deps.not_stalled_deps.is_empty());
        assert!(deps.stalled_deps.is_empty());

        deps.record_dependencies([3, 5, 7].into_iter());
        assert_eq!(deps.not_stalled_deps.len(), 3);
        assert!(deps.stalled_deps.is_empty());
        deps.record_dependencies([3, 6, 7].into_iter());
        assert_eq!(deps.not_stalled_deps.len(), 4);
        assert!(deps.stalled_deps.is_empty());

        deps.stalled_deps.insert(2);
        deps.record_dependencies([1, 2, 3].into_iter());
        assert_eq!(deps.not_stalled_deps.len(), 5);
        assert_eq!(deps.stalled_deps.len(), 1);
    }

    #[test]
    fn stall_aborted_dependencies() {
        let proxy = Arc::new(SchedulerProxy::new(10));
        let mut propagation_queue = BTreeSet::new();

        let err_status = ExecutionStatus::new(proxy.clone(), 0);
        let mut statuses = vec![CachePadded::new(err_status)];
        let mut deps = AbortedDependencies::new();

        assert!(!deps.is_stalled);
        assert_ok!(deps.stall(&statuses, &mut propagation_queue));
        assert!(deps.is_stalled);
        deps.not_stalled_deps.insert(0);
        assert_err!(deps.stall(&statuses, &mut propagation_queue));

        // From now on, ignore status for index 0 (mark as already stalled):
        assert!(deps.stalled_deps.insert(0));
        assert!(deps.not_stalled_deps.remove(&0));
        // Status 1 on which stall returns err, but also ignored (not in not_stalled).
        statuses.push(CachePadded::new(ExecutionStatus::new(proxy.clone(), 1)));

        let status_to_stall =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(1), 0, &proxy, 2);
        statuses.push(CachePadded::new(status_to_stall));
        let executing_status_to_stall =
            ExecutionStatus::new_for_test(InnerStatus::Executing(1), 0, &proxy, 3);
        statuses.push(CachePadded::new(executing_status_to_stall));
        let executed_status_to_stall =
            ExecutionStatus::new_for_test(InnerStatus::Executed(1), 0, &proxy, 4);
        statuses.push(CachePadded::new(executed_status_to_stall));

        let already_stalled_status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(1), 1, &proxy, 5);
        statuses.push(CachePadded::new(already_stalled_status));

        // Successful stall when status requires execution must remove 2 from execution
        // queue, while different status or unsuccessful stall should not.
        let status_len = statuses.len() as u32;
        proxy.execution_queue.lock().clear();
        proxy
            .execution_queue
            .lock()
            .append(&mut (2..status_len).into_iter().collect());
        deps.not_stalled_deps
            .append(&mut (2..status_len).into_iter().collect());
        assert_ok!(deps.stall(&statuses, &mut propagation_queue));

        // Check the results: execution queue, propagation_queue, deps.stalled & not_stalled.
        assert_eq!(proxy.execution_queue.lock().len(), 3);
        for i in 3..6 {
            assert!(proxy.execution_queue.lock().contains(&i));
        }

        assert_eq!(propagation_queue.len(), 3);
        for i in 2..5 {
            assert!(propagation_queue.contains(&i));
        }

        assert_eq!(deps.stalled_deps.len(), 5);
        assert_eq!(deps.not_stalled_deps.len(), 0);
        assert!(deps.stalled_deps.contains(&0)); // pre-inserted
        for i in 2..6 {
            assert!(deps.stalled_deps.contains(&i));
        }
    }

    #[test]
    fn unstall_aborted_dependencies() {
        let proxy = Arc::new(SchedulerProxy::new(10));
        let mut propagation_queue = BTreeSet::new();

        // One err status because of num_stalls = 0, another because of incarnation = 0.
        let err_status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(1), 0, &proxy, 0);
        let err_status_1 =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(0), 1, &proxy, 0);

        let mut statuses = vec![CachePadded::new(err_status)];
        let mut deps = AbortedDependencies::new();
        proxy.executed_idx.store(4, Ordering::Relaxed);

        deps.is_stalled = true;
        assert_ok!(deps.unstall(&statuses, &mut propagation_queue));
        assert!(!deps.is_stalled);
        deps.stalled_deps.insert(0);
        assert_err!(deps.unstall(&statuses, &mut propagation_queue));
        *statuses[0] = err_status_1;
        assert_err!(deps.unstall(&statuses, &mut propagation_queue));

        // From now on, ignore status for index 0 (mark as not_stalled):
        assert!(deps.not_stalled_deps.insert(0));
        assert!(deps.stalled_deps.remove(&0));
        // Status 1 on which stall returns err, but also ignored (not in stalled).
        statuses.push(CachePadded::new(ExecutionStatus::new(proxy.clone(), 1)));

        // All incarnations are 1, but executed_idx >= (2,3,4). Only 4 should be add to execution
        // queue, as 2 and 3 do not require execution. All should be added to propagation queue.
        let executing_status_to_unstall =
            ExecutionStatus::new_for_test(InnerStatus::Executing(1), 1, &proxy, 2);
        statuses.push(CachePadded::new(executing_status_to_unstall));
        let executed_status_to_unstall =
            ExecutionStatus::new_for_test(InnerStatus::Executed(1), 1, &proxy, 3);
        statuses.push(CachePadded::new(executed_status_to_unstall));
        let status_to_unstall =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(1), 1, &proxy, 4);
        statuses.push(CachePadded::new(status_to_unstall));

        // For below statuses, executed_idx < their indices: we test is_first_incarnation behavior.
        statuses.push(CachePadded::new(ExecutionStatus::new_for_test(
            InnerStatus::RequiresExecution(1),
            1,
            &proxy,
            5,
        )));
        statuses.push(CachePadded::new(ExecutionStatus::new_for_test(
            InnerStatus::RequiresExecution(2),
            1,
            &proxy,
            6,
        )));
        // Should not be added to the queues, as num_stalls = 2 (status remains stalled after call).
        let still_stalled_status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(2), 2, &proxy, 7);
        statuses.push(CachePadded::new(still_stalled_status));

        proxy.execution_queue.lock().clear();
        deps.stalled_deps
            .append(&mut (2..statuses.len() as u32).into_iter().collect());
        assert_ok!(deps.unstall(&statuses, &mut propagation_queue,));

        // Check the results: scheduling queue, propagation_queue, deps.stalled & not_stalled.
        assert_eq!(proxy.execution_queue.lock().len(), 2);
        for i in [4, 6].iter() {
            proxy.execution_queue.lock().contains(i);
        }

        assert_eq!(propagation_queue.len(), 5);
        for i in 2..7 {
            propagation_queue.contains(&i);
        }

        assert_eq!(deps.stalled_deps.len(), 0);
        assert_eq!(deps.not_stalled_deps.len(), 7);
        assert!(deps.not_stalled_deps.contains(&0)); // pre-inserted
        for i in 2..8 {
            assert!(deps.not_stalled_deps.contains(&i));
        }
    }

    #[test_case(1)]
    #[test_case(2)]
    #[test_case(4)]
    #[test_case(8)]
    #[test_case(16)]
    #[test_case(32)]
    fn commit_and_executed_idx_simple(num_workers: u32) {
        if num_workers as usize > num_cpus::get() {
            // Ideally, we would want:
            // https://users.rust-lang.org/t/how-to-programatically-ignore-a-unit-test/64096/5
            return;
        }

        let num_txns: u32 = 1000;
        let scheduler = SchedulerV2::new(num_txns, num_workers);
        let txn_execution_idx = AtomicU32::new(0);
        assert!(!scheduler.is_done());

        rayon::scope(|s| {
            for _ in 0..num_workers {
                s.spawn(|_| loop {
                    let idx = txn_execution_idx.fetch_add(1, Ordering::SeqCst);
                    if idx >= num_txns {
                        break;
                    }

                    assert_some_eq!(scheduler.try_start_executing(idx), 0);
                    assert_ok!(scheduler.finish_execution(idx, 0, BTreeSet::new()));
                });
            }
        });

        assert!(scheduler.is_done());
        assert_eq!(
            scheduler.next_to_commit_idx.load(Ordering::Relaxed),
            num_txns
        );
        assert_eq!(
            scheduler.proxy.executed_idx.load(Ordering::Relaxed),
            num_txns
        );
        assert_eq!(scheduler.post_commit_task_queue.len(), num_txns as usize);
        for i in 0..num_txns {
            assert!(scheduler.txn_status[i as usize].is_executed());
            assert!(scheduler.committed_marker[i as usize].load(Ordering::Relaxed));
            assert_ok_eq!(scheduler.post_commit_task_queue.pop(), i);
        }
    }

    #[test_case(1)]
    #[test_case(2)]
    #[test_case(4)]
    #[test_case(8)]
    #[test_case(16)]
    #[test_case(32)]
    // This test generates a DAG of dependencies, where each transaction depends on
    // exactly one prior transaction. When the first incarnation of the transaction
    // is executed, it checks if the dependent transaction has recorded its output.
    // If not, then an abort hook is added to the system, triggered by any subsequent
    // lower-indexed transaction's finish_execution (for testing purposes).
    // The re-execution after abort will perform the same check, and is asserted to
    // succeed due to the algorithm ensuring all prior transactions have finished
    // the first incarnation prior to scheduling. The test utilizes next_task method,
    // and also confirms that the block execution finishes / commits successfully.
    fn dependency_graph_single_reexecution(num_workers: u32) {
        if num_workers as usize > num_cpus::get() {
            // Ideally, we would want:
            // https://users.rust-lang.org/t/how-to-programatically-ignore-a-unit-test/64096/5
            return;
        }

        for seed in 0..5 {
            let num_txns: u32 = 100;
            let scheduler = SchedulerV2::new(num_txns, num_workers);
            let num_committed = AtomicU32::new(0);

            let mut r = StdRng::seed_from_u64(seed);
            let txn_deps: Vec<Option<usize>> = (0..num_txns)
                .map(|idx| {
                    let diff: u32 = r.gen_range(0, min(idx, 16) + 1);
                    // self dependency (diff = 0) works for this test and makes aborts more likely,
                    // and as such, the test stronger.
                    (diff <= idx).then_some((idx - diff) as usize)
                })
                .collect();
            let hooks = Mutex::new(BTreeSet::new());
            let hooks_taken: Vec<_> = (0..num_txns).map(|_| AtomicBool::new(false)).collect();
            let observed_executed: Vec<_> = (0..num_txns).map(|_| AtomicBool::new(false)).collect();

            rayon::scope(|s| {
                for _ in 0..num_workers {
                    s.spawn(|_| loop {
                        match scheduler.next_task().unwrap() {
                            TaskKind::Execute(txn_idx, incarnation) => {
                                assert!(incarnation < 2);

                                hooks_taken[txn_idx as usize].store(true, Ordering::Relaxed);
                                let invalidated: BTreeSet<_> = hooks
                                    .lock()
                                    .split_off(&(txn_idx + 1, 0))
                                    .into_iter()
                                    .collect();

                                if let Some(dep_idx) = txn_deps[txn_idx as usize] {
                                    if !scheduler.txn_status[dep_idx].ever_executed() {
                                        hooks.lock().insert((txn_idx, incarnation));
                                        if hooks_taken[dep_idx].load(Ordering::Relaxed) {
                                            // Hook is not guaraneed to be executed - call abort itself.
                                            assert_ok!(scheduler.try_abort(txn_idx, incarnation));
                                        }

                                        assert_ok!(scheduler.finish_execution(
                                            txn_idx,
                                            incarnation,
                                            invalidated,
                                        ));
                                        continue;
                                    }
                                }

                                assert_ok!(scheduler.finish_execution(
                                    txn_idx,
                                    incarnation,
                                    invalidated,
                                ));
                                observed_executed[txn_idx as usize].store(true, Ordering::Relaxed);
                            },
                            TaskKind::Commit(_) => {
                                num_committed.fetch_add(1, Ordering::Relaxed);
                            },
                            TaskKind::NextTask => {},
                            TaskKind::Done => {
                                break;
                            },
                        }
                    });
                }
            });

            assert_eq!(num_committed.load(Ordering::Relaxed), num_txns);
            for i in 0..num_txns {
                assert!(
                    observed_executed[i as usize].load(Ordering::Relaxed),
                    "Transaction {i} did not observe executed dependency status"
                );

                assert!(scheduler.txn_status[i as usize].is_executed());
                assert!(scheduler.committed_marker[i as usize].load(Ordering::Relaxed));
            }
        }
    }

    // If a key is not initialized when reading in below experiments, return the following value.
    const STORAGE_VALUE: u32 = u32::MAX;

    struct MockWrite {
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        registered_reads: BTreeSet<(TxnIndex, Incarnation)>,
    }

    #[test_case(10, 1000, 4, 10, false, 100)]
    // #[test_case(10, 1000, 4, 10, true, 100)]
    // #[test_case(1, 1000, 10, 40, false, 200)]
    // #[test_case(2, 1000, 10, 40, false, 200)]
    // #[test_case(4, 1000, 10, 40, false, 200)]
    // #[test_case(8, 1000, 10, 40, false, 200)]
    // #[test_case(16, 1000, 10, 40, false, 200)]
    // #[test_case(32, 1000, 10, 40, false, 200)]
    // #[test_case(48, 1000, 10, 40, false, 200)]
    // #[test_case(64, 1000, 10, 40, false, 200)]
    // #[test_case(96, 1000, 10, 40, false, 200)]
    // #[test_case(128, 1000, 10, 40, false, 200)]
    // #[test_case(1, 1000, 10, 50, false, 0)]
    // #[test_case(2, 1000, 10, 50, false, 30)]
    // #[test_case(2, 1000, 10, 50, false, 300)]
    // #[test_case(2, 1000, 10, 50, false, 2000)]
    // #[test_case(4, 1000, 10, 50, false, 30)]
    // #[test_case(4, 1000, 10, 50, false, 300)]
    // #[test_case(4, 1000, 10, 50, false, 2000)]
    // #[test_case(8, 1000, 10, 50, false, 30)]
    // #[test_case(8, 1000, 10, 50, false, 300)]
    // #[test_case(8, 1000, 10, 50, false, 2000)]
    // Executes the 'barrier' workload on the scheduler: there are num_keys many keys in the system.
    // Starting from 0-th transaction, each barrier_interval-th transaction writes to all keys, all
    // other transactions write to a single random key. All transactions read from a single random
    // key. For simplicity, read & write keys are fixed for all incarnations of the same transaction.
    // Moreover, every incarnation writes the same value - the index of the transaction. Correctness
    // is asserted by comparing the parallel execution output to sequential baseline.
    // resolve_dependency parameter controls whether each read is followed by a resolve_dependency
    // call, allowing us to measure the impact of this feature (e.g. waiting for estimates). Each
    // execution is delayed by mock_execution_time_us (in microseconds).
    fn barrier_workload(
        num_workers: u32,
        num_txns: u32,
        num_keys: usize,
        barrier_interval: u32,
        resolve_dependency: bool,
        mock_execution_time_us: u64,
    ) {
        let num_cpus = num_cpus::get();
        if num_workers as usize > num_cpus {
            println!("Number of specified workers {num_workers} > number of cpus {num_cpus}");
            // Ideally, we would want:
            // https://users.rust-lang.org/t/how-to-programatically-ignore-a-unit-test/64096/5
            return;
        }

        let mut times_us: Vec<usize> = vec![];
        for seed in 0..7 {
            let mut r = StdRng::seed_from_u64(seed);
            let txn_write_keys: Vec<Vec<usize>> = (0..num_txns)
                .map(|idx| {
                    if idx % barrier_interval == 0 {
                        (0..num_keys).into_iter().collect()
                    } else {
                        vec![r.gen_range(0, num_keys)]
                    }
                })
                .collect();
            let txn_read_keys: Vec<usize> =
                (0..num_txns).map(|_| r.gen_range(0, num_keys)).collect();

            // Compute the expected (baseline) read results.
            let mut storage_state: BTreeMap<usize, TxnIndex> =
                (0..num_keys).map(|key| (key, STORAGE_VALUE)).collect();
            let baseline_reads: Vec<TxnIndex> = (0..num_txns)
                .map(|baseline_idx| {
                    let ret = storage_state[&txn_read_keys[baseline_idx as usize]];

                    for write_key in &txn_write_keys[baseline_idx as usize] {
                        storage_state.insert(*write_key, baseline_idx);
                    }

                    ret
                })
                .collect();

            let scheduler = SchedulerV2::new(num_txns, num_workers);
            let mv_hashmap: DashMap<
                usize,
                // .0 is registered reads for storage value, .1 is versioned entries.
                (
                    BTreeSet<(TxnIndex, Incarnation)>,
                    BTreeMap<TxnIndex, CachePadded<MockWrite>>,
                ),
            > = (0..num_keys)
                .map(|key| (key, (BTreeSet::new(), BTreeMap::new())))
                .collect();
            let latest_reads: Vec<CachePadded<ArcSwapOption<TxnIndex>>> = (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect();

            let start_time = Instant::now();

            rayon::scope(|s| {
                for _ in 0..num_workers {
                    s.spawn(|_| loop {
                        match scheduler.next_task().unwrap() {
                            TaskKind::Execute(txn_idx, incarnation) => {
                                if mock_execution_time_us > 0 {
                                    thread::sleep(Duration::from_micros(mock_execution_time_us));
                                }

                                let dep_txn_idx = {
                                    // Perform and record a read.
                                    let mut key_entries = mv_hashmap
                                        .get_mut(&txn_read_keys[txn_idx as usize])
                                        .unwrap();
                                    let mut iter = key_entries.1.range_mut(0..txn_idx);

                                    let read = match iter.next_back() {
                                        Some((write_idx, entry)) => {
                                            assert_eq!(*write_idx, entry.txn_idx);
                                            entry.registered_reads.insert((txn_idx, incarnation));
                                            *write_idx
                                        },
                                        None => {
                                            key_entries.0.insert((txn_idx, incarnation));
                                            STORAGE_VALUE
                                        },
                                    };
                                    latest_reads[txn_idx as usize].store(Some(Arc::new(read)));
                                    read
                                };

                                if resolve_dependency && dep_txn_idx != STORAGE_VALUE {
                                    // TODO: if Ok(false) is returned (i.e. speculative abort is
                                    // requested), it now always occurs after mock_execution_time_us
                                    // delay, but in reality it could save part of execution time.
                                    scheduler
                                        .resolve_dependency(txn_idx, incarnation, dep_txn_idx)
                                        .unwrap();
                                }

                                // We could record writes only for incarnation 0:
                                // - In the workload, the writes of all incarnations are the same.
                                // - No registered reads would be invalidated by incarnations > 0
                                //   due to value-based validation.
                                // - Incarnation 0 should always record the writes: we don't check
                                //   here, but resolve_dependency currently does not instruct to
                                //   speculatively abort the 0-th incarnation.
                                // However, in order to measure scalability more fairly, we query
                                // the DashMap and check that the written value by a prior incarnatio
                                // is the same, and record writes in any case.
                                let invalidated_versions: BTreeSet<(TxnIndex, Incarnation)> = {
                                    let mut ret = BTreeSet::new();

                                    for write in &txn_write_keys[txn_idx as usize] {
                                        let mut key_entries = mv_hashmap.get_mut(write).unwrap();

                                        // Write should consider range including itself.
                                        let mut iter = key_entries.1.range_mut(0..=txn_idx);
                                        let mut cur_invalidated = match iter.next_back() {
                                            Some((write_idx, entry)) => {
                                                if incarnation == 0 {
                                                    assert_lt!(*write_idx, txn_idx);
                                                    // No reads can be transferred since write_idx
                                                    // differs from txn_idx (and indices are the values).
                                                    // Split at txn_idx + 1 as own reads are unaffected.
                                                    entry
                                                        .registered_reads
                                                        .split_off(&(txn_idx + 1, 0))
                                                } else {
                                                    assert_eq!(*write_idx, txn_idx);
                                                    assert_eq!(entry.txn_idx, txn_idx);
                                                    assert_eq!(entry.incarnation + 1, incarnation);
                                                    BTreeSet::new()
                                                }
                                            },
                                            None => key_entries.0.split_off(&(txn_idx + 1, 0)),
                                        };
                                        ret.append(&mut cur_invalidated);

                                        if let Entry::Vacant(vacant_entry) =
                                            key_entries.1.entry(txn_idx)
                                        {
                                            assert_eq!(incarnation, 0);
                                            vacant_entry.insert(CachePadded::new(MockWrite {
                                                txn_idx,
                                                incarnation,
                                                registered_reads: BTreeSet::new(),
                                            }));
                                        }
                                    }

                                    ret
                                };

                                assert_ok!(scheduler.finish_execution(
                                    txn_idx,
                                    incarnation,
                                    invalidated_versions,
                                ));
                            },
                            TaskKind::Commit(txn_idx) => {
                                // On commit, perform a read, and assert it is equal to the
                                // latest recorded read, as well as baseline.
                                let key_entries =
                                    mv_hashmap.get(&txn_read_keys[txn_idx as usize]).unwrap();
                                let mut iter = key_entries.1.range(0..txn_idx);
                                let commit_read = match iter.next_back() {
                                    Some((write_idx, entry)) => {
                                        assert_eq!(*write_idx, entry.txn_idx);
                                        *write_idx
                                    },
                                    None => STORAGE_VALUE,
                                };

                                assert_eq!(
                                    commit_read,
                                    *latest_reads[txn_idx as usize]
                                        .load_full()
                                        .expect("Latest read must be recorded"),
                                    "txn_idx = {txn_idx}",
                                );
                                assert_eq!(
                                    commit_read, baseline_reads[txn_idx as usize],
                                    "txn_idx = {txn_idx}"
                                );
                            },
                            TaskKind::NextTask => {},
                            TaskKind::Done => {
                                break;
                            },
                        }
                    });
                }
            });

            let execution_time = start_time.elapsed().as_micros();
            times_us.push(execution_time.try_into().unwrap());

            // basic checks on scheduler state.
            for i in 0..num_txns {
                assert!(scheduler.txn_status[i as usize].is_executed());
                assert!(scheduler.committed_marker[i as usize].load(Ordering::Relaxed));
            }
        }

        // Times reported in order of measurements. We can ignore e.g. first two as warm-ups.
        println!(
            "Barrier workload V2 execution Summary:\n    num_workers {num_workers}\n    \
	     num_txns {num_txns}\n    num_keys {num_keys}\n    barrier_interval \
	     {barrier_interval}\n    execution time {mock_execution_time_us}\n    \
	     resolve_dependency {resolve_dependency}\ntimes in microseconds: {:?}\n",
            times_us
        );
    }
}
