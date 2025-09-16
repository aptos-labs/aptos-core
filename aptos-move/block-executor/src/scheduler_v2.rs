// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cold_validation::{ColdValidationRequirements, ValidationRequirement},
    scheduler::ArmedLock,
    scheduler_status::ExecutionStatuses,
};
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use concurrent_queue::{ConcurrentQueue, PopError};
use crossbeam::utils::CachePadded;
use fail::fail_point;
use move_core_types::language_storage::ModuleId;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering},
};

/**
================================ BlockSTMv2 Scheduler (SchedulerV2) ================================

This module implements [SchedulerV2], the core component of the BlockSTMv2 execution engine.
[SchedulerV2] orchestrates the parallel execution of transactions within a block, managing
their lifecycle from initial scheduling through execution, potential aborts and re-executions,
and eventual commit and post-commit processing.

Key Responsibilities:
---------------------
1.  **Task Management**: [SchedulerV2] provides tasks to worker threads. These tasks can be
    to execute a transaction, or to perform post-commit processing for a committed transaction.
    It uses a [TaskKind] enum to represent these different types of tasks.

2.  **Transaction Lifecycle Coordination**: It interacts closely with `ExecutionStatuses` (from
    `scheduler_status.rs`) to track and update the state of each transaction (e.g.,
    `PendingScheduling`, `Executing`, `Executed`, `Aborted`).

3.  **Concurrency Control & Dependency Management**:
    -   **Abort Handling**: It utilizes an [AbortManager] to handle invalidations. When a
        transaction's execution output might affect other transactions that read its prior
        state, those dependent transactions are aborted and rescheduled.
    -   **Stall Propagation**: It manages [AbortedDependencies] to track which transactions
        have been previously aborted due to changes in a dependency. This information is
        used to implement a "stall" mechanism. If a transaction T_i is stalled (e.g.,
        because a lower-indexed transaction T_j it depends on was aborted and T_j might
        abort T_i again), T_i's re-execution might be deferred. Stalls propagate through
        the dependency graph that the scheduler automatically builds and maintains.

4.  **Commit Sequencing**: [SchedulerV2] ensures that transactions are committed in their
    original sequence (by transaction index). It manages a `next_to_commit_idx` and uses
    `CommitMarkerFlag`s to track the commit state of each transaction (NotCommitted,
    CommitStarted, Committed). A lock (`queueing_commits_lock`) is used to serialize
    the dispatching of sequential commit hooks. Once the lock is acquired,
    [SchedulerV2::start_commit] and [SchedulerV2::end_commit] are called to
    start and end the commit process for a transaction (which includes the caller
    specified sequential commit hooks / logic).

5.  **Execution Flow Control**:
    -   **`executed_once_max_idx`**: Tracks the highest transaction index up to which all
        transactions have been executed at least once. This is used to intelligently
        defer the first re-execution of a transaction until all preceding transactions
        have produced their initial speculative writes.
    -   **`min_never_scheduled_idx`**: An optimization that tracks the minimum transaction
        index that has not yet been scheduled. This is currently used to identify maximum
        range of the interval that may require a traversal for module read validation
        (after a committed txn that publishes a module), but can be generally useful for
        tracking the evolution of the "active" interval of the scheduler.
        TODO(BlockSTMv2): consider constraining the interval to have a maximum size, for
        optimizing performance as well as for integration w. execution pooling, etc.

6.  **Halt and Completion**: Provides mechanisms to halt ongoing execution prematurely and
    to determine when all transactions in the block are fully processed and committed.

Interaction with Other Components:
---------------------------------
-   **`ExecutionStatuses`**: The source of truth for the status (scheduling state, incarnation,
    stall count) of individual transactions. [SchedulerV2] queries and updates these statuses.
-   **[ExecutionQueueManager]**: Embedded within `ExecutionStatuses`, it manages the actual
    queue of transactions ready for execution. [SchedulerV2] interacts with it to pop
    transactions for execution and to add transactions back (e.g., upon re-scheduling).
-   **[AbortManager]**: Used by worker threads during `[SchedulerV2::finish_execution]`. When a worker
    completes a transaction, it uses the [AbortManager] to identify and initiate aborts for
    dependent transactions that read its (now potentially changed) output. [SchedulerV2]
    consumes the [AbortManager] to finalize these aborts and update dependencies.
-   **Worker Threads**: Continuously request tasks from [SchedulerV2] via [SchedulerV2::next_task].
    They execute these tasks and report results (e.g., via [SchedulerV2::finish_execution],
    [SchedulerV2::end_commit]).

Conceptual Execution Model:
--------------------------
Workers request tasks. [SchedulerV2] prioritizes:
1.  **Post-Commit Processing Tasks**: If there are transactions that have been committed and
    are awaiting their parallel post-commit logic, these are dispatched first. This provides
    more parallelism and ensures that committed work is finalized promptly.
2.  **Execution Tasks**: If no post-commit tasks are pending, [SchedulerV2] attempts to pop
    a transaction from the [ExecutionQueueManager]. If successful, it transitions the
    transaction's state to `Executing` and returns an `Execute` task to the worker.
3.  **Control Tasks**: If no work is immediately available, `NextTask` is returned, signaling
    the worker to try again. If all work is done or the scheduler is halted, `Done` is returned.

The design aims to maximize parallelism while ensuring correctness through careful management
of dependencies, aborts, and commit sequencing. The stall mechanism is a best-effort
heuristic to reduce wasted work from cascading aborts.
**/

/// Manages push-invalidations caused by a transaction's output and handles aborting dependent transactions.
///
/// [AbortManager] is a non-Sync struct designed for a worker executing a particular transaction
/// (the "owner transaction", identified by `owner_txn_idx` and `owner_incarnation`).
/// When the owner transaction finishes execution, its writes might necessitate the re-execution
/// of other transactions that read the same data locations. [AbortManager] is used by the
/// worker thread responsible for the owner transaction to:
///
/// 1.  Identify such dependent transactions.
/// 2.  Attempt to initiate their abort by calling [SchedulerV2::start_abort] on the scheduler.
/// 3.  Track the outcome of these abort attempts within its `invalidations` map.
///
/// After a transaction's execution attempt is processed by the scheduler, the [AbortManager] instance
/// is transferred by value (moving ownership) to the [SchedulerV2::finish_execution] function.
/// This transfer enforces a clear ownership model and ensures that the [AbortManager]'s state
/// is correctly consumed and finalized.
pub(crate) struct AbortManager<'a> {
    owner_txn_idx: TxnIndex,
    owner_incarnation: Incarnation,
    scheduler: &'a SchedulerV2,
    // Transaction index in the map implies a write by (owner_txn_idx, owner_incarnation)
    // invalidated a read by the said transaction. If the incarnation is stored in the
    // entry, then start_abort call was successful, implying a promise to call finish_abort.
    invalidated_dependencies: BTreeMap<TxnIndex, Option<Incarnation>>,
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
            invalidated_dependencies: BTreeMap::new(),
        }
    }

    pub(crate) fn invalidate_dependencies(
        &mut self,
        dependencies: BTreeMap<TxnIndex, Incarnation>,
    ) -> Result<(), PanicError> {
        // Might want to consider iterating over incarnations in reverse order to ensure
        // that invalidate method implementation can avoid outdated try_abort calls.
        for (txn_idx, incarnation) in dependencies {
            self.invalidate(txn_idx, incarnation)?;
        }
        Ok(())
    }

    fn invalidate(
        &mut self,
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

        // Decide whether to proceed with an abort attempt based on the current state of
        // `self.invalidated_dependencies`. Separated in two steps (as computing action_needed,
        // then performing the action) in order to satisfy the borrowing rules.
        let action_needed = match self.invalidated_dependencies.get(&invalidated_txn_idx) {
            None => {
                // An abort attempt is needed as there's no prior record for this `invalidated_txn_idx`.
                true
            },
            Some(None) => {
                // This means an abort was previously attempted for `invalidated_txn_idx` by this
                // AbortManager instance, but that attempt was not successful (e.g., `start_abort`
                // returned None because the transaction incarnation was already aborted).
                // We try again with the current `invalidated_incarnation`.
                true
            },
            Some(Some(stored_successful_abort_incarnation)) => {
                // An abort was previously successful for `stored_successful_abort_incarnation`.
                if *stored_successful_abort_incarnation < invalidated_incarnation {
                    // A previous invalidation targeted an older incarnation of `invalidated_txn_idx`
                    // which was successfully aborted and recorded.
                    // Now, a newer incarnation of the same `invalidated_txn_idx` is being targeted.
                    // This is an error: `SchedulerV2::finish_execution` must consume the AbortManager
                    // instance for the `stored_successful_abort_incarnation` before an attempt to
                    // abort a higher incarnation of the same `invalidated_txn_idx` can be made.
                    return Err(code_invariant_error(format!(
                        "Lower incarnation {} than {} already invalidated by Abort Manager for txn version ({}, {})",
                        *stored_successful_abort_incarnation, invalidated_incarnation,
                        self.owner_txn_idx, self.owner_incarnation
                    )));
                }
                // If *stored_incarnation >= invalidated_incarnation, it means either the same
                // or a newer incarnation (compared to the current invalidation) has already been
                // successfully aborted by this AbortManager instance. This can happen because
                // the reads from outdated incarnations are not assumed to be (eagerly) cleared.
                // In such cases, no new abort action is needed for this specific call. Note also
                // that an incarnation can register multiple reads that may later be invalidated.
                false
            },
        };

        if action_needed {
            let abort_outcome: Option<Incarnation> =
                self.start_abort(invalidated_txn_idx, invalidated_incarnation)?;

            // Update self.invalidations with the outcome. This will either insert a new entry
            // or update an existing one from None to Some(outcome).
            self.invalidated_dependencies
                .insert(invalidated_txn_idx, abort_outcome);
        }

        Ok(())
    }

    /// Attempts to initiate an abort for the given transaction and incarnation via the scheduler.
    ///
    /// Returns:
    /// - `Ok(Some(incarnation))` if [SchedulerV2::start_abort] was successful for `incarnation`.
    /// - `Ok(None)` if [SchedulerV2::start_abort] returned false (e.g., already aborted).
    /// - `Err(PanicError)` if [SchedulerV2::start_abort] itself returns an error.
    fn start_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<Option<Incarnation>, PanicError> {
        fail_point!("abort-manager-start-abort-none", |_| Ok(None));
        fail_point!("abort-manager-start-abort-some", |_| Ok(Some(incarnation)));
        Ok(self
            .scheduler
            .start_abort(txn_idx, incarnation)?
            .then_some(incarnation))
    }

    // For invalidated dependencies that are mapped to an incarnation, [SchedulerV2::start_abort]
    // was successful, and the [SchedulerV2::finish_abort] still needs to be performed.
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
            self.invalidated_dependencies,
        )
    }
}

/// Tracks downstream transactions previously aborted by an owner and manages stall propagation.
///
/// When an owner transaction T_owner (re-)executes and its write set changes, it might cause
/// other transactions (T_dep) that read T_owner's output to be aborted. This struct,
/// associated with T_owner, keeps a record of such T_dep transactions.
///
/// It also tracks which of these dependencies it has actively propagated stalls to (for later
/// removal) since such dependencies might be detected concurrently to stalls being added/removed
/// elsewhere. The primary purpose is to manage these "stalls". If T_owner itself is aborted or
/// stalled, it's likely that its previously aborted dependencies (T_dep) will also need to be
/// re-aborted if they re-execute. To prevent wasted work, a stall can be propagated from T_owner
/// to these T_dep transactions.
///
/// This struct distinguishes between dependencies for which a stall has been actively
/// propagated (`stalled_deps`) and those for which it has not (`not_stalled_deps`).
/// The `is_stalled` flag indicates whether the owner transaction itself is considered stalled
/// from the perspective of this [AbortedDependencies] instance, which then dictates whether
/// to propagate `add_stall` or `remove_stall` to its dependencies.
///
/// An invariant is maintained: `stalled_deps` and `not_stalled_deps` must always be disjoint.
struct AbortedDependencies {
    is_stalled: bool,
    not_stalled_deps: BTreeSet<TxnIndex>,
    stalled_deps: BTreeSet<TxnIndex>,
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
        statuses: &ExecutionStatuses,
        stall_propagation_queue: &mut BTreeSet<usize>,
    ) -> Result<(), PanicError> {
        for idx in &self.not_stalled_deps {
            // Assert the invariant in tests.
            #[cfg(test)]
            assert!(!self.stalled_deps.contains(idx));

            if statuses.add_stall(*idx)? {
                // May require recursive add_stalls.
                stall_propagation_queue.insert(*idx as usize);
            }
        }

        self.stalled_deps.append(&mut self.not_stalled_deps);
        self.is_stalled = true;
        Ok(())
    }

    // Calls [ExecutionStatuses::remove_stall] on the status and adds all indices from
    // stalled to not_stalled. Inserts indices for which remove_stall returned true into
    // the stall propagation queue. If such status is pending scheduling, ExecutionStatuses
    // uses execution queue manager to add the transaction to execution queue.
    fn remove_stall(
        &mut self,
        statuses: &ExecutionStatuses,
        stall_propagation_queue: &mut BTreeSet<usize>,
    ) -> Result<(), PanicError> {
        for idx in &self.stalled_deps {
            // Assert the invariant in tests.
            #[cfg(test)]
            assert!(!self.not_stalled_deps.contains(idx));

            if statuses.remove_stall(*idx)? {
                // May require recursive remove_stalls.
                stall_propagation_queue.insert(*idx as usize);
            }
        }

        self.not_stalled_deps.append(&mut self.stalled_deps);
        self.is_stalled = false;
        Ok(())
    }
}

/// Manages the execution queue for the BlockSTMv2 scheduler, tracks scheduling progress,
/// and exposes proxy interfaces to certain implementation details (e.g., scheduler statuses).
///
/// This component is responsible for maintaining an ordered set of transaction indices
/// that are ready to be processed by worker threads. It also tracks overall progress
/// related to transaction scheduling and initial execution phases.
pub(crate) struct ExecutionQueueManager {
    /// Tracks the highest transaction index `i` such that all transactions `0..i`
    /// have completed their first incarnation (i.e., executed at least once).
    /// This is crucial for BlockSTMv2's optimization where the first re-execution of
    /// a transaction `j` is deferred until `executed_once_max_idx >= j`. This ensures
    /// that `j` re-executes with the benefit of the initial speculative writes from all
    /// preceding transactions.
    executed_once_max_idx: CachePadded<AtomicU32>,
    /// Stores the minimum transaction index that has not yet been popped from the
    /// `execution_queue`. This serves as an upper bound for transactions that have not
    /// been executed yet and provides an indication of scheduling progress.
    min_never_scheduled_idx: CachePadded<AtomicU32>,
    /// Holds the indices of transactions currently scheduled for execution.
    /// Using a `BTreeSet` ensures that transactions are generally processed in an
    /// order (ascending by index by default when popping via `pop_first()`), which is
    /// beneficial for reducing execution conflicts.
    /// TODO(BlockSTMv2): Alternative implementations for performance (e.g. packed ints,
    /// intervals w. locks, CachePadded<ConcurrentQueue<TxnIndex>>).
    execution_queue: Mutex<BTreeSet<TxnIndex>>,
}

impl ExecutionQueueManager {
    pub(crate) fn new(num_txns: TxnIndex) -> Self {
        Self {
            executed_once_max_idx: CachePadded::new(AtomicU32::new(0)),
            min_never_scheduled_idx: CachePadded::new(AtomicU32::new(0)),
            execution_queue: Mutex::new((0..num_txns).collect()),
        }
    }

    fn pop_next(&self) -> Option<TxnIndex> {
        let ret = self.execution_queue.lock().pop_first();
        if let Some(idx) = ret {
            self.min_never_scheduled_idx
                .fetch_max(idx + 1, Ordering::Relaxed);
        }
        ret
    }

    fn min_never_scheduled_idx(&self) -> TxnIndex {
        self.min_never_scheduled_idx.load(Ordering::Relaxed)
    }

    // Note: the method must be performed while holding the idx-th status lock.
    pub(crate) fn add_to_schedule(&self, is_first_reexecution: bool, txn_idx: TxnIndex) {
        // In BlockSTMv2 algorithm, first re-execution gets a special scheduling treatment.
        // it is deferred until all previous transactions are executed at least once,
        // which is to ensure that all those transactions have produced their speculative
        // writes and the information can be used for intelligent scheduling. Note that
        // for the same reason, incarnation 0 (first execution) is never terminated early.
        if !is_first_reexecution || self.executed_once_max_idx.load(Ordering::Relaxed) >= txn_idx {
            self.execution_queue.lock().insert(txn_idx);
        }
    }

    pub(crate) fn remove_from_schedule(&self, txn_idx: TxnIndex) {
        self.execution_queue.lock().remove(&txn_idx);
    }
}

// Testing interfaces for ExecutionQueueManager.
impl ExecutionQueueManager {
    #[cfg(test)]
    pub(crate) fn new_for_test(executed_once_max_idx: u32) -> Self {
        Self {
            executed_once_max_idx: CachePadded::new(AtomicU32::new(executed_once_max_idx)),
            min_never_scheduled_idx: CachePadded::new(AtomicU32::new(0)),
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

/// Represents the different kinds of tasks that a worker thread can receive from [SchedulerV2].
///
/// This enum defines the instructions passed from [SchedulerV2::next_task] to an executor
/// (worker thread) to direct its activity.
#[derive(PartialEq, Debug)]
pub(crate) enum TaskKind<'a> {
    /// Instructs the worker to execute a specific `(TxnIndex, Incarnation)` of a transaction.
    /// The incarnation number starts at 0 for the first execution attempt and increments
    /// with each subsequent re-execution (after an abort).
    Execute(TxnIndex, Incarnation),
    /// Instructs the worker to perform post-commit processing for a given `TxnIndex`.
    /// This task is dispatched after a transaction has been successfully committed and its
    /// sequential client-side commit hook has been performed. The post-commit processing
    /// itself is assumed to be parallelizable and typically involves finalization or cleanup steps.
    PostCommitProcessing(TxnIndex),
    /// The module ids for which validation is required for txns in [from_idx_incl, to_idx_excl).
    ModuleValidation(TxnIndex, Incarnation, &'a BTreeSet<ModuleId>),
    /// Signals that no specific task (like `Execute` or `PostCommitProcessing`) is immediately
    /// available from the scheduler. The worker should typically call [SchedulerV2::next_task]
    /// again soon, possibly after a brief pause or yielding, to check for new work.
    NextTask,
    /// Signals that all transactions have been processed and committed, and the scheduler
    /// has no more work. The worker thread receiving this task can terminate.
    Done,
}

/// Flags representing the commit status of a transaction, used by [SchedulerV2].
///
/// These constant flag values are stored in [SchedulerV2::committed_marker] for each txn
/// to track its progress through the final stages of the commit process.
///
/// The typical lifecycle is: `NotCommitted` -> `CommitStarted` -> `Committed`.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum CommitMarkerFlag {
    /// Default state: The transaction has not yet been committed, nor has its
    /// sequential commit hook been dispatched.
    NotCommitted = 0,
    /// The transaction has been identified as the next to commit, and its sequential
    /// commit hook has been dispatched (obtained via [SchedulerV2::start_commit]).
    /// The system is now waiting for [SchedulerV2::end_commit] to be called for this transaction.
    CommitStarted = 1,
    /// The transaction's sequential commit hook has been successfully performed (indicated
    /// by a call to [SchedulerV2::end_commit]), and it is now fully committed from the
    /// scheduler's perspective. Its `PostCommitProcessing` task can now be scheduled.
    Committed = 2,
}

pub(crate) struct SchedulerV2 {
    /// Total number of transactions in the block. This is immutable after scheduler creation.
    num_txns: TxnIndex,
    /// The number of worker threads that will be processing tasks from this scheduler. Immutable.
    #[allow(dead_code)]
    num_workers: u32,

    /// Manages the `ExecutionStatus` for each transaction, which includes its current
    /// scheduling state, incarnation number, stall count, and dependency shortcut flag.
    /// Also embeds the [ExecutionQueueManager] for queuing transactions.
    txn_statuses: ExecutionStatuses,

    /// For each transaction `i`, `aborted_dependencies[i]` stores a list of transactions
    /// `j > i` that were previously aborted by `i` (due to `i`'s writes).
    /// This information is used to propagate stalls: if `i` is stalled or aborted,
    /// a stall signal is propagated to these dependent transactions `j`.
    /// Each [AbortedDependencies] instance is protected by a `Mutex`.
    aborted_dependencies: Vec<CachePadded<Mutex<AbortedDependencies>>>,

    /// The index of the next transaction that is eligible to have its sequential commit hook
    /// dispatched. This is incremented atomically as transactions are committed in order.
    next_to_commit_idx: CachePadded<AtomicU32>,
    /// Flag that becomes true when all transactions in the block have completed their
    /// post-commit processing, signaling that execution is finished.
    is_done: CachePadded<AtomicBool>,
    /// Flag that becomes true if the scheduler is explicitly halted (e.g., due to an error
    /// or a timeout). This signals workers to stop processing tasks.
    is_halted: CachePadded<AtomicBool>,

    /// Manages any uncommon validation requirements necessary before txns can be committed.
    /// For example, when a txn publishes a module, higher txns must have their module
    /// accesses validated since the reads are not covered by the normal (push) validation.
    cold_validation_requirements: CachePadded<ColdValidationRequirements<ModuleId>>,

    /// An armed lock used to serialize access to the critical section where sequential commit
    /// hooks are dispatched ([SchedulerV2::start_commit]). It helps manage contention with
    /// the arming mechanism.
    queueing_commits_lock: CachePadded<ArmedLock>,
    /// A concurrent queue holding the indices of transactions that have been committed and
    /// are ready for their parallel post-commit processing phase.
    post_commit_processing_queue: CachePadded<ConcurrentQueue<TxnIndex>>,
    /// For each txn `i`, `committed_marker[i]` stores its [CommitMarkerFlag], indicating
    /// its current stage in the commit process (NotCommitted, CommitStarted, Committed).
    committed_marker: Vec<CachePadded<AtomicU8>>,
}

impl SchedulerV2 {
    /// Creates a new [SchedulerV2] instance.
    ///
    /// Initializes all internal structures based on the total number of transactions
    /// (`num_txns`) in the block and the number of worker threads (`num_workers`).
    ///
    /// Panics if `num_txns` is 0 or `num_workers` is 0.
    pub(crate) fn new(num_txns: TxnIndex, num_workers: u32) -> Self {
        assert!(num_txns > 0, "No scheduler needed for 0 transactions");
        assert!(num_workers > 0, "Scheduler requires at least 1 worker");

        Self {
            num_txns,
            num_workers,
            txn_statuses: ExecutionStatuses::new(num_txns),
            aborted_dependencies: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(AbortedDependencies::new())))
                .collect(),
            next_to_commit_idx: CachePadded::new(AtomicU32::new(0)),
            is_done: CachePadded::new(AtomicBool::new(false)),
            is_halted: CachePadded::new(AtomicBool::new(false)),
            cold_validation_requirements: CachePadded::new(ColdValidationRequirements::new(
                num_txns,
            )),
            queueing_commits_lock: CachePadded::new(ArmedLock::new()),
            post_commit_processing_queue: CachePadded::new(ConcurrentQueue::<TxnIndex>::bounded(
                num_txns as usize,
            )),
            committed_marker: (0..num_txns)
                .map(|_| CachePadded::new(AtomicU8::new(CommitMarkerFlag::NotCommitted as u8)))
                .collect(),
        }
    }

    /// Attempts to acquire the `queueing_commits_lock` in a non-blocking way.
    ///
    /// Workers should call this to gain exclusive access to the critical section for
    /// dispatching sequential commit hooks. If the lock is acquired (`true`), the worker
    /// should proceed to call [SchedulerV2::start_commit] repeatedly, and then ensure
    /// [SchedulerV2::commit_hooks_unlock] is called.
    ///
    /// Returns `true` if the lock was acquired, `false` otherwise.
    pub(crate) fn commit_hooks_try_lock(&self) -> bool {
        self.queueing_commits_lock.try_lock()
    }

    /// Attempts to get the next transaction index that is ready to be committed. This method
    /// MUST be called only while holding the `queueing_commits_lock` (acquired via
    /// [SchedulerV2::commit_hooks_try_lock]). The worker can then perform a critical section
    /// consisting of any logic for committing a txn that needs to occur sequentially.
    /// The completion of this sequential commit hook logic must be followed by a call to
    /// [SchedulerV2::end_commit].
    ///
    /// The method checks the following conditions:
    /// 1. The scheduler is not halted.
    /// 2. `next_to_commit_idx` is less than `num_txns` (i.e., there are transactions remaining).
    /// 3. The transaction at `next_to_commit_idx` has its status as `Executed` (verified by
    ///    [ExecutionStatuses::is_executed]).
    ///
    /// If all conditions are met:
    /// - The `committed_marker` for `next_to_commit_idx` is updated from `NotCommitted` to
    ///   `CommitStarted`. Panics if the marker was not `NotCommitted`.
    /// - `next_to_commit_idx` is atomically incremented.
    /// - Returns `Ok(Some((txn_idx, incarnation)))` for the transaction to be committed.
    ///
    /// If conditions are not met it returns `Ok(None)` or an `Err` for invariant violations.
    ///
    /// An important invariant check: Before attempting to dispatch transaction `i`, it verifies
    /// that transaction `i-1` has its `committed_marker` as `Committed`. This ensures strict
    /// sequential processing of commit hooks.
    pub(crate) fn start_commit(&self) -> Result<Option<(TxnIndex, Incarnation)>, PanicError> {
        // Relaxed ordering due to armed lock acq-rel.
        let next_to_commit_idx = self.next_to_commit_idx.load(Ordering::Relaxed);
        assert!(next_to_commit_idx <= self.num_txns);

        if self.is_halted() || next_to_commit_idx == self.num_txns {
            // All sequential commit hooks are already dispatched.
            return Ok(None);
        }

        let incarnation = self.txn_statuses.incarnation(next_to_commit_idx);
        if self.txn_statuses.is_executed(next_to_commit_idx) {
            self.commit_marker_invariant_check(next_to_commit_idx)?;

            // All prior transactions are committed and the latest incarnation of the transaction
            // at next_to_commit_idx has finished but has not been aborted. If any of its reads was
            // incorrect, it would have been invalidated by the respective transaction's last
            // (committed) (re-)execution, and led to an abort in the corresponding finish execution
            // (which, inductively, must occur before the transaction is committed). Hence, it
            // must also be safe to commit the current transaction.
            //
            // The only exception is if there are unsatisfied cold validation requirements,
            // blocking the commit. These may not yet be scheduled for validation, or deferred
            // until after the txn finished execution, whereby deferral happens before txn status
            // becomes Executed, while validation and unblocking happens after.
            if self
                .cold_validation_requirements
                .is_commit_blocked(next_to_commit_idx, incarnation)
            {
                // May not commit a txn with an unsatisfied validation requirement. This will be
                // more rare than !is_executed in the common case, hence the order of checks.
                return Ok(None);
            }
            // The check might have passed after the validation requirement has been fulfilled.
            // Yet, if validation failed, the status would be aborted before removing the block,
            // which would increase the incarnation number. It is also important to note that
            // blocking happens during sequential commit hook, while holding the lock (which is
            // also held here), hence before the call of this method.
            if incarnation != self.txn_statuses.incarnation(next_to_commit_idx) {
                return Ok(None);
            }

            if self
                .committed_marker
                .get(next_to_commit_idx as usize)
                .map_or(false, |marker| {
                    marker.swap(CommitMarkerFlag::CommitStarted as u8, Ordering::Relaxed)
                        != CommitMarkerFlag::NotCommitted as u8
                })
            {
                return Err(code_invariant_error(format!(
                    "Marking {} as PENDING_COMMIT_HOOK, but previous marker != NOT_COMMITTED",
                    next_to_commit_idx
                )));
            }

            // TODO(BlockSTMv2): fetch_add as a RMW instruction causes a barrier even with
            // Relaxed ordering. The read is only used to check an invariant, so we can
            // eventually change to just a relaxed write.
            let prev_idx = self.next_to_commit_idx.fetch_add(1, Ordering::Relaxed);
            if prev_idx != next_to_commit_idx {
                return Err(code_invariant_error(format!(
                    "Scheduler committing {}, stored next to commit idx = {}",
                    next_to_commit_idx, prev_idx
                )));
            }

            return Ok(Some((
                next_to_commit_idx,
                self.txn_statuses.incarnation(next_to_commit_idx),
            )));
        }

        Ok(None)
    }

    /// Called by a worker after it has successfully executed sequential commit hook logic
    /// for 'txn_idx' in the critical section following [SchedulerV2::start_commit].
    ///
    /// This method performs two main actions:
    /// 1. Updates the `committed_marker` for `txn_idx` from `CommitStarted` to `Committed`.
    ///    Panics if the previous marker was not `CommitStarted`.
    /// 2. Pushes `txn_idx` to the `post_commit_processing_queue`, making it available for
    ///    a `PostCommitProcessing` task to be dispatched by [SchedulerV2::next_task].
    ///    Panics if the queue push fails (e.g., if the queue is full, which shouldn't happen
    ///    given it's bounded by `num_txns`).
    ///
    /// It is crucial that `txn_idx` was previously obtained from a successful call to
    /// [SchedulerV2::start_commit], and that the `queueing_commits_lock` was held
    /// by the worker during the execution of the commit hook and this call.
    pub(crate) fn end_commit(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        let prev_marker = self.committed_marker[txn_idx as usize].load(Ordering::Relaxed);
        if prev_marker != CommitMarkerFlag::CommitStarted as u8 {
            return Err(code_invariant_error(format!(
                "Marking txn {} as COMMITTED, but previous marker {} != {}",
                txn_idx,
                prev_marker,
                CommitMarkerFlag::CommitStarted as u8
            )));
        }
        // Allows next sequential commit hook to be processed.
        self.committed_marker[txn_idx as usize]
            .store(CommitMarkerFlag::Committed as u8, Ordering::Relaxed);

        if let Err(e) = self.post_commit_processing_queue.push(txn_idx) {
            return Err(code_invariant_error(format!(
                "Error adding {txn_idx} to commit queue, len {}, error: {:?}",
                self.post_commit_processing_queue.len(),
                e
            )));
        }

        Ok(())
    }

    /// Unlocks the `queueing_commits_lock`.
    ///
    /// This method must be called by a worker after it has finished attempting to acquire
    /// and process sequential commit hooks (i.e., after its loop calling
    /// [SchedulerV2::start_commit] and [SchedulerV2::end_commit] is done.
    ///
    /// If the next transaction to commit (`next_to_commit_idx`) is ready (i.e., executed
    /// and not halted), this method will "arm" the lock, signifying that new commit work
    /// might be available.
    pub(crate) fn commit_hooks_unlock(&self) {
        let next_to_commit_idx = self.next_to_commit_idx.load(Ordering::Relaxed);
        if next_to_commit_idx < self.num_txns
            && !self.is_halted()
            && self.txn_statuses.is_executed(next_to_commit_idx)
        {
            self.queueing_commits_lock.arm();
        }

        self.queueing_commits_lock.unlock();
    }

    /// Checks if the `post_commit_processing_queue` is empty.
    ///
    /// Returns `true` if there are no transactions awaiting post-commit processing,
    /// `false` otherwise.
    pub(crate) fn post_commit_processing_queue_is_empty(&self) -> bool {
        self.post_commit_processing_queue.is_empty()
    }

    /// Returns the minimum transaction index that has not yet been scheduled (i.e.,
    /// popped from the `execution_queue` by [ExecutionQueueManager::pop_next]).
    ///
    /// This provides an indication of how far along the scheduler is in dispatching
    /// initial execution tasks.
    ///
    /// The value is retrieved from [ExecutionQueueManager::min_never_scheduled_idx].
    ///
    /// Returns `Err(PanicError)` if the value read is inconsistent (e.g., greater
    /// than `num_txns`).
    pub(crate) fn min_never_scheduled_idx(&self) -> Result<TxnIndex, PanicError> {
        let ret = self
            .txn_statuses
            .get_execution_queue_manager()
            .min_never_scheduled_idx();
        if ret > self.num_txns {
            return Err(code_invariant_error(format!(
                "min_never_scheduled_idx: {} > num_txns: {}",
                ret, self.num_txns
            )));
        }
        Ok(ret)
    }

    /// Fetches the next task for a worker thread.
    ///
    /// This is the primary method workers call to get work from the scheduler.
    /// The scheduler prioritizes tasks as follows:
    /// 1.  **`Done`**: If [SchedulerV2::is_done] is true, it returns `TaskKind::Done`.
    /// 2.  **`PostCommitProcessing`**: Attempts to pop a `txn_idx` from the
    ///     `post_commit_processing_queue`. If successful, returns
    ///     `TaskKind::PostCommitProcessing(txn_idx)`. If this was the last transaction
    ///     (`num_txns - 1`), it also sets `is_done` to true.
    /// 3.  **`Done` (if halted)**: If the `post_commit_processing_queue` is empty and
    ///     [SchedulerV2::is_halted] is true, returns `TaskKind::Done`.
    /// 4.  **`Execute`**: Attempts to pop a `txn_idx` from the main `execution_queue` via
    ///     [ExecutionQueueManager::pop_next] (accessed through `txn_statuses`). If successful,
    ///     it then calls [SchedulerV2::start_executing] to mark the transaction as `Executing`
    ///     and get its current incarnation. If [SchedulerV2::start_executing] returns
    ///     `Some(incarnation)`, it returns `TaskKind::Execute(txn_idx, incarnation)`. If
    ///     [SchedulerV2::start_executing] returns `None`, it returns `TaskKind::NextTask`.
    /// 5.  **`NextTask`**: If none of the above yield a task (e.g., queues are empty, no work
    ///     to start), it returns `TaskKind::NextTask`, indicating the worker should try again.
    ///
    /// Returns `Err(PanicError)` if an invariant is violated (e.g., commit queue closed).
    ///
    /// TODO: take worker ID, dedicate some workers to scan high priority tasks (can use armed lock).
    /// We can also have different versions (e.g. for testing) of next_task.
    pub(crate) fn next_task(&self, worker_id: u32) -> Result<TaskKind<'_>, PanicError> {
        if self.is_done() {
            return Ok(TaskKind::Done);
        }

        if let Some(cold_validation_task) = self.handle_cold_validation_requirements(worker_id)? {
            return Ok(cold_validation_task);
        }

        match self.pop_post_commit_task()? {
            Some(txn_idx) => {
                return Ok(TaskKind::PostCommitProcessing(txn_idx));
            },
            None => {
                if self.is_halted() {
                    return Ok(TaskKind::Done);
                }
            },
        }

        if let Some(txn_idx) = self.txn_statuses.get_execution_queue_manager().pop_next() {
            if let Some(incarnation) = self.start_executing(txn_idx)? {
                return Ok(TaskKind::Execute(txn_idx, incarnation));
            }
        }

        Ok(TaskKind::NextTask)
    }

    /// Checks invariants and prepares the scheduler state for executing a block epilogue txn at
    /// block_epilogue_idx. If block_epilogue_idx is not the last txn, then the block must have
    /// been cut, and the status must be adjusted to execute the next incarnation. In particular,
    /// the status may not be 'Executing' (PanicError returned), as even speculative execution
    /// when the block is halted must record the execution result and notify the scheduler (or
    /// else the speculative outputs can't be cleaned up from the shared data structures).
    ///
    /// Otherwise, the status must be converted to 'Executing', possibly after aborting a previous
    /// 'Executed' incarnation.
    pub(crate) fn prepare_for_block_epilogue(
        &self,
        block_epilogue_idx: TxnIndex,
    ) -> Result<Incarnation, PanicError> {
        if block_epilogue_idx != self.num_txns {
            self.txn_statuses
                .prepare_for_block_epilogue(block_epilogue_idx)
        } else {
            Ok(0)
        }
    }

    /// Finalizes the execution of a transaction and processes its outcomes.
    ///
    /// This method is called by a worker after it has finished executing a transaction
    /// (identified by `abort_manager.owner_txn_idx` and `abort_manager.owner_incarnation`).
    /// The `abort_manager` contains information about transactions that were invalidated
    /// (i.e., need to be aborted) due to the writes of the just-executed transaction.
    ///
    /// Key actions performed:
    /// 1.  **Extracts Data**: Takes ownership of the `invalidated_set` from the `abort_manager`.
    /// 2.  **Record Aborted Dependencies**: If the executed `incarnation > 0`, records the
    ///     transactions that were successfully aborted (from `invalidated_set`) as aborted
    ///     dependencies of `txn_idx`. This is skipped for incarnation 0 as the initial writes
    ///     might cause invalidations very different from the subsequent re-executions.
    /// 3.  **Finish Aborts**: For each transaction in `invalidated_set` for which `start_abort`
    ///     was successful (i.e., `Some(incarnation)` was stored by [AbortManager]), calls
    ///     [ExecutionStatuses::finish_abort] to complete the abort process. These transactions
    ///     are added to a `stall_propagation_queue`.
    /// 4.  **Finish Own Execution**: Calls [ExecutionStatuses::finish_execution]
    ///     to update the status of the just-executed transaction (e.g., to `Executed` or to
    ///     `PendingScheduling` if it was aborted concurrently). If this call indicates the
    ///     transaction is now `Executed` (returns true), `txn_idx` is also added to the
    ///     `stall_propagation_queue`.
    /// 5.  **Arm Commit Lock**: If the transaction is now `Executed` and it's either transaction 0
    ///     or the preceding transaction (`txn_idx - 1`) is no longer `NotCommitted`, it arms
    ///     the `queueing_commits_lock`, implying that `txn_idx` might be ready for commit.
    /// 6.  **Update `executed_once_max_idx`**: If `incarnation == 0`, calls
    ///     [SchedulerV2::try_increase_executed_once_max_idx].
    /// 7.  **Propagate Stalls/Unstalls**: Calls [SchedulerV2::propagate] to recursively
    ///     process stall/unstall signals based on the status changes of transactions in the
    ///     `stall_propagation_queue`.
    ///
    /// Returns `Err(PanicError)` if any underlying status update fails.
    pub(crate) fn finish_execution<'a>(
        &'a self,
        abort_manager: AbortManager<'a>,
    ) -> Result<Option<BTreeSet<ModuleId>>, PanicError> {
        let (txn_idx, incarnation, invalidated_set) = abort_manager.take();

        if txn_idx == self.num_txns {
            // Must be the block epilogue txn.
            return Ok(None);
        }

        if incarnation > 0 {
            // Record aborted dependencies. Only recording for incarnations > 0 is in line with the
            // optimistic value validation principle of Block-STMv2. 0-th incarnation might invalidate
            // due to the first write, but later incarnations could make the same writes - in which case
            // there is no need to record (and stall, etc) the corresponding dependency.
            self.aborted_dependencies[txn_idx as usize]
                .lock()
                .record_dependencies(invalidated_set.keys().copied());
        }

        let mut stall_propagation_queue: BTreeSet<usize> = BTreeSet::new();
        for (txn_idx, maybe_incarnation) in invalidated_set {
            if let Some(incarnation) = maybe_incarnation {
                self.txn_statuses
                    .finish_abort(txn_idx, incarnation, false)?;
                stall_propagation_queue.insert(txn_idx as usize);
            }
        }

        let maybe_module_validation_requirements =
            self.txn_statuses.finish_execution(txn_idx, incarnation)?;
        if maybe_module_validation_requirements.is_some() {
            stall_propagation_queue.insert(txn_idx as usize);

            if txn_idx == 0
                || self.committed_marker[txn_idx as usize - 1].load(Ordering::Relaxed)
                    != CommitMarkerFlag::NotCommitted as u8
            {
                // If the committed marker is NOT_COMMITTED by the time the last execution of a
                // transaction finishes, then considering the lowest such index, arming will occur
                // either because txn_idx = 0 (base case), or after the marker is set, in the
                // commits_hooks_unlock method (which checks the executed status).
                self.queueing_commits_lock.arm();
            }
        }

        if incarnation == 0 {
            self.try_increase_executed_once_max_idx(txn_idx);
        }

        // Handle recursive propagation of add / remove stall.
        self.propagate(stall_propagation_queue)?;

        Ok(maybe_module_validation_requirements)
    }

    /// Sets the scheduler to a halted state.
    ///
    /// This is an atomic operation. Once halted, [SchedulerV2::is_halted] will return `true`,
    /// and [SchedulerV2::next_task] will start returning `TaskKind::Done` once the
    /// `post_commit_processing_queue` is empty.
    /// This is typically used to signal an abnormal termination or a need to stop processing
    /// transactions beyond a certain index.
    ///
    /// Returns `true` if this call actually changed the state from not-halted to halted,
    /// `false` if it was already halted.
    pub(crate) fn halt(&self) -> bool {
        // TODO(BlockSTMv2): Notify waiting workers when supported.
        !self.is_halted.swap(true, Ordering::SeqCst)
    }

    /// Checks if the scheduler is globally halted or if a specific transaction incarnation
    /// has been aborted.
    ///
    /// This is used by workers during transaction execution to determine if they should
    /// stop processing early.
    ///
    /// - Returns `true` if [SchedulerV2::is_halted] is true.
    /// - If not globally halted and `incarnation == 0`, returns `false` (0-th incarnation
    ///   is never aborted early to ensure its speculative writes are produced).
    /// - Otherwise (not halted, `incarnation > 0`), returns the result of
    ///   [ExecutionStatuses::already_started_abort].
    #[inline]
    pub(crate) fn is_halted_or_aborted(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        if self.is_halted() {
            return true;
        }

        if incarnation == 0 {
            // Never interrupt the 0-th incarnation due to an early abort to get the first output
            // estimation (even if it is based on invalidated reads).
            return false;
        }

        self.txn_statuses
            .already_started_abort(txn_idx, incarnation)
    }

    // An interface for a particular transaction to be aborted directly, without abort manager.
    // Abort manager is used with finish_execution API, which is a part of push invalidation
    // flow and a core logic of BlockSTMv2. The direct_abort API below is targeted for other
    // scenarios in which direct access to the abort functionality is helpful.
    //
    // This is currently used in three scenarios:
    // (1) when processing dependencies (if the heuristics determines the txn is better to be
    // speculatively aborted, e.g. since it is low priority and likely to be invalidated.
    // Since the status must be Executing, we do not bother with propagating add / remove stall.
    // TODO(BlockSTMv2): Consider adding this functionality.
    // (2) when re-executing a txn during commit, i.e. due to a delayed field invalidation or
    // for applying certain outputs of module publishing txn to the shared data structures.
    // In this case, the caller needs to re-execute the txn itself, and start_next_incarnation
    // parameter is set to true. This ensures the status was Executed, and atomically turns into
    // Executing bypassing PendingScheduling to make sure it is not assigned to a different worker
    // by the scheduler / QueueingCommitManager.
    // (3) The module validation pass can cause invalidation, which requires aborting.
    pub(crate) fn direct_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        start_next_incarnation: bool,
    ) -> Result<bool, PanicError> {
        if self.txn_statuses.start_abort(txn_idx, incarnation)? {
            self.txn_statuses
                .finish_abort(txn_idx, incarnation, start_next_incarnation)?;
            return Ok(true);
        }

        if start_next_incarnation {
            return Err(code_invariant_error(format!(
                "SchedulerV2: self-abort with start_next_incarnation failed for {} {}",
                txn_idx, incarnation
            )));
        }

        Ok(false)
    }

    pub(crate) fn record_validation_requirements(
        &self,
        worker_id: u32,
        txn_idx: TxnIndex,
        module_ids: BTreeSet<ModuleId>,
    ) -> Result<(), PanicError> {
        if worker_id >= self.num_workers {
            return Err(code_invariant_error(format!(
                "Worker ID {} must be less than the number of workers {}",
                worker_id, self.num_workers
            )));
        }
        if txn_idx >= self.num_txns {
            return Err(code_invariant_error(format!(
                "Txn index {} must be less than the number of transactions {}",
                txn_idx, self.num_txns
            )));
        }

        let min_never_scheduled_idx = self.min_never_scheduled_idx()?;
        if txn_idx >= min_never_scheduled_idx {
            return Err(code_invariant_error(format!(
                "Calling txn idx {} must be less than min_never_scheduled_idx {}",
                txn_idx, min_never_scheduled_idx
            )));
        }
        self.cold_validation_requirements.record_requirements(
            worker_id,
            txn_idx,
            min_never_scheduled_idx,
            module_ids,
        )
    }

    /// Called from the executor when validation requirement has been fulfilled.
    pub(crate) fn finish_cold_validation_requirement(
        &self,
        worker_id: u32,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        was_deferred: bool,
    ) -> Result<(), PanicError> {
        if was_deferred {
            self.cold_validation_requirements
                .deferred_requirements_completed(txn_idx, incarnation)?;
        } else {
            self.cold_validation_requirements
                .validation_requirement_processed(worker_id, txn_idx, incarnation, false)?;
        }
        Ok(())
    }
}

/// Private interfaces
impl SchedulerV2 {
    // TODO(BlockSTMv2): Tests for SchedulerV2 handling of cold validation requirements.
    // (currently covered by cold validation tests & proptests).
    fn handle_cold_validation_requirements(
        &self,
        worker_id: u32,
    ) -> Result<Option<TaskKind<'_>>, PanicError> {
        if !self
            .cold_validation_requirements
            .is_dedicated_worker(worker_id)
        {
            return Ok(None);
        }

        if let Some((
            txn_idx,
            incarnation,
            ValidationRequirement {
                requirements: modules_to_validate,
                is_deferred,
            },
        )) = self
            .cold_validation_requirements
            .get_validation_requirement_to_process(
                worker_id,
                // Heuristic formula for when the cold validation requirement should be
                // processed, based on the distance from the last committed index, and
                // increasing linearly with the number of workers. If a requirement is for
                // a txn with an index higher than the computed threshold, then the worker
                // prioritizes other tasks, with additional benefit that when an incarnation
                // aborts, its requirements become outdated and no need to be processed.
                self.next_to_commit_idx.load(Ordering::Relaxed)
                    + self.num_workers as TxnIndex * 3
                    + 4,
                &self.txn_statuses,
            )?
        {
            if is_deferred {
                let defer_outcome = self.txn_statuses.defer_module_validation(
                    txn_idx,
                    incarnation,
                    modules_to_validate,
                )?;

                if defer_outcome == Some(false) {
                    // defer call did not succeed because the incarnation had finished execution.
                    // Ask the caller (the dedicated worker) to process the requirements normally.
                    return Ok(Some(TaskKind::ModuleValidation(
                        txn_idx,
                        incarnation,
                        modules_to_validate,
                    )));
                }

                self.cold_validation_requirements
                    .validation_requirement_processed(
                        worker_id,
                        txn_idx,
                        incarnation,
                        // When the defer call was not successful because the requirements were no
                        // longer relevant, validation_still_needed parameter must be passed as false.
                        defer_outcome == Some(true),
                    )?;
            } else {
                // Cheap check for whether the requirement is already outdated.
                if self
                    .txn_statuses
                    .already_started_abort(txn_idx, incarnation)
                {
                    self.cold_validation_requirements
                        .validation_requirement_processed(worker_id, txn_idx, incarnation, false)?;
                } else {
                    return Ok(Some(TaskKind::ModuleValidation(
                        txn_idx,
                        incarnation,
                        modules_to_validate,
                    )));
                }
            }
        }
        Ok(None)
    }

    // Called when considering committing a txn. At this point, the commit hooks lock is held by
    // the caller, and the marker here should be 'COMMITTED'. NOT_COMMITTED means the previous
    // call to [SchedulerV2::start_commit] that increased the index did not set the status,
    // while PENDING_COMMIT_HOOK would imply the caller never made the end_commit call
    // (should only happen in error scenarios).
    fn commit_marker_invariant_check(
        &self,
        next_to_commit_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        if next_to_commit_idx > 0 {
            let prev_committed_marker =
                self.committed_marker[next_to_commit_idx as usize - 1].load(Ordering::Relaxed);
            if prev_committed_marker != CommitMarkerFlag::Committed as u8 {
                return Err(code_invariant_error(format!(
                    "Trying to get commit hook for {}, but previous index marker {} != {} (COMMITTED)",
                    next_to_commit_idx, prev_committed_marker, CommitMarkerFlag::Committed as u8,
                )));
            };
        }
        Ok(())
    }

    fn pop_post_commit_task(&self) -> Result<Option<TxnIndex>, PanicError> {
        match self.post_commit_processing_queue.pop() {
            Ok(txn_idx) => {
                if txn_idx == self.num_txns - 1 {
                    self.is_done.store(true, Ordering::SeqCst);
                }
                Ok(Some(txn_idx))
            },
            Err(PopError::Empty) => Ok(None),
            Err(PopError::Closed) => {
                Err(code_invariant_error("Commit queue should never be closed"))
            },
        }
    }

    /// Propagates stall or unstall signals recursively through the dependency graph.
    ///
    /// This method processes a `stall_propagation_queue` containing transaction indices
    /// whose states might have changed (e.g., executed, aborted, stalled, unstalled).
    /// For each `task_idx` popped from the queue:
    /// 1. It acquires a lock on `self.aborted_dependencies[task_idx]`.
    /// 2. It checks the current status of `task_idx` using `txn_statuses`:
    ///    - If `task_idx` is [ExecutionStatuses::shortcut_executed_and_not_stalled]
    ///      (meaning executed and not currently considered stalled by `ExecutionStatuses`),
    ///      it calls [AbortedDependencies::remove_stall] on its [AbortedDependencies].
    ///      This, in turn, might add more indices (dependencies of `task_idx` that
    ///      were unstalled) to the `stall_propagation_queue`.
    ///    - Otherwise (if `task_idx` is not executed or is stalled), it calls
    ///      [AbortedDependencies::add_stall] on its [AbortedDependencies]. This might
    ///      add more indices (dependencies of `task_idx` that were stalled) to the
    ///      `stall_propagation_queue`.
    ///
    /// The process continues until the `stall_propagation_queue` is empty.
    /// This mechanism ensures that stall states are consistently propagated based on the
    /// most up-to-date status of transactions.
    fn propagate(&self, mut stall_propagation_queue: BTreeSet<usize>) -> Result<(), PanicError> {
        // Dependencies of each transaction always have higher indices than the transaction itself.
        // This means that the stall propagation queue is always processed in ascending order of
        // transaction indices, and that the processing loop is guaranteed to terminate.
        while let Some(task_idx) = stall_propagation_queue.pop_first() {
            // Make sure the conditions are checked under dependency lock.
            let mut aborted_deps_guard = self.aborted_dependencies[task_idx].lock();

            // Checks the current status to determine whether to propagate add / remove stall,
            // calling which only affects its currently not_stalled (or stalled) dependencies.
            // Allows to store indices in propagation queue (not add or remove commands) & avoids
            // handling corner cases such as merging commands (as propagation process is not atomic).
            if self
                .txn_statuses
                .shortcut_executed_and_not_stalled(task_idx)
            {
                // Still makes sense to propagate remove_stall.
                aborted_deps_guard
                    .remove_stall(&self.txn_statuses, &mut stall_propagation_queue)?;
            } else {
                // Not executed or stalled - still makes sense to propagate add_stall.
                aborted_deps_guard.add_stall(&self.txn_statuses, &mut stall_propagation_queue)?;
            }
        }
        Ok(())
    }

    /// Initiates the abort process for a specific transaction incarnation via `ExecutionStatuses`.
    ///
    /// This is a wrapper around [ExecutionStatuses::start_abort]. If the abort is successfully
    /// initiated (i.e., [ExecutionStatuses::start_abort] returns `Ok(true)`):
    /// - Increments the `SPECULATIVE_ABORT_COUNT` counter.
    /// - Clears any speculative transaction logs for the `txn_idx` (as its current execution
    ///   attempt is being aborted).
    ///
    /// Returns the result of [ExecutionStatuses::start_abort].
    fn start_abort(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> Result<bool, PanicError> {
        self.txn_statuses.start_abort(txn_idx, incarnation)
    }

    /// Initiates the execution of a transaction via `ExecutionStatuses`.
    ///
    /// This is a direct wrapper around [ExecutionStatuses::start_executing]. It attempts to
    /// transition the transaction at `txn_idx` to the `Executing` state.
    ///
    /// Returns `Ok(Some(incarnation))` if successful, `Ok(None)` if the transaction
    /// was not in a state to start execution (e.g., not `PendingScheduling`), or
    /// `Err(PanicError)` if an error occurs within `ExecutionStatuses`.
    fn start_executing(&self, txn_idx: TxnIndex) -> Result<Option<Incarnation>, PanicError> {
        self.txn_statuses.start_executing(txn_idx)
    }

    /// Attempts to advance the `executed_once_max_idx` watermark.
    ///
    /// This watermark tracks the highest contiguous transaction index `i` such that all
    /// transactions `0..i` have been executed at least once.
    ///
    /// This method is typically called when a transaction `txn_idx` finishes its 0-th incarnation.
    /// If `executed_once_max_idx` is currently equal to `txn_idx`, it means `txn_idx` might
    /// be the transaction completing a contiguous block of first-time executions.
    /// The method then iterates from `txn_idx` upwards, checking `[ExecutionStatuses::ever_executed]`
    /// for each subsequent transaction `idx`. For each `idx` that has been executed at least once:
    /// - `executed_once_max_idx` is updated to `idx + 1`.
    /// - If `idx` is found to be [ExecutionStatuses::pending_scheduling_and_not_stalled], it is
    ///   re-added to the `execution_queue`. This handles cases where a transaction `idx` might
    ///   have been scheduled for re-execution (e.g., its first re-execution) but was deferred
    ///   because `executed_once_max_idx` was less than `idx`. Now that the watermark is advancing
    ///   past `idx`, it can be truly scheduled.
    /// The iteration stops when an `idx` is encountered that has not `ever_executed` or when
    /// `num_txns` is reached.
    fn try_increase_executed_once_max_idx(&self, txn_idx: TxnIndex) {
        let execution_queue_manager = self.txn_statuses.get_execution_queue_manager();
        // Synchronization is provided by the ordering of [SchedulerV2::finish_execution]
        // updating the transaction inner status (under lock), and the ever_executed
        // check below, which also acquires the lock. In particular, ordering is as follows:
        // (a) finish_execution(idx) with idx lock -> executed_idx == txn_idx check
        // (b) increment executed_idx to txn_idx -> ever_executed check under lock
        // Note that (classic flags principle), in case when ever_executed check fails,
        // executed_idx == txn_idx check is guaranteed to succeed.
        if execution_queue_manager
            .executed_once_max_idx
            .load(Ordering::Relaxed)
            == txn_idx
        {
            let mut idx = txn_idx;
            while idx < self.num_txns && self.txn_statuses.ever_executed(idx) {
                // A successful check of ever_executed holds idx-th status lock and follows an
                // increment of executed_once_max_idx to idx in the prior loop iteration.
                execution_queue_manager
                    .executed_once_max_idx
                    .store(idx + 1, Ordering::Relaxed);

                // Note: for first re-execution, [ExecutionQueueManager::add_to_schedule] adds
                // an index to the execution queue only once executed_once_max_idx >= idx.
                // We need to ensure that re-execution is not missed due to a concurrency
                // race where after the index is added to the execution queue below, it gets
                // removed by [ExecutionStatuses::add_stall] but not re-added due to the
                // aforementioned check after [ExecutionStatuses::remove_stall]. This holds
                // because stall can only remove idx from the execution queue while holding
                // the idx-th status lock, which would have to be after ever_executed, and
                // the corresponding remove_stall would hence acquire the same lock even later,
                // and hence be guaranteed to observe executed_once_max_idx >= idx.

                // TODO(BlockSTMv2): Audit / should we keep ever_executed lock instead of re-acquiring.
                if self.txn_statuses.pending_scheduling_and_not_stalled(idx) {
                    execution_queue_manager.execution_queue.lock().insert(idx);
                }

                idx += 1;
            }
        }
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
    use crate::scheduler_status::{ExecutionStatus, SchedulingStatus, StatusWithIncarnation};
    use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some_eq};
    use fail::FailScenario;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::cmp::min;
    use test_case::test_case;

    // Helper function to invalidate all transactions after a given index
    fn invalidate_after_index(
        abort_manager: &mut AbortManager,
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
        let mut stall_propagation_queue = BTreeSet::new();

        // num_txns is 6.
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(6), vec![
                ExecutionStatus::new(),
                ExecutionStatus::new(),
                // Statuses for txn_idx 2, 3, 4 have incarnation > 0 and different inner status.
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    0,
                ),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(
                        SchedulingStatus::Executing(BTreeSet::new()),
                        1,
                    ),
                    0,
                ),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 1),
                    0,
                ),
                // Status for txn 5 is already stalled.
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    1,
                ),
            ]);
        assert_eq!(statuses.len(), 6);
        let manager = &statuses.get_execution_queue_manager();
        let mut deps = AbortedDependencies::new();

        assert!(!deps.is_stalled);
        assert_ok!(deps.add_stall(&statuses, &mut stall_propagation_queue));
        assert!(deps.is_stalled);
        deps.not_stalled_deps.insert(0);
        // Err because of incarnation 0.
        assert_err!(deps.add_stall(&statuses, &mut stall_propagation_queue));
        // From now on, mark 0 as already stalled.
        assert!(deps.stalled_deps.insert(0));
        assert!(deps.not_stalled_deps.remove(&0));

        // Successful stall when status requires execution must remove 2 from execution
        // queue, while different status or unsuccessful stall should not.
        manager.execution_queue.lock().clear();
        manager.execution_queue.lock().append(&mut (2..6).collect());
        deps.not_stalled_deps.append(&mut (2..6).collect());
        assert_ok!(deps.add_stall(&statuses, &mut stall_propagation_queue));

        // Check the results: execution queue, propagation_queue, deps.stalled & not_stalled.
        assert_eq!(manager.execution_queue.lock().len(), 3);
        for i in 3..6 {
            assert!(manager.execution_queue.lock().contains(&i));
        }

        // 5 is not in the propagation queue because it was already stalled.
        assert_eq!(stall_propagation_queue.len(), 3);
        for i in 2..5 {
            assert!(stall_propagation_queue.contains(&i));
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
        let mut stall_propagation_queue = BTreeSet::new();

        // num_txns is 8.
        let mut statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(6), vec![
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    0,
                ),
                ExecutionStatus::new(),
                // For the next 3 statuses, executed_once_max_idx will be >= their
                // indices. Only 4 should be add to execution queue, as 2 and 3 do
                // not require execution. All should be added to propagation queue.
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(
                        SchedulingStatus::Executing(BTreeSet::new()),
                        1,
                    ),
                    1,
                ),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 1),
                    1,
                ),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    1,
                ),
                // For below statuses, executed_once_max_idx will be < their indices:
                // we will test is_first_incarnation behavior.
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    1,
                ),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 2),
                    1,
                ),
                // Should not be added to the queues, as num_stalls = 2 (status
                // remains stalled after call).
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 2),
                    2,
                ),
            ]);
        let mut deps = AbortedDependencies::new();
        assert_eq!(statuses.len(), 8);

        deps.is_stalled = true;
        assert_ok!(deps.remove_stall(&statuses, &mut stall_propagation_queue));
        assert!(!deps.is_stalled);
        deps.stalled_deps.insert(0);
        // Removing stall should fail because num_stalls = 0.
        assert_err!(deps.remove_stall(&statuses, &mut stall_propagation_queue));
        *statuses.get_status_mut(0) = ExecutionStatus::new_for_test(
            StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 0),
            1,
        );
        // Removing stall should fail because incarnation = 0.
        assert_err!(deps.remove_stall(&statuses, &mut stall_propagation_queue));

        let manager = &statuses.get_execution_queue_manager();
        manager.executed_once_max_idx.store(4, Ordering::Relaxed);

        // From now on, ignore status for index 0 (mark as not_stalled):
        assert!(deps.not_stalled_deps.insert(0));
        assert!(deps.stalled_deps.remove(&0));

        manager.execution_queue.lock().clear();
        deps.stalled_deps.append(&mut (2..8).collect());
        assert_ok!(deps.remove_stall(&statuses, &mut stall_propagation_queue,));

        // Check the results: scheduling queue, propagation_queue, deps.stalled & not_stalled.
        assert_eq!(manager.execution_queue.lock().len(), 2);
        for i in [4, 6].iter() {
            assert!(manager.execution_queue.lock().contains(i));
        }

        assert_eq!(stall_propagation_queue.len(), 5);
        for i in 2..7 {
            stall_propagation_queue.contains(&i);
        }

        assert_eq!(deps.stalled_deps.len(), 0);
        assert_eq!(deps.not_stalled_deps.len(), 7);
        assert!(deps.not_stalled_deps.contains(&0)); // pre-inserted
        for i in 2..8 {
            assert!(deps.not_stalled_deps.contains(&i));
        }
    }

    #[test]
    fn propagate() {
        let scheduler = SchedulerV2::new(10, 2);

        let test_indices = [0, 2, 4];
        for idx in test_indices {
            assert!(!scheduler.aborted_dependencies[idx].lock().is_stalled);
            assert!(!scheduler.txn_statuses.get_status(idx as u32).is_stalled());
        }
        scheduler.propagate(BTreeSet::from(test_indices)).unwrap();
        for idx in test_indices {
            assert!(scheduler.aborted_dependencies[idx].lock().is_stalled);
            // Propagate does not call stall for the status itself, only
            // propagates to aborted dependencies based on the status (assumption
            // being the status is already updated, e.g. due to propagation).
            assert!(!scheduler.txn_statuses.get_status(idx as u32).is_stalled());
        }

        scheduler.aborted_dependencies[0].lock().is_stalled = false;

        // Add 4 as dependency of 2 and get its stall removed.
        scheduler.aborted_dependencies[2]
            .lock()
            .stalled_deps
            .insert(4);
        assert_some_eq!(scheduler.start_executing(2).unwrap(), 0);
        assert_some_eq!(scheduler.start_executing(4).unwrap(), 0);
        assert_ok!(scheduler.txn_statuses.finish_execution(2, 0));
        assert_ok!(scheduler.txn_statuses.finish_execution(4, 0));
        // Propagate starts at 2 (does not call remove stall), but will call remove on 4.
        assert_ok_eq!(scheduler.txn_statuses.add_stall(4), true);
        assert!(scheduler.txn_statuses.get_status(4).is_stalled());
        scheduler.propagate(BTreeSet::from([2])).unwrap();
        assert!(!scheduler.aborted_dependencies[2].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[4].lock().is_stalled);
        assert!(!scheduler.txn_statuses.get_status(4).is_stalled());
    }

    fn stall_and_add_dependency(
        scheduler: &SchedulerV2,
        idx: TxnIndex,
        dep_idx: TxnIndex,
        num_stalls: usize,
    ) {
        assert!(num_stalls > 0);

        assert_some_eq!(scheduler.start_executing(dep_idx).unwrap(), 0);
        assert_ok!(scheduler.finish_execution(AbortManager::new(dep_idx, 0, scheduler)));
        assert_ok_eq!(scheduler.txn_statuses.add_stall(dep_idx), true);
        assert!(scheduler.txn_statuses.get_status(dep_idx).is_stalled());
        for _ in 1..num_stalls {
            assert_ok_eq!(scheduler.txn_statuses.add_stall(dep_idx), false);
        }
        assert!(scheduler.txn_statuses.get_status(dep_idx).is_stalled());

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
        assert_some_eq!(scheduler.start_executing(0).unwrap(), 0);

        scheduler.aborted_dependencies[0].lock().is_stalled = true;
        stall_and_add_dependency(&scheduler, 0, 2, 1);
        stall_and_add_dependency(&scheduler, 0, 3, 2);

        assert_ok!(scheduler.finish_execution(AbortManager::new(0, 0, &scheduler)));

        assert!(!scheduler.aborted_dependencies[0].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[2].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[3].lock().is_stalled);
        assert!(!scheduler.txn_statuses.get_status(0).is_stalled());
        assert!(!scheduler.txn_statuses.get_status(2).is_stalled());
        assert!(scheduler.txn_statuses.get_status(3).is_stalled());
        assert_ok_eq!(scheduler.txn_statuses.remove_stall(3), true);

        for i in 0..3 {
            assert_eq!(
                scheduler.committed_marker[i].load(Ordering::Relaxed),
                CommitMarkerFlag::NotCommitted as u8
            );
        }
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_abort_manager_invalidate() {
        let scheduler = SchedulerV2::new(10, 1);
        let mut abort_manager = AbortManager::new(2, 0, &scheduler);

        // Check initial state - no invalidations should be recorded
        assert!(abort_manager.invalidated_dependencies.is_empty());

        let scenario = FailScenario::setup();
        assert!(fail::has_failpoints());

        // Test invalidating lower version (error), start_abort (not called) via failpoint.
        fail::cfg("abort-manager-start-abort-none", "panic").unwrap();
        assert_err!(abort_manager.invalidate(1, 0));
        assert_err!(abort_manager.invalidate(2, 0)); // same version
        assert_err!(abort_manager.invalidate(0, 0));

        // Test case where start_abort returns None (simulating false)
        fail::cfg("abort-manager-start-abort-none", "return").unwrap();
        assert_ok!(abort_manager.invalidate(3, 0));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&3), &None);
        fail::remove("abort-manager-start-abort-none");
        // Make sure None can get replaced with an incarnation.
        fail::cfg("abort-manager-start-abort-some", "return").unwrap();
        assert_ok!(abort_manager.invalidate(3, 2));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&3), &Some(2));

        // Test case where start_abort returns Some(incarnation).
        assert_ok!(abort_manager.invalidate(4, 0));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&4), &Some(0));

        // Test occupied entry with Some value - error if lower incarnation start_aborted.
        fail::cfg("abort-manager-start-abort-some", "panic").unwrap();
        assert_err!(abort_manager.invalidate(4, 1));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&4), &Some(0));

        // Test invalidating with equal incarnation as stored - should be ignored.
        // Configure failpoint to panic but it shouldn't be called since incarnation matches.
        assert_ok!(abort_manager.invalidate(4, 0));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&4), &Some(0));
        fail::remove("abort-manager-start-abort");

        // Test multiple invalidations for different transactions
        fail::cfg("abort-manager-start-abort-some", "return").unwrap();
        assert_ok!(abort_manager.invalidate(5, 2));
        assert_ok!(abort_manager.invalidate(6, 4));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&5), &Some(2));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&6), &Some(4));
        fail::remove("abort-manager-start-abort-some");

        // Test that invalidations are preserved after multiple calls (in different order),
        // and that lower incarnations are ignored.
        fail::cfg("abort-manager-start-abort-some", "panic").unwrap();
        assert_ok!(abort_manager.invalidate(5, 1));
        assert_err!(abort_manager.invalidate(5, 4));
        assert_err!(abort_manager.invalidate(6, 6));
        assert_ok!(abort_manager.invalidate(6, 1));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&5), &Some(2));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&6), &Some(4));

        scenario.teardown();
    }

    #[test]
    fn test_abort_manager_take() {
        let scheduler = SchedulerV2::new(10, 1);
        let mut abort_manager = AbortManager::new(2, 0, &scheduler);

        // Set up failpoint before running test
        let scenario = FailScenario::setup();
        assert!(fail::has_failpoints());

        // Record some invalidations with specific failpoint configurations.
        fail::cfg("abort-manager-start-abort-none", "return").unwrap();
        assert_ok!(abort_manager.invalidate(3, 0));
        fail::remove("abort-manager-start-abort-none");
        fail::cfg("abort-manager-start-abort-some", "return").unwrap();
        assert_ok!(abort_manager.invalidate(4, 1));
        assert_ok!(abort_manager.invalidate(5, 2));

        // Verify the invalidations before taking them.
        assert_eq!(abort_manager.invalidated_dependencies.len(), 3);
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&3), &None);
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&4), &Some(1));
        assert_some_eq!(abort_manager.invalidated_dependencies.get(&5), &Some(2));

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

                    assert_some_eq!(scheduler.start_executing(idx).unwrap(), 0);
                    assert_ok!(scheduler.finish_execution(AbortManager::new(idx, 0, &scheduler)));
                });
            }
        });

        assert!(!scheduler.is_done());
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 0);
        assert_eq!(
            scheduler
                .txn_statuses
                .get_execution_queue_manager()
                .executed_once_max_idx
                .load(Ordering::Relaxed),
            num_txns
        );
        assert_eq!(scheduler.post_commit_processing_queue.len(), 0);

        for i in 0..num_txns {
            assert!(scheduler.txn_statuses.is_executed(i));

            assert_eq!(
                scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                CommitMarkerFlag::NotCommitted as u8
            );
            assert_err!(scheduler.end_commit(i));

            assert_some_eq!(scheduler.start_commit().unwrap(), (i, 0));
            assert_eq!(
                scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                CommitMarkerFlag::CommitStarted as u8
            );

            // Commit hook needs to complete for next one to be dispatched.
            if i != num_txns - 1 {
                // The check for commit flag is after next_to_commit_idx = num_txns
                // check which returns Ok(None).
                assert_err!(scheduler.start_commit());
            } else {
                assert_none!(scheduler.start_commit().unwrap());
            }
            assert_ok!(scheduler.end_commit(i));
            assert_eq!(
                scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                CommitMarkerFlag::Committed as u8
            );

            assert_err!(scheduler.end_commit(i));
        }

        assert_eq!(
            scheduler.post_commit_processing_queue.len(),
            num_txns as usize
        );

        assert!(scheduler.txn_statuses.is_executed(0));
        assert_ok_eq!(scheduler.post_commit_processing_queue.pop(), 0);

        for i in 1..num_txns {
            assert!(!scheduler.is_done());
            assert_ok_eq!(scheduler.next_task(0), TaskKind::PostCommitProcessing(i));
        }
        assert!(scheduler.is_done());
    }

    #[test]
    fn remove_stall_propagation_scenario() {
        let mut scheduler = SchedulerV2::new(10, 1);
        *scheduler.txn_statuses.get_status_mut(3) = ExecutionStatus::new_for_test(
            StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 5),
            0,
        );
        stall_and_add_dependency(&scheduler, 3, 5, 1);
        stall_and_add_dependency(&scheduler, 5, 7, 1);
        stall_and_add_dependency(&scheduler, 3, 8, 2);
        stall_and_add_dependency(&scheduler, 3, 6, 1);
        stall_and_add_dependency(&scheduler, 6, 9, 1);
        assert_ok_eq!(scheduler.txn_statuses.start_abort(6, 0), true);
        assert_ok!(scheduler.txn_statuses.finish_abort(6, 0, false));

        assert_ok!(scheduler.propagate(BTreeSet::from([3])));

        assert!(!scheduler.txn_statuses.get_status(3).is_stalled());
        assert!(!scheduler.txn_statuses.get_status(5).is_stalled());
        assert!(!scheduler.txn_statuses.get_status(7).is_stalled());
        assert!(scheduler.txn_statuses.get_status(8).is_stalled());
        assert!(!scheduler.txn_statuses.get_status(6).is_stalled());
        assert!(scheduler.txn_statuses.get_status(9).is_stalled());
        assert!(!scheduler.aborted_dependencies[0].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[5].lock().is_stalled);
        assert!(!scheduler.aborted_dependencies[7].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[8].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[6].lock().is_stalled);
        assert!(scheduler.aborted_dependencies[9].lock().is_stalled);
    }

    #[test]
    fn start_commit_simple() {
        let mut scheduler = SchedulerV2::new(10, 1);

        // Test txn index 0.
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 0);
        assert_none!(scheduler.start_commit().unwrap());
        // Next task should start executing (0, 0).
        assert_ok_eq!(scheduler.next_task(0), TaskKind::Execute(0, 0));
        assert_none!(scheduler.start_commit().unwrap());
        // After execution is finished, commit hook can be dispatched.
        assert_ok!(scheduler.finish_execution(AbortManager::new(0, 0, &scheduler)));
        assert_eq!(
            scheduler.committed_marker[0].load(Ordering::Relaxed),
            CommitMarkerFlag::NotCommitted as u8
        );

        assert_some_eq!(scheduler.start_commit().unwrap(), (0, 0));
        assert_eq!(scheduler.next_to_commit_idx.load(Ordering::Relaxed), 1);
        assert_eq!(
            scheduler.committed_marker[0].load(Ordering::Relaxed),
            CommitMarkerFlag::CommitStarted as u8
        );

        // Ok(None) because txn 1 has not finished execution yet.
        assert_none!(scheduler.start_commit().unwrap());
        scheduler.next_to_commit_idx.store(0, Ordering::Relaxed);
        // But calling it again with index 0 would lead to an error.
        assert_err!(scheduler.start_commit());

        scheduler.next_to_commit_idx.store(3, Ordering::Relaxed);
        // Execution status is checked first, so start_commit returns Ok(None).

        assert_none!(scheduler.start_commit().unwrap());

        *scheduler.txn_statuses.get_status_mut(3) = ExecutionStatus::new_for_test(
            StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 5),
            1,
        );
        // Now it is an error because the commit flag of txn 2 is not Committed.
        assert_err!(scheduler.start_commit());
        scheduler.committed_marker[2].store(CommitMarkerFlag::Committed as u8, Ordering::Relaxed);
        assert!(scheduler.txn_statuses.get_status(3).is_stalled());
        assert_eq!(
            scheduler.committed_marker[3].load(Ordering::Relaxed),
            CommitMarkerFlag::NotCommitted as u8
        );
        // No longer an error, but should commit despite being currently stalled.
        assert_some_eq!(scheduler.start_commit().unwrap(), (3, 5));

        scheduler.next_to_commit_idx.store(10, Ordering::Relaxed);
        scheduler.committed_marker[9].store(CommitMarkerFlag::Committed as u8, Ordering::Relaxed);
        assert_none!(scheduler.start_commit().unwrap());
    }

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
    #[test_case(1)]
    #[test_case(2)]
    #[test_case(4)]
    #[test_case(8)]
    #[test_case(16)]
    #[test_case(32)]
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
                                scheduler.start_commit().unwrap()
                            {
                                assert!(incarnation < 2);
                                assert!(scheduler.txn_statuses.is_executed(txn_idx));

                                assert_eq!(
                                    scheduler.committed_marker[txn_idx as usize]
                                        .load(Ordering::Relaxed),
                                    CommitMarkerFlag::CommitStarted as u8
                                );
                                scheduler.end_commit(txn_idx).unwrap();

                                if num_committed.fetch_add(1, Ordering::Relaxed) == num_txns / 2
                                    && seed & 7 == 0
                                {
                                    // Halt must occur after end_commit call (so we
                                    // do not miss a post-processing task in next_task).
                                    scheduler.halt();
                                }
                                assert_eq!(
                                    scheduler.committed_marker[txn_idx as usize]
                                        .load(Ordering::Relaxed),
                                    CommitMarkerFlag::Committed as u8
                                );
                            }

                            scheduler.commit_hooks_unlock();
                        }

                        match scheduler.next_task(0).unwrap() {
                            TaskKind::Execute(txn_idx, incarnation) => {
                                assert!(incarnation < 2);

                                hooks_taken[txn_idx as usize].store(true, Ordering::Relaxed);
                                let mut abort_manager =
                                    AbortManager::new(txn_idx, incarnation, &scheduler);

                                // Invalidate all hooks after this transaction
                                invalidate_after_index(
                                    &mut abort_manager,
                                    &mut hooks.lock(),
                                    txn_idx,
                                )
                                .unwrap();

                                let mut dep_ok = true;
                                if let Some(dep_idx) = txn_deps[txn_idx as usize] {
                                    if !scheduler.txn_statuses.ever_executed(dep_idx as u32) {
                                        hooks.lock().insert((txn_idx, incarnation));
                                        if hooks_taken[dep_idx].load(Ordering::Relaxed) {
                                            // Hook is not guaraneed to be executed - call abort itself.
                                            if assert_ok!(
                                                scheduler.start_abort(txn_idx, incarnation)
                                            ) {
                                                assert_ok!(scheduler.txn_statuses.finish_abort(
                                                    txn_idx,
                                                    incarnation,
                                                    false
                                                ));
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
                                    CommitMarkerFlag::Committed as u8
                                );
                                assert!(scheduler.txn_statuses.is_executed(txn_idx));
                            },
                            TaskKind::NextTask => {},
                            TaskKind::ModuleValidation(_, _, _) => {
                                unreachable!("Module validation task should not be scheduled");
                            },
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

                assert!(scheduler.txn_statuses.is_executed(i));
                assert_eq!(
                    scheduler.committed_marker[i as usize].load(Ordering::Relaxed),
                    CommitMarkerFlag::Committed as u8
                );
                // Eventually removing stalls should propagate to all dependencies.
                assert!(!scheduler.aborted_dependencies[i as usize].lock().is_stalled);
            }
        }
    }
}
