// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scheduler::{ArmedLock, DependencyCondvar, DependencyStatus},
    scheduler_status::{DependencyInstruction, DependencyResolution, ExecutionStatus},
};
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use concurrent_queue::{ConcurrentQueue, PopError};
use crossbeam::utils::CachePadded;
use fail::fail_point;
use std::{
    cell::RefCell,
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap, BTreeSet,
    },
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering},
        Arc,
    },
};

/**
A transaction may be (re-)executed multiple times, each time with an incremented
incarnation number. Each (txn_idx, incarnation) pair defines a full version, each
represented by a node in an abstract graph that the scheduler maintains. The prior
incarnation aborting is a precondition for the next incarnation to exist.

PanicError returned from APIs indicates scheduler internal invariant failure.

TODO: proper documentation.
**/

// Execution priority determined based on the proximity to the committed prefix.
#[derive(Clone, Copy)]
enum ExecutionPriority {
    Highest,
    High,
    Medium,
    Low,
}

// Non-Sync struct designed for a worker executing a particular transaction / incarnation
// to manage the push-invalidations caused by its output (writes). It dispatches try_abort
// calls, maintains the appropriate state (based on the outcomes) with interior mutability,
// and is provided by value to finish_execution call to enforce correct usage pattern.
pub(crate) struct AbortManager<'a> {
    owner_txn_idx: TxnIndex,
    owner_incarnation: Incarnation,
    scheduler: &'a SchedulerV2,
    // Transaction index in the map implies a write by (owner_txn_idx, owner_incarnation)
    // invalidated a read by the said transaction. If the incarnation is stored in the
    // entry, then try_abort call was successful, implying a promise to call finish_abort.
    invalidations: RefCell<BTreeMap<TxnIndex, Option<Incarnation>>>,
}

impl<'a> AbortManager<'a> {
    pub(crate) fn new(
        owner_txn_idx: TxnIndex,
        owner_incarnation: Incarnation,
        scheduler: &'a SchedulerV2,
    ) -> Self {
        Self {
            owner_txn_idx,
            owner_incarnation,
            scheduler,
            invalidations: RefCell::new(BTreeMap::new()),
        }
    }

    pub(crate) fn invalidate_dependencies(
        &self,
        dependencies: BTreeSet<(TxnIndex, Incarnation)>,
    ) -> Result<(), PanicError> {
        for (txn_idx, incarnation) in dependencies {
            self.invalidate(txn_idx, incarnation)?;
        }
        Ok(())
    }

    fn invalidate(
        &self,
        invalidated_txn_idx: TxnIndex,
        invalidated_incarnation: Incarnation,
    ) -> Result<(), PanicError> {
        if invalidated_txn_idx <= self.owner_txn_idx {
            return Err(code_invariant_error(format!(
                "Execution of version ({}, {}) may not invalidate lower version ({}, {})",
                self.owner_txn_idx,
                self.owner_incarnation,
                invalidated_txn_idx,
                invalidated_incarnation,
            )));
        }

        let mut invalidations = self.invalidations.borrow_mut();
        match invalidations.entry(invalidated_txn_idx) {
            Vacant(vacant_entry) => {
                // For vacant entries, we always need to try abort
                let _ = vacant_entry
                    .insert(self.try_abort(invalidated_txn_idx, invalidated_incarnation)?);
            },
            Occupied(mut occupied_entry) => {
                match occupied_entry.get() {
                    None => {
                        // Only try abort if we don't have a stored incarnation
                        *occupied_entry.get_mut() =
                            self.try_abort(invalidated_txn_idx, invalidated_incarnation)?;
                    },
                    Some(stored_incarnation) => {
                        if *stored_incarnation < invalidated_incarnation {
                            // The caller would have to perform finish_execution for the stored incarnation with a
                            // successful try_abort in order for a higher incarnation to exist (and be invalidated).
                            return Err(code_invariant_error(format!(
                                "Lower incarnation {} than {} has already been invalidated by Abort Manager for txn {}",
                                stored_incarnation, invalidated_incarnation, self.owner_txn_idx
                            )));
                        }
                    },
                }
            },
        }

        Ok(())
    }

    fn try_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<Option<TxnIndex>, PanicError> {
        fail_point!("abort-manager-try-abort-none", |_| Ok(None));
        fail_point!("abort-manager-try-abort-some", |_| Ok(Some(incarnation)));
        Ok(self
            .scheduler
            .try_abort(txn_idx, incarnation)?
            .then(|| incarnation))
    }

    // Returns an iterator over invalidated transaction indices, as well as full versions,
    // i.e. (txn_idx, incarnation) pairs, for which try_abort was successful. For those
    // versions, the finish abort still needs to be performed.
    fn take(
        self,
    ) -> (
        TxnIndex,
        Incarnation,
        BTreeMap<TxnIndex, Option<Incarnation>>,
    ) {
        (
            self.owner_txn_idx,
            self.owner_incarnation,
            self.invalidations.take(),
        )
    }
}

// Describes downstream dependencies for a transaction that have previously gotten aborted
// due to reading the transaction's output (that changed). Since such dependencies might be
// detected in the system concurrently to stalls being added and removed, this structure
// also tracks which dependencies it has added stalls to (for later removal). The invariant
// maintained by the implementation is that the intersection of stalled and not_stalled
// is always empty.
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

    // Calls add_stall on the status and adds all indices from not_stalled to stalled.
    // Inserts indices for which add_stall returned true into the propagation queue.
    fn add_stall(
        &mut self,
        statuses: &[CachePadded<ExecutionStatus>],
        propagation_queue: &mut BTreeSet<usize>,
    ) -> Result<(), PanicError> {
        for idx in &self.not_stalled_deps {
            // Assert the invariant in tests.
            test_assert!(!self.stalled_deps.contains(&idx));

            if statuses[*idx as usize].add_stall()? {
                // May require recursive add_stalls.
                propagation_queue.insert(*idx as usize);
            }
        }

        self.stalled_deps.append(&mut self.not_stalled_deps);
        self.is_stalled = true;
        Ok(())
    }

    // Calls remove_stall on the status and adds all indices from stalled to not_stalled.
    // Inserts indices for which remove_stall returned true into the propagation queue.
    // Additionally if status requires execution, index is added to the scheduling queue.
    fn remove_stall(
        &mut self,
        statuses: &[CachePadded<ExecutionStatus>],
        propagation_queue: &mut BTreeSet<usize>,
    ) -> Result<(), PanicError> {
        for idx in &self.stalled_deps {
            // Assert the invariant in tests.
            test_assert!(!self.not_stalled_deps.contains(&idx));

            if statuses[*idx as usize].remove_stall()? {
                // May require recursive remove_stalls.
                propagation_queue.insert(*idx as usize);
            }
        }

        self.not_stalled_deps.append(&mut self.stalled_deps);
        self.is_stalled = false;
        Ok(())
    }
}

// Returned from next task interface, representing instruction to the executor using SchedulerV2.
#[derive(PartialEq, Debug)]
pub(crate) enum TaskKind {
    // Execute transaction with a given index and incarnation (0-indexed).
    Execute(TxnIndex, Incarnation),
    // Execute any post-commit processing, if applicable. Dispatched to the executor exactly once
    // per transaction index after the transaction is committed and the corresponding sequential
    // commit hook is completed (from scheduler pov assumed to be embarassingly parallelizable).
    PostCommitProcessing(TxnIndex),
    // A task was not readily available. Can be viewed as a None variant if Option<TaskKind> was
    // returned. Gives the control back to the caller of next task / executor, even though the
    // most common use might be to call next_task again in a loop.
    NextTask,
    // Scheduling is complete, the executor can terminate the worker.
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

// Const flag values that may be stored as committed marker for a transaction.
const NOT_COMMITTED: u8 = 0;
const PENDING_COMMIT_HOOK: u8 = 1;
const COMMITTED: u8 = 2;

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
    is_halted: CachePadded<AtomicBool>,
    /// Tasks queue for post commit tasks with a fixed capacity of number of transactions.
    queueing_commits_lock: CachePadded<ArmedLock>,
    post_commit_processing_queue: CachePadded<ConcurrentQueue<TxnIndex>>,
    committed_marker: Vec<CachePadded<AtomicU8>>,

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
            is_halted: CachePadded::new(AtomicBool::new(false)),
            queueing_commits_lock: CachePadded::new(ArmedLock::new()),
            post_commit_processing_queue: CachePadded::new(ConcurrentQueue::<TxnIndex>::bounded(
                num_txns as usize,
            )),
            committed_marker: (0..num_txns)
                .map(|_| CachePadded::new(AtomicU8::new(NOT_COMMITTED)))
                .collect(),
            proxy,
        }
    }

    // Marks the transaction as (fully) committed, i.e. indicates that the caller (executor) has
    // successfully performed the sequential (clint-side) commit hook logic. The transaction index
    // must have previously been obtained via try_sequential_commit_hook method call, while
    // holding the lock (commit_hooks_try_lock / commit_hooks_unlock).
    //
    // Also schedules the corresponding task for parallel post-processing of the transaction.
    pub(crate) fn commit_hook_performed(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        // Allows next sequential commit hook to be processed.
        if self.committed_marker[txn_idx as usize].swap(COMMITTED, Ordering::Relaxed)
            != PENDING_COMMIT_HOOK
        {
            return Err(code_invariant_error(format!(
                "Marking txn {} as COMMITTED, but previous marker != PENDING_COMMIT_HOOK",
                txn_idx
            )));
        }

        if self.post_commit_processing_queue.push(txn_idx).is_err() {
            return Err(code_invariant_error(format!(
                "Error adding {txn_idx} to commit queue, len {}",
                self.post_commit_processing_queue.len()
            )));
        }

        Ok(())
    }

    pub(crate) fn commit_hooks_unlock(&self) {
        let next_to_commit_idx = self.next_to_commit_idx.load(Ordering::Relaxed);
        if next_to_commit_idx < self.num_txns
            && !self.is_halted()
            && self.txn_status[next_to_commit_idx as usize].is_executed()
        {
            self.queueing_commits_lock.arm();
        }

        self.queueing_commits_lock.unlock();
    }

    pub(crate) fn commit_hooks_try_lock(&self) -> bool {
        self.queueing_commits_lock.try_lock()
    }

    // Should be called (i.e. in a while loop until None) after a successful should_perform_commit_hooks
    // call. Completing the hooks should be followed by a commit_hook_performed call.
    pub(crate) fn try_get_sequential_commit_hook(
        &self,
    ) -> Result<Option<(TxnIndex, Incarnation)>, PanicError> {
        // Relaxed ordering due to armed lock acq-rel.
        let next_to_commit_idx = self.next_to_commit_idx.load(Ordering::Relaxed) as usize;

        assert!(next_to_commit_idx <= self.num_txns as usize);
        if next_to_commit_idx > 0 {
            // Since the commit hooks lock is held by caller during this method and while performing
            // the hook itself, the marker here should be 'COMMITTED'. NOT_COMMITTED means the previous
            // call to try_get_sequential_commit_hook that must have increased the index did not
            // set the status, while PENDING_COMMIT_HOOK means the caller never successfully followed
            // the hook by the commit_hook_peformed call (should only happen in error scenarios).
            let prev_committed_marker =
                self.committed_marker[next_to_commit_idx - 1].load(Ordering::Relaxed);
            if prev_committed_marker != COMMITTED {
                return Err(code_invariant_error(format!(
                    "Trying to get commit hook for {}, but previous index marker {} != 2 (COMMITTED)",
                    next_to_commit_idx, prev_committed_marker,
                )));
            };
        }

        if self.is_halted() || next_to_commit_idx == self.num_txns as usize {
            // All sequential commit hooks are already dispatched.
            return Ok(None);
        }

        if self.txn_status[next_to_commit_idx].is_executed() {
            // All prior transactions are committed and the latest incarnation of the transaction
            // at next_to_commit_idx has finished but has not been aborted. If any of its read was
            // incorrect, it would have been invalidated by the respective transaction's last
            // (committed) (re-)execution, and led to an abort in the corresponding finish execution
            // (which, inductively, must occur before the transaction is committed). Hence, it
            // must also be safe to commit the current transaction.

            if self.committed_marker[next_to_commit_idx as usize]
                .swap(PENDING_COMMIT_HOOK, Ordering::Relaxed)
                != NOT_COMMITTED
            {
                return Err(code_invariant_error(format!(
                    "Marking {} as PENDING_COMMIT_HOOK, but previous marker != NOT_COMMITTED",
                    self.post_commit_processing_queue.len()
                )));
            }

            // Increments cause cache invalidations, but finish_execution / try_commit
            // performs only a relaxed read to check (see above).
            let prev_idx = self
                .next_to_commit_idx
                .swap(next_to_commit_idx as u32 + 1, Ordering::Relaxed);
            if prev_idx != next_to_commit_idx as u32 {
                return Err(code_invariant_error(format!(
                    "Scheduler committing {}, stored next to commit idx = {}",
                    next_to_commit_idx, prev_idx
                )));
            }

            return Ok(Some((
                next_to_commit_idx as u32,
                self.txn_status[next_to_commit_idx].last_incarnation(),
            )));
        }

        Ok(None)
    }

    pub(crate) fn post_commit_processing_queue_is_empty(&self) -> bool {
        self.post_commit_processing_queue.is_empty()
    }

    // TODO: take worker ID, dedicate some workers to scan high priority tasks (can use armed lock).
    // We can also have different versions (e.g. for testing) of next_task.
    pub(crate) fn next_task(&self) -> Result<TaskKind, PanicError> {
        if self.is_done() {
            return Ok(TaskKind::Done);
        }

        match self.post_commit_processing_queue.pop() {
            Ok(txn_idx) => {
                if txn_idx == self.num_txns - 1 {
                    self.is_done.store(true, Ordering::SeqCst);
                }
                return Ok(TaskKind::PostCommitProcessing(txn_idx));
            },
            Err(PopError::Empty) => {
                if self.is_halted() {
                    return Ok(TaskKind::Done);
                }
            },
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

    pub(crate) fn halt(&self) -> bool {
        if !self.is_halted.swap(true, Ordering::SeqCst) {
            for txn_idx in 0..self.num_txns {
                self.txn_status[txn_idx as usize].halt();
            }
            return true;
        }
        false
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

        use DependencyResolution::*;
        match self.txn_status[dep_txn_idx as usize]
            .resolve_dependency(DependencyInstruction::Default)?
        {
            SafeToProceed => Ok(true),
            Wait(_) => {
                unreachable!("Wait resolution for Default instruction")
            },
            None | Halted => Ok(false),
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
                        // Change the inner status to aborted so finishing execution will make the
                        // transaction eligible for re-execution. Status was Executing so we keep
                        // things simple and ignore aborted dependencies / stall propagation.
                        if self.txn_status[txn_idx as usize].try_abort(incarnation)? {
                            self.txn_status[txn_idx as usize].finish_abort(incarnation)?;
                        }

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
                DependencyResolution::Halted => {
                    return Ok(false);
                },
            }
        }
    }

    // Abort manager that the worker / txn execution used to process all invalidations in BlockSTMv2 (while
    // applying its own output) is provided by value as an argument (to enforce the proper usage pattern).
    pub(crate) fn finish_execution<'a>(
        &'a self,
        abort_manager: AbortManager<'a>,
    ) -> Result<(), PanicError> {
        let (txn_idx, incarnation, invalidated_set) = abort_manager.take();

        if incarnation > 0 {
            // Record aborted dependencies. Only recording for incarnations > 0 is in line with the
            // optimistic value validation principle of Block-STMv2. 0-th incarnation might invalidate
            // due to the first write, but later incarnations could make the same writes - in which case
            // there is no need to record (and stall, etc) the corresponding dependency.
            self.aborted_dependencies[txn_idx as usize]
                .lock()
                .record_dependencies(invalidated_set.iter().map(|(txn_idx, _)| *txn_idx));
        }

        let mut propagation_queue: BTreeSet<usize> = BTreeSet::new();
        for (txn_idx, maybe_incarnation) in invalidated_set {
            if let Some(incarnation) = maybe_incarnation {
                self.txn_status[txn_idx as usize].finish_abort(incarnation)?;
                propagation_queue.insert(txn_idx as usize);
            }
        }

        if self.txn_status[txn_idx as usize].finish_execution(incarnation)? {
            propagation_queue.insert(txn_idx as usize);

            if txn_idx == 0
                || self.committed_marker[txn_idx as usize - 1].load(Ordering::Relaxed)
                    != NOT_COMMITTED
            {
                // If the committed marker is NOT_COMMITTED by the time the last execution of a
                // transaction finishes, then considering the lowest such index, arming will occur
                // either because txn_idx = 0 (base case), or after the marker is set, in the
                // commits_hooks_unlock method (which checks the executed status).
                self.queueing_commits_lock.arm();
            }
        }

        if incarnation == 0 {
            self.try_increase_executed_idx(txn_idx);
        }

        // Handle recursive propagation of add / remove stall.
        self.propagate(propagation_queue)?;

        Ok(())
    }

    pub(crate) fn is_halted_or_aborted(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        if self.is_halted() {
            return true;
        }

        if incarnation == 0 {
            // Never interrupt the 0-th incarnation due to an early abort to get the first output
            // estimation (even if it is based on invalidated reads).
            return false;
        }

        self.txn_status[txn_idx as usize].already_try_aborted(incarnation)
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

    fn propagate(&self, mut propagation_queue: BTreeSet<usize>) -> Result<(), PanicError> {
        while let Some(task_idx) = propagation_queue.pop_first() {
            // Make sure the conditions are checked under dependency lock.
            let mut aborted_deps_guard = self.aborted_dependencies[task_idx].lock();

            // checks the current status to determine whether to propagate add / remove stall,
            // calling which only affects its currently not_stalled (or stalled) dependencies.
            // Allows to store indices in propagation queue (not add or remove commands) & avoids
            // handling corner cases such as merging commands (as propagation process is not atomic).
            if self.txn_status[task_idx].shortcut_executed_and_not_stalled() {
                // Still makes sense to propagate remove_stall.
                aborted_deps_guard.remove_stall(&self.txn_status, &mut propagation_queue)?;
            } else {
                // Not executed or stalled - still makes sense to propagate add_stall.
                aborted_deps_guard.add_stall(&self.txn_status, &mut propagation_queue)?;
            }
        }
        Ok(())
    }

    fn try_abort(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> Result<bool, PanicError> {
        self.txn_status[txn_idx as usize].try_abort(incarnation)
    }

    fn try_start_executing(&self, txn_idx: TxnIndex) -> Option<Incarnation> {
        self.txn_status[txn_idx as usize].try_start_executing()
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
                // A successful check of ever_executed holds idx-th status lock and follows an
                // increment of executed_idx to idx in the prior loop iteration. Adding stall
                // can only remove idx from the execution queue while holding the idx-th status
                // lock, which would have to be after ever_executed, and the corresponding
                // remove_stall would hence acquire the same lock even later, and hence be
                // guaranteed to observe executed_idx >= idx. TODO: confirm carefully.
                self.proxy.executed_idx.store(idx + 1, Ordering::Relaxed);

                // Note: Should we keep the lock from ever_executed instead of re-acquiring.
                if self.txn_status[idx as usize].pending_scheduling_and_not_stalled() {
                    self.proxy.execution_queue.lock().insert(idx);
                }

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

    fn is_halted(&self) -> bool {
        self.is_halted.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler_status::{InnerStatus, StatusEnum};
    use arc_swap::ArcSwapOption;
    use claims::{assert_err, assert_lt, assert_none, assert_ok, assert_ok_eq, assert_some_eq};
    use dashmap::DashMap;
    use fail::FailScenario;
    use num_cpus;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::{
        cmp::min,
        thread,
        time::{Duration, Instant},
    };
    use test_case::test_case;

    // Helper function to invalidate all transactions after a given index
    fn invalidate_after_index(
        abort_manager: &AbortManager,
        invalidated_set: &mut BTreeSet<(TxnIndex, Incarnation)>,
        after_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        for (invalidated_txn_idx, invalidated_incarnation) in
            invalidated_set.split_off(&(after_idx + 1, 0))
        {
            abort_manager.invalidate(invalidated_txn_idx, invalidated_incarnation)?;
        }
        Ok(())
    }

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
        assert_ok!(deps.add_stall(&statuses, &mut propagation_queue));
        assert!(deps.is_stalled);
        deps.not_stalled_deps.insert(0);
        assert_err!(deps.add_stall(&statuses, &mut propagation_queue));

        // From now on, ignore status for index 0 (mark as already stalled):
        assert!(deps.stalled_deps.insert(0));
        assert!(deps.not_stalled_deps.remove(&0));
        // Status 1 on which stall returns err, but also ignored (not in not_stalled).
        statuses.push(CachePadded::new(ExecutionStatus::new(proxy.clone(), 1)));

        let status_to_stall = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            0,
            &proxy,
            2,
        );
        statuses.push(CachePadded::new(status_to_stall));
        let executing_status_to_stall = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executing, 1),
            0,
            &proxy,
            3,
        );
        statuses.push(CachePadded::new(executing_status_to_stall));
        let executed_status_to_stall = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executed, 1),
            0,
            &proxy,
            4,
        );
        statuses.push(CachePadded::new(executed_status_to_stall));

        let already_stalled_status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            1,
            &proxy,
            5,
        );
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
        assert_ok!(deps.add_stall(&statuses, &mut propagation_queue));

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
    fn remove_stall_aborted_dependencies() {
        let proxy = Arc::new(SchedulerProxy::new(10));
        let mut propagation_queue = BTreeSet::new();

        // One err status because of num_stalls = 0, another because of incarnation = 0.
        let err_status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            0,
            &proxy,
            0,
        );
        let err_status_1 = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 0),
            1,
            &proxy,
            0,
        );

        let mut statuses = vec![CachePadded::new(err_status)];
        let mut deps = AbortedDependencies::new();
        proxy.executed_idx.store(4, Ordering::Relaxed);

        deps.is_stalled = true;
        assert_ok!(deps.remove_stall(&statuses, &mut propagation_queue));
        assert!(!deps.is_stalled);
        deps.stalled_deps.insert(0);
        assert_err!(deps.remove_stall(&statuses, &mut propagation_queue));
        *statuses[0] = err_status_1;
        assert_err!(deps.remove_stall(&statuses, &mut propagation_queue));

        // From now on, ignore status for index 0 (mark as not_stalled):
        assert!(deps.not_stalled_deps.insert(0));
        assert!(deps.stalled_deps.remove(&0));
        // Status 1 on which stall returns err, but also ignored (not in stalled).
        statuses.push(CachePadded::new(ExecutionStatus::new(proxy.clone(), 1)));

        // All incarnations are 1, but executed_idx >= (2,3,4). Only 4 should be add to execution
        // queue, as 2 and 3 do not require execution. All should be added to propagation queue.
        let executing_status_to_remove_stall = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executing, 1),
            1,
            &proxy,
            2,
        );
        statuses.push(CachePadded::new(executing_status_to_remove_stall));
        let executed_status_to_remove_stall = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executed, 1),
            1,
            &proxy,
            3,
        );
        statuses.push(CachePadded::new(executed_status_to_remove_stall));
        let status_to_remove_stall = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            1,
            &proxy,
            4,
        );
        statuses.push(CachePadded::new(status_to_remove_stall));

        // For below statuses, executed_idx < their indices: we test is_first_incarnation behavior.
        statuses.push(CachePadded::new(ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            1,
            &proxy,
            5,
        )));
        statuses.push(CachePadded::new(ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 2),
            1,
            &proxy,
            6,
        )));
        // Should not be added to the queues, as num_stalls = 2 (status remains stalled after call).
        let still_stalled_status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 2),
            2,
            &proxy,
            7,
        );
        statuses.push(CachePadded::new(still_stalled_status));

        proxy.execution_queue.lock().clear();
        deps.stalled_deps
            .append(&mut (2..statuses.len() as u32).into_iter().collect());
        assert_ok!(deps.remove_stall(&statuses, &mut propagation_queue,));

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

    fn propagate() {
        let scheduler = SchedulerV2::new(10, 2);

        let test_indices = [0, 2, 4];
        for idx in test_indices {
            assert!(!scheduler.aborted_dependencies[idx].lock().is_stalled);
            assert!(!scheduler.txn_status[idx].is_stalled());
        }
        scheduler
            .propagate(BTreeSet::from(test_indices.clone()))
            .unwrap();
        for idx in test_indices {
            assert!(scheduler.aborted_dependencies[idx].lock().is_stalled);
            // Propagate does not call stall for the status itself, only
            // propagates to aborted dependencies based on the status (assumption
            // being the status is already updated, e.g. due to propagation).
            assert!(!scheduler.txn_status[0].is_stalled());
        }

        scheduler.aborted_dependencies[0].lock().is_stalled = false;

        // Add 4 as dependency of 2 and get its stall removed.
        scheduler.aborted_dependencies[2]
            .lock()
            .stalled_deps
            .insert(4);
        assert_some_eq!(scheduler.try_start_executing(2), 0);
        assert_some_eq!(scheduler.try_start_executing(4), 0);
        assert_ok!(scheduler.txn_status[2].finish_execution(0));
        assert_ok!(scheduler.txn_status[4].finish_execution(0));
        // Propagate starts at 2 (does not call remove stall), but will call remove on 4.
        assert_ok_eq!(scheduler.txn_status[4].add_stall(), true);
        assert!(scheduler.txn_status[4].is_stalled());
        scheduler.propagate(BTreeSet::from([2])).unwrap();
        assert!(!scheduler.aborted_dependencies[2].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[4].lock().is_stalled);
        assert!(!scheduler.txn_status[4].is_stalled());
    }

    fn stall_and_add_dependency(
        scheduler: &SchedulerV2,
        idx: TxnIndex,
        dep_idx: TxnIndex,
        num_stalls: usize,
    ) {
        assert!(num_stalls > 0);

        assert_some_eq!(scheduler.try_start_executing(dep_idx), 0);
        assert_ok!(scheduler.finish_execution(AbortManager::new(dep_idx, 0, scheduler)));
        assert_ok_eq!(scheduler.txn_status[dep_idx as usize].add_stall(), true);
        assert!(scheduler.txn_status[dep_idx as usize].is_stalled());
        for _ in 1..num_stalls {
            assert_ok_eq!(scheduler.txn_status[dep_idx as usize].add_stall(), false);
        }
        assert!(scheduler.txn_status[dep_idx as usize].is_stalled());

        scheduler.aborted_dependencies[dep_idx as usize]
            .lock()
            .is_stalled = true;
        scheduler.aborted_dependencies[idx as usize]
            .lock()
            .stalled_deps
            .insert(dep_idx);
    }

    #[test]
    fn finish_execution_remove_stall() {
        let scheduler = SchedulerV2::new(10, 2);
        assert_some_eq!(scheduler.try_start_executing(0), 0);

        scheduler.aborted_dependencies[0].lock().is_stalled = true;
        stall_and_add_dependency(&scheduler, 0, 2, 1);
        stall_and_add_dependency(&scheduler, 0, 3, 2);

        assert_ok!(scheduler.finish_execution(AbortManager::new(0, 0, &scheduler)));

        assert!(!scheduler.aborted_dependencies[0].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[2].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[3].lock().is_stalled);
        assert!(!scheduler.txn_status[0].is_stalled());
        assert!(!scheduler.txn_status[2].is_stalled());
        assert!(scheduler.txn_status[3].is_stalled());
        assert_ok_eq!(scheduler.txn_status[3].remove_stall(), true);

        for i in 0..3 {
            assert_eq!(
                scheduler.committed_marker[i].load(Ordering::Relaxed),
                NOT_COMMITTED
            );
        }
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_abort_manager_invalidate() {
        let scheduler = SchedulerV2::new(10, 1);
        let abort_manager = AbortManager::new(2, 0, &scheduler);

        // Check initial state - no invalidations should be recorded
        assert!(abort_manager.invalidations.borrow().is_empty());

        let scenario = FailScenario::setup();
        assert!(fail::has_failpoints());

        // Test invalidating lower version (error), try_abort (not called) via failpoint.
        fail::cfg("abort-manager-try-abort-none", "panic").unwrap();
        assert_err!(abort_manager.invalidate(1, 0));
        assert_err!(abort_manager.invalidate(2, 0)); // same version
        assert_err!(abort_manager.invalidate(0, 0));

        // Test case where try_abort returns None (simulating false)
        fail::cfg("abort-manager-try-abort-none", "return").unwrap();
        assert_ok!(abort_manager.invalidate(3, 0));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&3), &None);
        fail::remove("abort-manager-try-abort-none");
        // Make sure None can get replaced with an incarnation.
        fail::cfg("abort-manager-try-abort-some", "return").unwrap();
        assert_ok!(abort_manager.invalidate(3, 2));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&3), &Some(2));

        // Test case where try_abort returns Some(incarnation).
        assert_ok!(abort_manager.invalidate(4, 0));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&4), &Some(0));

        // Test occupied entry with Some value - error if lower incarnation try_aborted.
        fail::cfg("abort-manager-try-abort-some", "panic").unwrap();
        assert_err!(abort_manager.invalidate(4, 1));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&4), &Some(0));

        // Test invalidating with equal incarnation as stored - should be ignored.
        // Configure failpoint to panic but it shouldn't be called since incarnation matches.
        assert_ok!(abort_manager.invalidate(4, 0));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&4), &Some(0));
        fail::remove("abort-manager-try-abort");

        // Test multiple invalidations for different transactions
        fail::cfg("abort-manager-try-abort-some", "return").unwrap();
        assert_ok!(abort_manager.invalidate(5, 2));
        assert_ok!(abort_manager.invalidate(6, 4));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&5), &Some(2));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&6), &Some(4));
        fail::remove("abort-manager-try-aborts-some");

        // Test that invalidations are preserved after multiple calls (in different order),
        // and that lower incarnations are ignored.
        fail::cfg("abort-manager-try-abort-some", "panic").unwrap();
        assert_ok!(abort_manager.invalidate(5, 1));
        assert_err!(abort_manager.invalidate(5, 4));
        assert_err!(abort_manager.invalidate(6, 6));
        assert_ok!(abort_manager.invalidate(6, 1));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&5), &Some(2));
        assert_some_eq!(abort_manager.invalidations.borrow().get(&6), &Some(4));

        scenario.teardown();
    }

    #[test]
    fn test_abort_manager_take() {
        let scheduler = SchedulerV2::new(10, 1);
        let abort_manager = AbortManager::new(2, 0, &scheduler);

        // Set up failpoint before running test
        let scenario = FailScenario::setup();
        assert!(fail::has_failpoints());

        // Record some invalidations with specific failpoint configurations.
        fail::cfg("abort-manager-try-abort-none", "return").unwrap();
        assert_ok!(abort_manager.invalidate(3, 0));
        fail::remove("abort-manager-try-abort-none");
        fail::cfg("abort-manager-try-abort-some", "return").unwrap();
        assert_ok!(abort_manager.invalidate(4, 1));
        assert_ok!(abort_manager.invalidate(5, 2));

        // Verify the invalidations before taking them.
        let invalidations = abort_manager.invalidations.borrow();
        assert_eq!(invalidations.len(), 3);
        assert_some_eq!(invalidations.get(&3), &None);
        assert_some_eq!(invalidations.get(&4), &Some(1));
        assert_some_eq!(invalidations.get(&5), &Some(2));
        drop(invalidations);

        // Take the invalidations and verify the returned values.
        let (owner_txn_idx, owner_incarnation, invalidations) = abort_manager.take();

        assert_eq!(owner_txn_idx, 2);
        assert_eq!(owner_incarnation, 0);
        assert_eq!(invalidations.len(), 3);
        assert_some_eq!(invalidations.get(&3), &None);
        assert_some_eq!(invalidations.get(&4), &Some(1));
        assert_some_eq!(invalidations.get(&5), &Some(2));

        scenario.teardown();
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
                    assert_ok!(scheduler.finish_execution(AbortManager::new(idx, 0, &scheduler)));
                });
            }
        });

        assert!(!scheduler.is_done());
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 0);
        assert_eq!(
            scheduler.proxy.executed_idx.load(Ordering::Relaxed),
            num_txns
        );
        assert_eq!(scheduler.post_commit_processing_queue.len(), 0);

        for i in 0..num_txns {
            assert!(scheduler.txn_status[i as usize].is_executed());

            assert_eq!(
                scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                NOT_COMMITTED
            );
            assert_err!(scheduler.commit_hook_performed(i));
            // Reset it back, as the call w. error swaps the status.
            scheduler.committed_marker[i as usize].store(NOT_COMMITTED, Ordering::Relaxed);

            assert_some_eq!(scheduler.try_get_sequential_commit_hook().unwrap(), (i, 0));
            assert_eq!(
                scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                PENDING_COMMIT_HOOK
            );

            // Commit hook needs to complete for next one to be dispatched.
            assert_err!(scheduler.try_get_sequential_commit_hook());
            assert_ok!(scheduler.commit_hook_performed(i));
            assert_eq!(
                scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                COMMITTED
            );

            assert_err!(scheduler.commit_hook_performed(i));
            // Reset again.
            scheduler.committed_marker[i as usize].store(COMMITTED, Ordering::Relaxed);
        }

        assert_eq!(
            scheduler.post_commit_processing_queue.len(),
            num_txns as usize
        );

        assert!(scheduler.txn_status[0].is_executed());
        assert_ok_eq!(scheduler.post_commit_processing_queue.pop(), 0);

        for i in 1..num_txns {
            assert!(!scheduler.is_done());
            assert_ok_eq!(scheduler.next_task(), TaskKind::PostCommitProcessing(i));
        }
        assert!(scheduler.is_done());
    }

    #[test]
    fn remove_stall_propagation_scenario() {
        let mut scheduler = SchedulerV2::new(10, 1);
        scheduler.txn_status[3] = CachePadded::new(ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executed, 5),
            0,
            &scheduler.proxy,
            2,
        ));
        stall_and_add_dependency(&scheduler, 3, 5, 1);
        stall_and_add_dependency(&scheduler, 5, 7, 1);
        stall_and_add_dependency(&scheduler, 3, 8, 2);
        stall_and_add_dependency(&scheduler, 3, 6, 1);
        stall_and_add_dependency(&scheduler, 6, 9, 1);
        assert_ok_eq!(scheduler.txn_status[6].try_abort(0), true);
        assert_ok!(scheduler.txn_status[6].finish_abort(0));

        assert_ok!(scheduler.propagate(BTreeSet::from([3])));

        assert!(!scheduler.txn_status[3].is_stalled());
        assert!(!scheduler.txn_status[5].is_stalled());
        assert!(!scheduler.txn_status[7].is_stalled());
        assert!(scheduler.txn_status[8].is_stalled());
        assert!(!scheduler.txn_status[6].is_stalled());
        assert!(scheduler.txn_status[9].is_stalled());
        assert!(!scheduler.aborted_dependencies[0].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[5].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[7].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[8].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[6].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[9].lock().is_stalled);
    }

    #[test]
    fn try_get_sequential_commit_hook_simple() {
        let mut scheduler = SchedulerV2::new(10, 1);

        // Test txn index 0.
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 0);
        assert_none!(scheduler.try_get_sequential_commit_hook().unwrap());
        // Next task should start executing (0, 0).
        assert_ok_eq!(scheduler.next_task(), TaskKind::Execute(0, 0));
        assert_none!(scheduler.try_get_sequential_commit_hook().unwrap());
        // After execution is finished, commit hook can be dispatched.
        assert_ok!(scheduler.finish_execution(AbortManager::new(0, 0, &scheduler)));
        assert_eq!(
            scheduler.committed_marker[0].load(Ordering::Relaxed),
            NOT_COMMITTED
        );

        assert_some_eq!(scheduler.try_get_sequential_commit_hook().unwrap(), (0, 0));
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 1);
        assert_eq!(
            scheduler.committed_marker[0].load(Ordering::Relaxed),
            PENDING_COMMIT_HOOK
        );

        assert_err!(scheduler.try_get_sequential_commit_hook());
        scheduler.next_to_commit_idx.store(3, Ordering::Relaxed);
        assert_err!(scheduler.try_get_sequential_commit_hook());
        scheduler.committed_marker[2].store(COMMITTED, Ordering::Relaxed);
        assert_none!(scheduler.try_get_sequential_commit_hook().unwrap());

        scheduler.txn_status[3] = CachePadded::new(ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executed, 5),
            1,
            &scheduler.proxy,
            2,
        ));
        assert!(scheduler.txn_status[3].is_stalled());
        assert_eq!(
            scheduler.committed_marker[3].load(Ordering::Relaxed),
            NOT_COMMITTED
        );
        // Should commit despite being currently stalled.
        assert_some_eq!(scheduler.try_get_sequential_commit_hook().unwrap(), (3, 5));

        scheduler.next_to_commit_idx.store(10, Ordering::Relaxed);
        scheduler.committed_marker[9].store(COMMITTED, Ordering::Relaxed);
        assert_none!(scheduler.try_get_sequential_commit_hook().unwrap());
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
    //
    // The test is internally executed multiple times, with a different seed. Some seeds
    // halts the scheduler before all transactions are committed.
    fn dependency_graph_single_reexecution(num_workers: u32) {
        if num_workers as usize > num_cpus::get() {
            // Ideally, we would want:
            // https://users.rust-lang.org/t/how-to-programatically-ignore-a-unit-test/64096/5
            return;
        }

        // TODO: Seed divisible by 8 is reserved as a basic halting test.
        for seed in 0..10000 {
            let num_txns: u32 = if seed & 7 == 0 { 1000 } else { 100 };
            let scheduler = SchedulerV2::new(num_txns, num_workers);
            let num_committed = AtomicU32::new(0);
            let num_processed = AtomicU32::new(0);

            let mut r = StdRng::seed_from_u64(seed);
            let txn_deps: Vec<Option<usize>> = (0..num_txns)
                .map(|idx| {
                    let diff: u32 = r.gen_range(0, min(idx, 16) + 1);
                    (diff > 0 && diff <= idx).then_some((idx - diff) as usize)
                })
                .collect();
            let hooks = Mutex::new(BTreeSet::new());
            let hooks_taken: Vec<_> = (0..num_txns).map(|_| AtomicBool::new(false)).collect();
            let observed_executed: Vec<_> = (0..num_txns).map(|_| AtomicBool::new(false)).collect();

            rayon::scope(|s| {
                for _ in 0..num_workers {
                    s.spawn(|_| loop {
                        while scheduler.commit_hooks_try_lock() {
                            while let Some((txn_idx, incarnation)) =
                                scheduler.try_get_sequential_commit_hook().unwrap()
                            {
                                assert!(incarnation < 2);
                                assert!(scheduler.txn_status[txn_idx as usize].is_executed());

                                assert_eq!(
                                    scheduler.committed_marker[txn_idx as usize]
                                        .load(Ordering::Relaxed),
                                    PENDING_COMMIT_HOOK
                                );
                                scheduler.commit_hook_performed(txn_idx).unwrap();

                                if num_committed.fetch_add(1, Ordering::Relaxed) == num_txns / 2
                                    && seed & 7 == 0
                                {
                                    // Halt must occur after commit_hook_performed call (so we
                                    // do not miss a post-processing task in next_task).
                                    scheduler.halt();
                                }
                                assert_eq!(
                                    scheduler.committed_marker[txn_idx as usize]
                                        .load(Ordering::Relaxed),
                                    COMMITTED
                                );
                            }

                            scheduler.commit_hooks_unlock();
                        }

                        match scheduler.next_task().unwrap() {
                            TaskKind::Execute(txn_idx, incarnation) => {
                                assert!(incarnation < 2);

                                hooks_taken[txn_idx as usize].store(true, Ordering::Relaxed);
                                let abort_manager =
                                    AbortManager::new(txn_idx, incarnation, &scheduler);

                                // Invalidate all hooks after this transaction
                                invalidate_after_index(&abort_manager, &mut hooks.lock(), txn_idx)
                                    .unwrap();

                                let mut dep_ok = true;
                                if let Some(dep_idx) = txn_deps[txn_idx as usize] {
                                    if !scheduler.txn_status[dep_idx].ever_executed() {
                                        hooks.lock().insert((txn_idx, incarnation));
                                        if hooks_taken[dep_idx].load(Ordering::Relaxed) {
                                            // Hook is not guaraneed to be executed - call abort itself.
                                            if assert_ok!(scheduler.try_abort(txn_idx, incarnation))
                                            {
                                                assert_ok!(scheduler.txn_status[txn_idx as usize]
                                                    .finish_abort(incarnation));
                                            }
                                        };
                                        dep_ok = false;
                                    }
                                }

                                if dep_ok {
                                    observed_executed[txn_idx as usize]
                                        .store(true, Ordering::Relaxed);
                                }
                                assert_ok!(scheduler.finish_execution(abort_manager));
                            },
                            TaskKind::PostCommitProcessing(txn_idx) => {
                                num_processed.fetch_add(1, Ordering::Relaxed);

                                assert_eq!(
                                    scheduler.committed_marker[txn_idx as usize]
                                        .load(Ordering::Relaxed),
                                    COMMITTED
                                );
                                assert!(scheduler.txn_status[txn_idx as usize].is_executed());
                            },
                            TaskKind::NextTask => {},
                            TaskKind::Done => {
                                break;
                            },
                        }
                    });
                }
            });

            let num_committed = num_committed.load(Ordering::Relaxed);
            assert_eq!(num_committed, num_processed.load(Ordering::Relaxed));
            assert_eq!(
                num_committed,
                if seed & 7 != 0 {
                    num_txns
                } else {
                    num_txns / 2 + 1
                }
            );
            for i in 0..num_committed {
                assert!(
                    observed_executed[i as usize].load(Ordering::Relaxed),
                    "Transaction {i} did not observe executed dependency status"
                );

                assert!(scheduler.txn_status[i as usize].is_executed());
                assert_eq!(
                    scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                    COMMITTED
                );
                // Eventually removing stalls should propagate to all dependencies.
                assert!(!scheduler.aborted_dependencies[i as usize].lock().is_stalled);
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
                        while scheduler.commit_hooks_try_lock() {
                            while let Some((txn_idx, _)) =
                                scheduler.try_get_sequential_commit_hook().unwrap()
                            {
                                scheduler.commit_hook_performed(txn_idx).unwrap();
                            }
                            scheduler.commit_hooks_unlock();
                        }

                        match scheduler.next_task().unwrap() {
                            TaskKind::Execute(txn_idx, incarnation) => {
                                if mock_execution_time_us > 0 {
                                    // TODO: emulate interrupt requested (invalidated).
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

                                let abort_manager =
                                    AbortManager::new(txn_idx, incarnation, &scheduler);

                                // Record writes and invalidate affected reads
                                for write in &txn_write_keys[txn_idx as usize] {
                                    let mut key_entries = mv_hashmap.get_mut(write).unwrap();

                                    // Write should consider range including itself.
                                    let mut iter = key_entries.1.range_mut(0..=txn_idx);
                                    match iter.next_back() {
                                        Some((write_idx, entry)) => {
                                            if incarnation == 0 {
                                                assert_lt!(*write_idx, txn_idx);
                                                // No reads can be transferred since write_idx
                                                // differs from txn_idx (and indices are the values).
                                                // Split at txn_idx + 1 as own reads are unaffected.
                                                invalidate_after_index(
                                                    &abort_manager,
                                                    &mut entry.registered_reads,
                                                    txn_idx,
                                                )
                                                .unwrap();
                                            } else {
                                                assert_eq!(*write_idx, txn_idx);
                                                assert_eq!(entry.txn_idx, txn_idx);
                                                assert_eq!(entry.incarnation + 1, incarnation);
                                            }
                                        },
                                        None => {
                                            // Invalidate all reads after this transaction
                                            invalidate_after_index(
                                                &abort_manager,
                                                &mut key_entries.0,
                                                txn_idx,
                                            )
                                            .unwrap();
                                        },
                                    }

                                    if let Vacant(vacant_entry) = key_entries.1.entry(txn_idx) {
                                        assert_eq!(incarnation, 0);
                                        vacant_entry.insert(CachePadded::new(MockWrite {
                                            txn_idx,
                                            incarnation,
                                            registered_reads: BTreeSet::new(),
                                        }));
                                    }
                                }

                                assert_ok!(scheduler.finish_execution(abort_manager));
                            },
                            TaskKind::PostCommitProcessing(txn_idx) => {
                                assert_eq!(
                                    scheduler.committed_marker[txn_idx as usize]
                                        .load(Ordering::Relaxed),
                                    COMMITTED
                                );
                                assert!(scheduler.txn_status[txn_idx as usize].is_executed());

                                // Past commit, perform a read, and assert it is equal to the
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
                assert_eq!(
                    scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                    COMMITTED
                );
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
