// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::explicit_sync_wrapper::ExplicitSyncWrapper;
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use concurrent_queue::{ConcurrentQueue, PopError};
use crossbeam::utils::CachePadded;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::{
    cmp::{max, min},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Condvar,
    },
};

const TXN_IDX_MASK: u64 = (1 << 32) - 1;

pub type Wave = u32;

#[derive(Debug)]
pub struct ArmedLock {
    // Last bit:   1 -> unlocked; 0 -> locked
    // Second bit: 1 -> there's work; 0 -> no work
    locked: AtomicU64,
}

impl ArmedLock {
    pub fn new() -> Self {
        Self {
            locked: AtomicU64::new(3),
        }
    }

    // try_lock succeeds when the lock is unlocked and armed (there is work to do).
    pub fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange_weak(3, 0, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    pub fn unlock(&self) {
        self.locked.fetch_or(1, Ordering::Release);
    }

    pub fn arm(&self) {
        self.locked.fetch_or(2, Ordering::Release);
    }
}

#[derive(Debug)]
pub enum DependencyStatus {
    // The dependency is not resolved yet.
    Unresolved,
    // The dependency is resolved.
    Resolved,
    // The parallel execution is halted.
    ExecutionHalted,
}
type DependencyCondvar = Arc<(Mutex<DependencyStatus>, Condvar)>;

// Return value of the function wait_for_dependency
#[derive(Debug)]
pub enum DependencyResult {
    Dependency(DependencyCondvar),
    Resolved,
    ExecutionHalted,
}

/// Two types of execution tasks: Execution and Wakeup.
/// Execution is a normal execution task, Wakeup is a task that just wakes up a suspended execution.
/// See explanations for the ExecutionStatus below.
#[derive(Debug, Clone)]
pub enum ExecutionTaskType {
    Execution,
    Wakeup(DependencyCondvar),
}

/// Task type that the parallel execution workers get from the scheduler.
#[derive(Debug)]
pub enum SchedulerTask {
    /// Execution task with a version of the transaction, and whether it's waking up an already
    /// executing worker (suspended / waiting on a dependency).
    ExecutionTask(TxnIndex, Incarnation, ExecutionTaskType),
    /// Validation task with a version of the transaction, and the validation wave information.
    ValidationTask(TxnIndex, Incarnation, Wave),
    /// Retry holds no task (similar None if we wrapped tasks in Option)
    Retry,
    /// Done implies that there are no more tasks and the scheduler is done.
    Done,
}

/////////////////////////////// Explanation for ExecutionStatus ///////////////////////////////
/// All possible execution status for each transaction. In the explanation below, we abbreviate
/// 'execution status' as 'status'. Each status contains the latest incarnation number,
/// where incarnation = i means it is the i-th execution instance of the transaction.
///
/// 'Ready' means that the corresponding incarnation should be executed and the scheduler
/// must eventually create a corresponding execution task. The scheduler ensures that exactly one
/// execution task gets created, changing the status to 'Executing' in the process. 'Ready' status
/// contains an ExecutionTaskType, which is either Execution or Wakeup. If it is Execution, then
/// the scheduler creates an execution task for the corresponding incarnation. If it is Wakeup,
/// a dependency condition variable is set in ExecutionTaskType::Wakeup(DependencyCondvar): an execution
/// of a prior incarnation is waiting on it with a read dependency resolved (when dependency was
/// encountered, the status changed to Suspended, and suspended changed to Ready when the dependency
/// finished its execution). In this case the caller need not create a new execution task, but
/// just notify the suspended execution via the dependency condition variable.
///
/// 'Executing' status of an incarnation turns into 'Executed' if the execution task finishes, or
/// if a dependency is encountered, it becomes 'Ready(incarnation)' once the
/// dependency is resolved. An 'Executed' status allows creation of validation tasks for the
/// corresponding incarnation, and a validation failure leads to an abort. The scheduler ensures
/// that there is exactly one abort, changing the status to 'Aborting' in the process. Once the
/// thread that successfully aborted performs everything that's required, it sets the status
/// to 'Ready(incarnation + 1)', allowing the scheduler to create an execution
/// task for the next incarnation of the transaction.
///
/// 'ExecutionHalted' is a transaction status marking that parallel execution is halted, due to
/// reasons such as module r/w intersection or exceeding per-block gas limit. It is safe to ignore
/// this status during the transaction invariant checks, e.g., suspend(), resume(), set_executed_status().
///
/// Status transition diagram:
/// Ready(i)                                                                               ---
///    |  try_incarnate (incarnate successfully)                                             |
///    |                                                                                     |
///    ↓         suspend (waiting on dependency)                resume                       |
/// Executing(i) -----------------------------> Suspended(i) ------------> Ready(i)          |
///    |                                                                                     | halt_transaction_execution
///    |  finish_execution                                                                   |-----------------> ExecutionHalted
///    ↓                                                                                     |
/// Executed(i) (pending for (re)validations) ---------------------------> Committed(i)      |
///    |                                                                                     |
///    |  try_abort (abort successfully)                                                     |
///    ↓                finish_abort                                                         |
/// Aborting(i) ---------------------------------------------------------> Ready(i+1)      ---
///
#[derive(Debug)]
enum ExecutionStatus {
    Ready(Incarnation, ExecutionTaskType),
    Executing(Incarnation, ExecutionTaskType),
    Suspended(Incarnation, DependencyCondvar),
    Executed(Incarnation),
    // TODO[agg_v2](cleanup): rename to Finalized or ReadyToCommit / CommitReady?
    // it gets committed later, without scheduler tracking.
    Committed(Incarnation),
    Aborting(Incarnation),
    // The bool in the ExecutionHalted tracks an useful invariant for block epilogue txn:
    // if the txn status when halted was Executing or Suspended.
    ExecutionHalted(bool),
}

impl PartialEq for ExecutionStatus {
    fn eq(&self, other: &Self) -> bool {
        use ExecutionStatus::*;
        match (self, other) {
            (
                &Ready(ref a, ExecutionTaskType::Execution),
                &Ready(ref b, ExecutionTaskType::Execution),
            )
            | (
                &Executing(ref a, ExecutionTaskType::Execution),
                &Executing(ref b, ExecutionTaskType::Execution),
            )
            | (
                &Ready(ref a, ExecutionTaskType::Wakeup(_)),
                &Ready(ref b, ExecutionTaskType::Wakeup(_)),
            )
            | (
                &Executing(ref a, ExecutionTaskType::Wakeup(_)),
                &Executing(ref b, ExecutionTaskType::Wakeup(_)),
            )
            | (&Suspended(ref a, _), &Suspended(ref b, _))
            | (&Executed(ref a), &Executed(ref b))
            | (&Committed(ref a), &Committed(ref b))
            | (&Aborting(ref a), &Aborting(ref b)) => a == b,
            _ => false,
        }
    }
}

/////////////////////////////// Explanation for ValidationStatus ///////////////////////////////
/// All possible validation status for each transaction. In the explanation below, we abbreviate
/// 'validation status' as 'status'. Each status contains three wave numbers, each with different
/// meanings, but in general the concept of 'wave' keeps track of the version number of the validation.
///
/// 'max_triggered_wave' records the maximum wave that was triggered at the transaction index, and
/// will be incremented every time when the validation_idx is decreased. Initialized as 0.
///
/// 'maybe_max_validated_wave' records the maximum wave among successful validations of the corresponding
/// transaction, will be incremented upon successful validation (finish_validation). Initialized as None.
///
/// 'required_wave' in addition records the wave that must be successfully validated in order
/// for the transaction to be committed, required to handle the case of the optimization in
/// finish_execution when only the transaction itself is validated (if last incarnation
/// didn't write outside of the previous write-set). Initialized as 0.
///
/// Other than ValidationStatus, the 'wave' information is also recorded in 'validation_idx' and 'commit_state'.
/// Below is the description of the wave meanings and how they are updated. More details can be
/// found in the definition of 'validation_idx' and 'commit_state'.
///
/// In 'validation_idx', the first 32 bits identifies a validation wave while the last 32 bits
/// contain an index that tracks the minimum of all transaction indices that require validation.
/// The wave is incremented whenever the validation_idx is reduced due to transactions requiring
/// validation, in particular, after aborts and executions that write outside of the write set of
/// the same transaction's previous incarnation.
///
/// In 'commit_state', the first element records the next transaction to commit, and the
/// second element records the lower bound on the wave of a validation that must be successful
/// in order to commit the next transaction. The wave is updated in try_commit, upon seeing an
/// executed txn with higher max_triggered_wave. Note that the wave is *not* updated with the
/// required_wave of the txn that is being committed.
///
///
/////////////////////////////// Algorithm Description for Updating Waves ///////////////////////////////
/// In the following, 'update' means taking the maximum.
/// (1) Upon decreasing validation_idx, increment validation_idx.wave and update txn's
/// max_triggered_wave <- validation_idx.wave;
/// (2) Upon finishing execution of txn that is below validation_idx, update txn's
/// required_wave <- validation_idx.wave; (otherwise, the last triggered wave is below and will validate).
/// (3) Upon validating a txn successfully, update txn's maybe_max_validated_wave <- validation_idx.wave;
/// (4) Upon trying to commit an executed txn, update commit_state.wave <- txn's max_triggered_wave.
/// (5) If txn's maybe_max_validated_wave >= max(commit_state.wave, txn's required_wave), can commit the txn.
///
/// Remark: commit_state.wave is updated only with max_triggered_wave but not required_wave. This is
/// because max_triggered_wave implies that this wave of validations was required for all higher transactions
/// (and is set as a part of decrease_validation_idx), while required_wave is set for the transaction only
/// (when a validation task is returned to the caller). Moreover, the code is structured in a way that
/// decrease_validation_idx is always called for txn_idx + 1 (e.g. when aborting, there is no need to validate
/// the transaction before re-execution, and in finish_execution, even if there is a need to validate txn_idx,
/// it is returned to the caller directly, which is done so as an optimization and also for uniformity).
#[derive(Debug)]
struct ValidationStatus {
    max_triggered_wave: Wave,
    required_wave: Wave,
    maybe_max_validated_wave: Option<Wave>,
}

impl ValidationStatus {
    pub fn new() -> Self {
        ValidationStatus {
            max_triggered_wave: 0,
            required_wave: 0,
            maybe_max_validated_wave: None,
        }
    }
}

pub trait TWaitForDependency {
    fn wait_for_dependency(
        &self,
        txn_idx: TxnIndex,
        dep_txn_idx: TxnIndex,
    ) -> Result<DependencyResult, PanicError>;
}

pub struct Scheduler {
    /// Number of txns to execute, immutable.
    num_txns: TxnIndex,

    /// An index i maps to indices of other transactions that depend on transaction i, i.e. they
    /// should be re-executed once transaction i's next incarnation finishes.
    txn_dependency: Vec<CachePadded<Mutex<Vec<TxnIndex>>>>,
    /// An index i maps to the most up-to-date status of transaction i.
    txn_status: Vec<CachePadded<(RwLock<ExecutionStatus>, RwLock<ValidationStatus>)>>,

    /// Next transaction to commit, and sweeping lower bound on the wave of a validation that must
    /// be successful in order to commit the next transaction.
    commit_state: CachePadded<ExplicitSyncWrapper<(TxnIndex, Wave)>>,

    // Note: with each thread reading both counters when deciding the next task, and being able
    // to choose either execution or validation task, separately padding these indices may increase
    // (real) cache invalidation traffic more than combat false sharing. Hence, currently we
    // don't pad separately, but instead put them in between two padded members (same cache line).
    // TODO: investigate the trade-off. Re-consider if we change task assignment logic (i.e. make
    // validation/execution preferences stick to the worker threads).
    /// A shared index that tracks the minimum of all transaction indices that require execution.
    /// The threads increment the index and attempt to create an execution task for the corresponding
    /// transaction, if the status of the txn is 'Ready'. This implements a counting-based
    /// concurrent ordered set. It is reduced as necessary when transactions become ready to be
    /// executed, in particular, when execution finishes and dependencies are resolved.
    execution_idx: AtomicU32,
    /// The first 32 bits identifies a validation wave while the last 32 bits contain an index
    /// that tracks the minimum of all transaction indices that require validation.
    /// The threads increment this index and attempt to create a validation task for the
    /// corresponding transaction (if the status of the txn is 'Executed'), associated with the
    /// observed wave in the first 32 bits. Each validation wave represents the sequence of
    /// validations that must happen due to the fixed serialization order of transactions.
    /// The index is reduced as necessary when transactions require validation, in particular,
    /// after aborts and executions that write outside of the write set of the same transaction's
    /// previous incarnation. This also creates a new wave of validations, identified by the
    /// monotonically increasing index stored in the first 32 bits.
    validation_idx: AtomicU64,

    /// Shared marker that is set when a thread detects that all txns can be committed.
    done_marker: CachePadded<AtomicBool>,

    has_halted: CachePadded<AtomicBool>,

    queueing_commits_lock: CachePadded<ArmedLock>,

    commit_queue: ConcurrentQueue<u32>,
}

/// Public Interfaces for the Scheduler
impl Scheduler {
    pub fn new(num_txns: TxnIndex) -> Self {
        // Empty block should early return and not create a scheduler.
        assert!(num_txns > 0, "No scheduler needed for 0 transactions");

        Self {
            num_txns,
            txn_dependency: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(Vec::new())))
                .collect(),
            txn_status: (0..num_txns)
                .map(|_| {
                    CachePadded::new((
                        RwLock::new(ExecutionStatus::Ready(0, ExecutionTaskType::Execution)),
                        RwLock::new(ValidationStatus::new()),
                    ))
                })
                .collect(),
            commit_state: CachePadded::new(ExplicitSyncWrapper::new((0, 0))),
            execution_idx: AtomicU32::new(0),
            validation_idx: AtomicU64::new(0),
            done_marker: CachePadded::new(AtomicBool::new(false)),
            has_halted: CachePadded::new(AtomicBool::new(false)),
            queueing_commits_lock: CachePadded::new(ArmedLock::new()),
            commit_queue: ConcurrentQueue::<u32>::bounded(num_txns as usize),
        }
    }

    pub fn add_to_commit_queue(&self, txn_idx: u32) {
        self.commit_queue
            .push(txn_idx)
            .expect("Pushing to the commit_queue should never fail");
    }

    pub fn pop_from_commit_queue(&self) -> Result<u32, PopError> {
        self.commit_queue.pop()
    }

    pub fn queueing_commits_mark_done(&self) {
        self.queueing_commits_lock.unlock()
    }

    pub fn queueing_commits_arm(&self) {
        self.queueing_commits_lock.arm()
    }

    pub fn should_coordinate_commits(&self) -> bool {
        self.queueing_commits_lock.try_lock()
    }

    /// If successful, returns Some(TxnIndex), the index of committed transaction.
    pub fn try_commit(&self) -> Option<(TxnIndex, Incarnation)> {
        let mut commit_state = self.commit_state.acquire();
        let (commit_idx, commit_wave) = commit_state.dereference_mut();

        if *commit_idx == self.num_txns {
            return None;
        }

        let validation_status = self.txn_status[*commit_idx as usize].1.read();

        // Acquired the validation status read lock.
        if let Some(status) = self.txn_status[*commit_idx as usize]
            .0
            .try_upgradable_read()
        {
            // Acquired the execution status read lock, which can be upgrade to write lock if necessary.
            if let ExecutionStatus::Executed(incarnation) = *status {
                // Status is executed and we are holding the lock.

                // Note we update the wave inside commit_state only with max_triggered_wave,
                // since max_triggered_wave records the new wave when validation index is
                // decreased thus affecting all later txns as well,
                // while required_wave only records the new wave for one single txn.
                *commit_wave = max(*commit_wave, validation_status.max_triggered_wave);
                if let Some(validated_wave) = validation_status.maybe_max_validated_wave {
                    if validated_wave >= max(*commit_wave, validation_status.required_wave) {
                        let mut status_write = RwLockUpgradableReadGuard::upgrade(status);
                        // Upgrade the execution status read lock to write lock.
                        // Can commit.
                        *status_write = ExecutionStatus::Committed(incarnation);

                        *commit_idx += 1;
                        if *commit_idx == self.num_txns {
                            // All txns have been committed, the parallel execution can finish.
                            self.done_marker.store(true, Ordering::SeqCst);
                        }
                        return Some((*commit_idx - 1, incarnation));
                    }
                }
            }

            // Transaction needs to be at least [re]validated, and possibly also executed.
            // Once that happens, we will `arm` the queueing_commit.
            // Concurrency correctness - Both locks are held here.
            return None;
        }

        // Re-arm to try commit again.
        self.queueing_commits_arm();

        None
    }

    #[cfg(test)]
    /// Return the TxnIndex and Wave of current commit index
    pub fn commit_state(&self) -> (TxnIndex, u32) {
        let commit_state = self.commit_state.dereference();
        (commit_state.0, commit_state.1)
    }

    pub(crate) fn must_finish_execution_if_halted(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        let mut status = self.txn_status[txn_idx as usize].0.write();
        if let ExecutionStatus::ExecutionHalted(safely_finished) = &mut *status {
            // Even though the status is halted at txn_idx, the work to process and apply the outputs
            // must complete (to guarantee update to the shared data structures are applied correctly).
            // Hence, finish_execution must be called on the txn_idx, or prepare_for_block_epilogue
            // will return a PanicError.
            *safely_finished = false;
        }
        Ok(())
    }

    pub(crate) fn prepare_for_block_epilogue(
        &self,
        block_epilogue_idx: TxnIndex,
    ) -> Result<Incarnation, PanicError> {
        if block_epilogue_idx == self.num_txns {
            return Ok(0);
        }

        let mut status = self.txn_status[block_epilogue_idx as usize].0.write();
        if let ExecutionStatus::ExecutionHalted(safely_finished) = *status {
            if !safely_finished {
                return Err(code_invariant_error(format!(
                    "Status at block epilogue txn {} not safely finished after ExecutionHalted but not finished",
                    block_epilogue_idx
                )));
            }
        } else {
            return Err(code_invariant_error(format!(
                "Status {:?} at block epilogue txn {} not ExecutionHalted",
                &*status, block_epilogue_idx
            )));
        }

        *status = ExecutionStatus::Ready(1, ExecutionTaskType::Execution);
        Ok(1)
    }

    /// Try to abort version = (txn_idx, incarnation), called upon validation failure.
    /// When the invocation manages to update the status of the transaction, it changes
    /// Executed(incarnation) => Aborting(incarnation), it returns true. Otherwise,
    /// returns false. Since incarnation numbers never decrease, this also ensures
    /// that the same version may not successfully abort more than once.
    pub fn try_abort(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        // lock the execution status.
        // Note: we could upgradable read, then upgrade and write. Similar for other places.
        // However, it is likely an overkill (and overhead to actually upgrade),
        // while unlikely there would be much contention on a specific index lock.
        let mut status = self.txn_status[txn_idx as usize].0.write();

        if *status == ExecutionStatus::Executed(incarnation) {
            *status = ExecutionStatus::Aborting(incarnation);
            true
        } else {
            false
        }
    }

    /// Return the next task for the thread.
    pub fn next_task(&self) -> SchedulerTask {
        loop {
            if self.done() {
                // No more tasks.
                return SchedulerTask::Done;
            }

            let (idx_to_validate, wave) =
                Self::unpack_validation_idx(self.validation_idx.load(Ordering::Acquire));

            let idx_to_execute = self.execution_idx.load(Ordering::Acquire);

            let prefer_validate = idx_to_validate < min(idx_to_execute, self.num_txns)
                && !self.never_executed(idx_to_validate);

            if !prefer_validate && idx_to_execute >= self.num_txns {
                return SchedulerTask::Retry;
            }

            if prefer_validate {
                if let Some((txn_idx, incarnation, wave)) =
                    self.try_validate_next_version(idx_to_validate, wave)
                {
                    return SchedulerTask::ValidationTask(txn_idx, incarnation, wave);
                }
            }

            if idx_to_execute < self.num_txns {
                if let Some((txn_idx, incarnation, execution_task_type)) =
                    self.try_execute_next_version()
                {
                    return SchedulerTask::ExecutionTask(txn_idx, incarnation, execution_task_type);
                }
            }
        }
    }

    pub fn finish_validation(&self, txn_idx: TxnIndex, wave: Wave) {
        let mut validation_status = self.txn_status[txn_idx as usize].1.write();
        validation_status.maybe_max_validated_wave = Some(
            validation_status
                .maybe_max_validated_wave
                .map_or(wave, |prev_wave| max(prev_wave, wave)),
        );
    }

    fn wake_dependencies_after_execution(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        let txn_deps: Vec<TxnIndex> = {
            let mut stored_deps = self.txn_dependency[txn_idx as usize].lock();
            // Holding the lock, take dependency vector.
            std::mem::take(&mut stored_deps)
        };

        // Mark dependencies as resolved and find the minimum index among them.
        let mut min_dep = None;
        for dep in txn_deps {
            self.resume(dep)?;

            if min_dep.is_none() || min_dep.is_some_and(|min_dep| min_dep > dep) {
                min_dep = Some(dep);
            }
        }
        if let Some(execution_target_idx) = min_dep {
            // Decrease the execution index as necessary to ensure resolved dependencies
            // get a chance to be re-executed.
            self.execution_idx
                .fetch_min(execution_target_idx, Ordering::SeqCst);
        }
        Ok(())
    }

    /// After txn is executed, schedule its dependencies for re-execution.
    /// If revalidate_suffix is true, decrease validation_idx to schedule all higher transactions
    /// for (re-)validation. Otherwise, in some cases (if validation_idx not already lower),
    /// return a validation task of the transaction to the caller (otherwise Retry).
    pub fn finish_execution(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        revalidate_suffix: bool,
    ) -> Result<SchedulerTask, PanicError> {
        // Note: It is preferable to hold the validation lock throughout the finish_execution,
        // in particular before updating execution status. The point was that we don't want
        // any validation to come before the validation status is correspondingly updated.
        // It may be possible to reduce granularity, but shouldn't make performance difference
        // and like this correctness argument is much easier to see, which is also why we grab
        // the write lock directly, and never release it during the whole function. This way,
        // even validation status readers have to wait if they somehow end up at the same index.
        let mut validation_status = self.txn_status[txn_idx as usize].1.write();
        self.set_executed_status(txn_idx, incarnation)?;

        self.wake_dependencies_after_execution(txn_idx)?;

        let (cur_val_idx, mut cur_wave) =
            Self::unpack_validation_idx(self.validation_idx.load(Ordering::Acquire));

        // Needs to be re-validated in a new wave
        if cur_val_idx > txn_idx {
            if revalidate_suffix {
                // The transaction execution required revalidating all higher txns (not
                // only itself), currently happens when incarnation writes to a new path
                // (w.r.t. the write-set of its previous completed incarnation).
                if let Some(wave) = self.decrease_validation_idx(txn_idx + 1) {
                    cur_wave = wave;
                };
            }
            // Update the minimum wave this txn needs to pass.
            validation_status.required_wave = cur_wave;
            return Ok(SchedulerTask::ValidationTask(
                txn_idx,
                incarnation,
                cur_wave,
            ));
        }

        Ok(SchedulerTask::Retry)
    }

    /// Wakes up dependencies of the specified transaction, and decreases validation index so that
    /// all transactions above are re-validated.
    pub fn wake_dependencies_and_decrease_validation_idx(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        // We have exclusivity on this transaction.
        self.wake_dependencies_after_execution(txn_idx)?;

        // We skipped decreasing validation index when invalidating, as we were
        // executing it immediately, and are doing so now (unconditionally).
        self.decrease_validation_idx(txn_idx + 1);

        Ok(())
    }

    /// Finalize a validation task of version (txn_idx, incarnation). In some cases,
    /// may return a re-execution task back to the caller (otherwise, Retry).
    pub fn finish_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<SchedulerTask, PanicError> {
        {
            // acquire exclusive lock on the validation status of txn_idx, and hold the lock
            // while calling decrease_validation_idx below. Otherwise, this thread might get
            // suspended after setting aborted ( = ready) status, and other threads might finish
            // re-executing, then commit txn_idx, and potentially commit txn_idx + 1 before
            // decrease_validation_idx would be able to set max_triggered_wave.
            //
            // Also, as a convention, we always acquire validation status lock before execution
            // status lock, as we have to have a consistent order and this order is easier to
            // provide correctness between finish_execution & try_commit.
            let _validation_status = self.txn_status[txn_idx as usize].1.write();

            self.set_aborted_status(txn_idx, incarnation)?;

            // Schedule higher txns for validation, skipping txn_idx itself (needs to be
            // re-executed first).
            self.decrease_validation_idx(txn_idx + 1);

            // Can release the lock early.
        }

        // txn_idx must be re-executed, and if execution_idx is lower, it will be.
        if self.execution_idx.load(Ordering::Acquire) > txn_idx {
            // Optimization: execution_idx is higher than txn_idx, but decreasing it may
            // lead to wasted work for all indices between txn_idx and execution_idx.
            // Instead, attempt to create a new incarnation and return the corresponding
            // re-execution task back to the caller. If incarnation fails, there is
            // nothing to do, as another thread must have succeeded to incarnate and
            // obtain the task for re-execution.
            if let Some((new_incarnation, execution_task_type)) = self.try_incarnate(txn_idx) {
                return Ok(SchedulerTask::ExecutionTask(
                    txn_idx,
                    new_incarnation,
                    execution_task_type,
                ));
            }
        }

        Ok(SchedulerTask::Retry)
    }

    /// This function can halt the BlockSTM early, even if there are unfinished tasks.
    /// It will set the done_marker to be true, and resolve all pending dependencies.
    ///
    /// Currently, the reasons for halting the scheduler are as follows:
    /// 1. There is a module publishing txn that has read/write intersection with any txns
    ///    even during speculative execution.
    /// 2. There is a resource group serialization error.
    /// 3. There is a txn with VM execution status Abort.
    /// 4. There is a txn with VM execution status SkipRest.
    /// 5. The committed txns have exceeded the PER_BLOCK_GAS_LIMIT.
    /// 6. All transactions have been committed.
    ///
    /// For scenarios 1, 2 & 3, the output of the block execution will be an error, leading
    /// to a fallback with sequential execution. For scenarios 4, 5 & 6, execution outputs
    /// of the committed txn prefix will be returned from block execution.
    pub(crate) fn halt(&self) -> bool {
        // The first thread that sets done_marker to be true will be responsible for
        // resolving the conditional variables, to help other theads that may be pending
        // on the read dependency. See the comment of the function halt_transaction_execution().
        if !self.done_marker.swap(true, Ordering::SeqCst) {
            for txn_idx in 0..self.num_txns {
                self.halt_transaction_execution(txn_idx);
            }
        }

        !self.has_halted.swap(true, Ordering::SeqCst)
    }

    #[inline]
    pub(crate) fn has_halted(&self) -> bool {
        self.has_halted.load(Ordering::Relaxed)
    }
}

impl TWaitForDependency for Scheduler {
    /// When a txn depends on another txn, adds it to the dependency list of the other txn.
    /// Returns true if successful, or false, if the dependency got resolved in the meantime.
    /// If true is returned, Scheduler guarantees that later (dep_txn_idx will finish execution)
    /// transaction txn_idx will be resumed, and corresponding execution task created.
    /// If false is returned, it is caller's responsibility to repeat the read that caused the
    /// dependency and continue the ongoing execution of txn_idx.
    #[allow(clippy::literal_string_with_formatting_args)]
    fn wait_for_dependency(
        &self,
        txn_idx: TxnIndex,
        dep_txn_idx: TxnIndex,
    ) -> Result<DependencyResult, PanicError> {
        if txn_idx <= dep_txn_idx || dep_txn_idx >= self.num_txns {
            return Err(code_invariant_error(
                "In wait_for_dependency: {txn_idx} > {dep_txn_idx}, num txns = {self.num_txns}",
            ));
        }

        // Note: Could pre-check that txn dep_txn_idx isn't in an executed state, but the caller
        // usually has just observed the read dependency.

        // Create a condition variable associated with the dependency.
        let dep_condvar = Arc::new((Mutex::new(DependencyStatus::Unresolved), Condvar::new()));

        let mut stored_deps = self.txn_dependency[dep_txn_idx as usize].lock();

        // Note: is_executed & suspend calls acquire (a different, status) mutex, while holding
        // (dependency) mutex. This is the only place in scheduler where a thread may hold > 1
        // mutexes. Thus, acquisitions always happen in the same order (here), may not deadlock.

        if self.is_executed(dep_txn_idx, true).is_some() {
            // Current status of dep_txn_idx is 'executed' (or even committed), so the dependency
            // got resolved. To avoid zombie dependency (and losing liveness), must return here
            // and not add a (stale) dependency.

            // Note: acquires (a different, status) mutex, while holding (dependency) mutex.
            // For status lock this only happens here, thus the order is always higher index to lower.
            return Ok(DependencyResult::Resolved);
        }

        // If the execution is already halted, suspend will return false.
        // The synchronization is guaranteed by the Mutex around txn_status.
        // If the execution is halted, the first finishing thread will first set the status of each txn
        // to be ExecutionHalted, then notify the conditional variable. So if a thread sees ExecutionHalted,
        // it knows the execution is halted and it can return; otherwise, the finishing thread will notify
        // the conditional variable later and awake the pending thread.
        if !self.suspend(txn_idx, dep_condvar.clone())? {
            return Ok(DependencyResult::ExecutionHalted);
        }

        // Safe to add dependency here (still holding the lock) - finish_execution of txn
        // dep_txn_idx is guaranteed to acquire the same lock later and clear the dependency.
        stored_deps.push(txn_idx);

        // Stored deps gets unlocked here.

        Ok(DependencyResult::Dependency(dep_condvar))
    }
}

/// Private functions of the Scheduler
impl Scheduler {
    /// Helper function to be called from Scheduler::halt(); Sets the transaction status to Halted and
    /// notifies the waiting thread, if applicable. The guarantee is that if halt(txn_idx) is called,
    /// then no thread can remain suspended on some dependency while executing transaction txn_idx.
    ///
    /// Proof sketch as a result of code invariants that can be checked:
    /// 1. Status is replaced with ExecutionHalted, and ExecutionHalted status can never change.
    /// 2. In order for wait_for_dependency by txn_idx to return a CondVar to wait on, suspend must
    ///    be successful, which implies the status at that point may not be ExecutionHalted and that
    ///    the status at that point would be set to Suspended(_, CondVar).
    /// 3. Suspended status can turn into Ready or Executing, all containing the CondVar, unless a
    ///    worker with a ExecutionTaskType::WakeUp actually wakes up the suspending thread.
    /// 4. The waking up consists of acquiring the CondVar lock, setting the status to ExecutionHalted
    ///    or Resolved (if the worker with WakeUp task did it), and also calling notify_one. This
    ///    ensures that a thread that waits until the condition variable changes from Unresolved will
    ///    get released in all cases.
    fn halt_transaction_execution(&self, txn_idx: TxnIndex) {
        let mut status = self.txn_status[txn_idx as usize].0.write();

        // Always replace the status.
        match std::mem::replace(&mut *status, ExecutionStatus::ExecutionHalted(true)) {
            ExecutionStatus::Suspended(_, condvar)
            | ExecutionStatus::Ready(_, ExecutionTaskType::Wakeup(condvar))
            | ExecutionStatus::Executing(_, ExecutionTaskType::Wakeup(condvar)) => {
                // Condvar lock must always be taken inner-most.
                let (lock, cvar) = &*condvar;

                let mut lock = lock.lock();
                *lock = DependencyStatus::ExecutionHalted;
                cvar.notify_one();
            },
            ExecutionStatus::Executing(_, _) | ExecutionStatus::Aborting(_) => {
                *status = ExecutionStatus::ExecutionHalted(false);
            },
            _ => (),
        }
    }

    fn unpack_validation_idx(validation_idx: u64) -> (TxnIndex, Wave) {
        (
            (validation_idx & TXN_IDX_MASK) as TxnIndex,
            (validation_idx >> 32) as Wave,
        )
    }

    fn pack_into_validation_index(idx: TxnIndex, wave: Wave) -> u64 {
        (idx as u64) | ((wave as u64) << 32)
    }

    fn next_validation_index(idx: u64) -> u64 {
        idx + 1
    }

    /// Decreases the validation index, adjusting the wave and validation status as needed.
    fn decrease_validation_idx(&self, target_idx: TxnIndex) -> Option<Wave> {
        // We only call with txn_idx + 1, so it can equal num_txns, but not be strictly larger.
        assert!(target_idx <= self.num_txns);
        if target_idx == self.num_txns {
            return None;
        }

        if let Ok(prev_val_idx) =
            self.validation_idx
                .fetch_update(Ordering::SeqCst, Ordering::Acquire, |val_idx| {
                    let (txn_idx, wave) = Self::unpack_validation_idx(val_idx);
                    if txn_idx > target_idx {
                        let mut validation_status = self.txn_status[target_idx as usize].1.write();
                        // Update the minimum wave all the suffix txn needs to pass.
                        // We set it to max for safety (to avoid overwriting with lower values
                        // by a slower thread), but currently this isn't strictly required
                        // as all callers of decrease_validation_idx hold a write lock on the
                        // previous transaction's validation status.
                        validation_status.max_triggered_wave =
                            max(validation_status.max_triggered_wave, wave + 1);

                        Some(Self::pack_into_validation_index(target_idx, wave + 1))
                    } else {
                        None
                    }
                })
        {
            let (_, wave) = Self::unpack_validation_idx(prev_val_idx);
            // Note that 'wave' is the previous wave value, and we must update it to 'wave + 1'.
            Some(wave + 1)
        } else {
            None
        }
    }

    /// Try and incarnate a transaction. Only possible when the status is
    /// Ready(incarnation), in which case Some(incarnation) is returned and the
    /// status is (atomically, due to the mutex) updated to Executing(incarnation).
    /// An unsuccessful incarnation returns None. Since incarnation numbers never decrease
    /// for each transaction, incarnate function may not succeed more than once per version.
    fn try_incarnate(&self, txn_idx: TxnIndex) -> Option<(Incarnation, ExecutionTaskType)> {
        if txn_idx >= self.num_txns {
            return None;
        }

        // Note: we could upgradable read, then upgrade and write. Similar for other places.
        // However, it is likely an overkill (and overhead to actually upgrade),
        // while unlikely there would be much contention on a specific index lock.
        let mut status = self.txn_status[txn_idx as usize].0.write();
        if let ExecutionStatus::Ready(incarnation, execution_task_type) = &*status {
            let ret: (u32, ExecutionTaskType) = (*incarnation, (*execution_task_type).clone());
            *status = ExecutionStatus::Executing(*incarnation, (*execution_task_type).clone());
            Some(ret)
        } else {
            None
        }
    }

    /// If the status of transaction is Executed(incarnation), returns Some(incarnation),
    /// Useful to determine when a transaction can be validated, and to avoid a race in
    /// dependency resolution.
    /// If include_committed is true (which is when calling from wait_for_dependency),
    /// then committed transaction is also considered executed (for dependency resolution
    /// purposes). If include_committed is false (which is when calling from
    /// try_validate_next_version), then we are checking if a transaction may be validated,
    /// and a committed (in between) txn does not need to be scheduled for validation -
    /// so can return None.
    fn is_executed(&self, txn_idx: TxnIndex, include_committed: bool) -> Option<Incarnation> {
        let status = self.txn_status[txn_idx as usize].0.read();
        match *status {
            ExecutionStatus::Executed(incarnation) => Some(incarnation),
            ExecutionStatus::Committed(incarnation) => {
                if include_committed {
                    // Committed txns are also considered executed for dependency resolution purposes.
                    Some(incarnation)
                } else {
                    // Committed txns do not need to be scheduled for validation in try_validate_next_version.
                    None
                }
            },
            _ => None,
        }
    }

    /// Returns true iff no incarnation (even the 0-th one) has set the executed status, i.e.
    /// iff the execution status is READY_TO_EXECUTE/EXECUTING/SUSPENDED for incarnation 0.
    fn never_executed(&self, txn_idx: TxnIndex) -> bool {
        let status = self.txn_status[txn_idx as usize].0.read();
        matches!(
            *status,
            ExecutionStatus::Ready(0, _)
                | ExecutionStatus::Executing(0, _)
                | ExecutionStatus::Suspended(0, _)
        )
    }

    /// Grab an index to try and validate next (by fetch-and-incrementing validation_idx).
    /// - If the index is out of bounds, return None (and invoke a check of whether
    /// all txns can be committed).
    /// - If the transaction is ready for validation (EXECUTED state), return the version
    /// to the caller.
    /// - Otherwise, return None.
    fn try_validate_next_version(
        &self,
        idx_to_validate: TxnIndex,
        wave: Wave,
    ) -> Option<(TxnIndex, Incarnation, Wave)> {
        // We do compare-and-swap here instead of fetch-and-increment as for execution index
        // because we would like to not validate transactions when lower indices are in the
        // 'never_executed' state (to avoid unnecessarily reducing validation index and creating
        // redundant validation tasks). This is checked in the caller (in 'next_task' function),
        // but if we used fetch-and-increment, two threads can arrive in a cloned state and
        // both increment, effectively skipping over the 'never_executed' transaction index.
        let curr_validation_idx = Self::pack_into_validation_index(idx_to_validate, wave);
        let next_validation_idx = Self::next_validation_index(curr_validation_idx);
        if self
            .validation_idx
            .compare_exchange(
                curr_validation_idx,
                next_validation_idx,
                Ordering::SeqCst,
                Ordering::Acquire,
            )
            .is_ok()
        {
            // Successfully claimed idx_to_validate to attempt validation.
            // If incarnation was last executed, and thus ready for validation,
            // return version and wave for validation task, otherwise None.
            return self
                .is_executed(idx_to_validate, false)
                .map(|incarnation| (idx_to_validate, incarnation, wave));
        }

        None
    }

    /// Grab an index to try and execute next (by fetch-and-incrementing execution_idx).
    /// - If the index is out of bounds, return None (and invoke a check of whether
    /// all txns can be committed).
    /// - If the transaction is ready for execution (Ready state), attempt
    /// to create the next incarnation (should happen exactly once), and if successful,
    /// return the version to the caller for the corresponding ExecutionTask.
    /// - Otherwise, return None.
    fn try_execute_next_version(&self) -> Option<(TxnIndex, Incarnation, ExecutionTaskType)> {
        let idx_to_execute = self.execution_idx.fetch_add(1, Ordering::SeqCst);

        if idx_to_execute >= self.num_txns {
            return None;
        }

        // If successfully incarnated (changed status from ready to executing),
        // return version for execution task, otherwise None.
        self.try_incarnate(idx_to_execute)
            .map(|(incarnation, execution_task_type)| {
                (idx_to_execute, incarnation, execution_task_type)
            })
    }

    /// Put a transaction in a suspended state, with a condition variable that can be
    /// used to wake it up after the dependency is resolved.
    /// Return true when the txn is successfully suspended.
    /// Return false when the execution is halted.
    fn suspend(
        &self,
        txn_idx: TxnIndex,
        dep_condvar: DependencyCondvar,
    ) -> Result<bool, PanicError> {
        let mut status = self.txn_status[txn_idx as usize].0.write();
        match *status {
            ExecutionStatus::Executing(incarnation, _) => {
                *status = ExecutionStatus::Suspended(incarnation, dep_condvar);
                Ok(true)
            },
            ExecutionStatus::ExecutionHalted(_) => Ok(false),
            _ => Err(code_invariant_error(format!(
                "Unexpected status {:?} in suspend",
                &*status,
            ))),
        }
    }

    /// When a dependency is resolved, mark the transaction as Ready.
    /// The caller must ensure that the transaction is in the Suspended state.
    fn resume(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        let mut status = self.txn_status[txn_idx as usize].0.write();
        match &*status {
            ExecutionStatus::Suspended(incarnation, dep_condvar) => {
                *status = ExecutionStatus::Ready(
                    *incarnation,
                    ExecutionTaskType::Wakeup(dep_condvar.clone()),
                );
                Ok(())
            },
            ExecutionStatus::ExecutionHalted(_) => Ok(()),
            _ => Err(code_invariant_error(format!(
                "Unexpected status {:?} in resume",
                &*status,
            ))),
        }
    }

    /// Set status of the transaction to Executed(incarnation).
    fn set_executed_status(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<(), PanicError> {
        let mut status = self.txn_status[txn_idx as usize].0.write();
        match &mut *status {
            ExecutionStatus::Executing(stored_incarnation, _)
                if *stored_incarnation == incarnation =>
            {
                *status = ExecutionStatus::Executed(incarnation);
                Ok(())
            },
            ExecutionStatus::ExecutionHalted(safely_finished) => {
                *safely_finished = true;
                // The execution is already halted.
                Ok(())
            },
            _ => Err(code_invariant_error(format!(
                "Expected Executing incarnation {incarnation}, got {:?}",
                &*status,
            ))),
        }
    }

    /// After a successful abort, mark the transaction as ready for re-execution with
    /// an incremented incarnation number.
    fn set_aborted_status(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<(), PanicError> {
        let mut status = self.txn_status[txn_idx as usize].0.write();
        match &mut *status {
            ExecutionStatus::Aborting(stored_incarnation) if *stored_incarnation == incarnation => {
                *status = ExecutionStatus::Ready(incarnation + 1, ExecutionTaskType::Execution);
                Ok(())
            },
            ExecutionStatus::ExecutionHalted(safely_finished) => {
                *safely_finished = true;
                // The execution is already halted.
                Ok(())
            },
            _ => Err(code_invariant_error(format!(
                "Expected Aborting incarnation {incarnation}, got {:?}",
                &*status,
            ))),
        }
    }

    /// Checks whether the done marker is set. The marker can only be set by 'try_commit'.
    fn done(&self) -> bool {
        self.done_marker.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_matches, assert_ok, assert_ok_eq, assert_some};

    #[test]
    fn scheduler_halt() {
        let s = Scheduler::new(5);
        assert!(!s.done());
        assert!(s.halt());
        assert!(s.done());
        assert!(!s.halt());
    }

    #[test]
    fn scheduler_halt_status() {
        let s = Scheduler::new(5);
        for i in 0..5 {
            s.try_incarnate(i);
        }
        let dep_arc = |wait_result| -> DependencyCondvar {
            match wait_result {
                Ok(DependencyResult::Dependency(dep_arc)) => dep_arc,
                _ => unreachable!("Must return a dependency {:?}", wait_result),
            }
        };

        let dep_1 = dep_arc(s.wait_for_dependency(1, 0));
        let dep_2 = dep_arc(s.wait_for_dependency(2, 0));
        // Check wait for dependency error conditions w. indices (correct statuses).
        assert_err!(s.wait_for_dependency(3, 3));
        assert_err!(s.wait_for_dependency(6, 5));
        let dep_3 = dep_arc(s.wait_for_dependency(3, 0));
        assert_ok!(s.resume(2));
        assert_ok!(s.resume(3));
        assert_some!(s.try_incarnate(3));

        assert_matches!(&*dep_1.0.lock(), DependencyStatus::Unresolved);
        assert_matches!(&*dep_2.0.lock(), DependencyStatus::Unresolved);
        assert_matches!(&*dep_3.0.lock(), DependencyStatus::Unresolved);
        s.halt();
        assert_matches!(&*dep_1.0.lock(), DependencyStatus::ExecutionHalted);
        assert_matches!(&*dep_2.0.lock(), DependencyStatus::ExecutionHalted);
        assert_matches!(&*dep_3.0.lock(), DependencyStatus::ExecutionHalted);

        assert_ok_eq!(
            s.suspend(
                1,
                Arc::new((Mutex::new(DependencyStatus::Unresolved), Condvar::new()))
            ),
            false
        );
    }

    #[test]
    fn scheduler_panic_error() {
        let s = Scheduler::new(2);
        assert_err!(s.suspend(
            0,
            Arc::new((Mutex::new(DependencyStatus::Unresolved), Condvar::new()))
        ));
        assert_err!(s.resume(0));
        assert_err!(s.set_executed_status(0, 0));
        assert_err!(s.set_aborted_status(0, 0));
        assert_err!(s.wait_for_dependency(1, 0));
    }
}
