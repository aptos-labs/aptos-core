// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use crossbeam::utils::CachePadded;
use std::{
    cmp::min,
    hint,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar,
    },
};

// Type aliases.
pub type TxnIndex = usize;
pub type Incarnation = usize;
pub type Version = (TxnIndex, Incarnation);
type DependencyCondvar = Arc<(Mutex<bool>, Condvar)>;

// A struct to track the number of active tasks in the scheduler using RAII.
pub struct TaskGuard<'a> {
    counter: &'a AtomicUsize,
}

impl<'a> TaskGuard<'a> {
    pub fn new(counter: &'a AtomicUsize) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self { counter }
    }
}

impl Drop for TaskGuard<'_> {
    fn drop(&mut self) {
        assert!(self.counter.fetch_sub(1, Ordering::SeqCst) > 0);
    }
}

/// A holder for potential task returned from the Scheduler. ExecutionTask and ValidationTask
/// each contain a version of transaction that must be executed or validated, respectively.
/// NoTask holds no task (similar None if we wrapped tasks in Option), and Done implies that
/// there are no more tasks and the scheduler is done.
pub enum SchedulerTask<'a> {
    ExecutionTask(Version, Option<DependencyCondvar>, TaskGuard<'a>),
    ValidationTask(Version, TaskGuard<'a>),
    NoTask,
    Done,
}

/// All possible statuses for each transaction. Each status contains the latest incarnation number.
///
/// 'ReadyToExecute' means that the corresponding incarnation should be executed and the scheduler
/// must eventually create a corresponding execution task. The scheduler ensures that exactly one
/// execution task gets created, changing the status to 'Executing' in the process. If a dependency
/// condition variable is set, then an execution of a prior incarnation is waiting on it with
/// a read dependency resolved (when dependency was encountered, the status changed to Suspended,
/// and suspended changed to ReadyToExecute when the dependency finished its execution). In this case
/// the caller need not create a new execution task, but just nofity the suspended execution.
///
/// 'Executing' status of an incarnation turns into 'Executed' if the execution task finishes, or
/// if a dependency is encountered, it becomes 'ReadyToExecute(incarnation + 1)' once the
/// dependency is resolved. An 'Executed' status allows creation of validation tasks for the
/// corresponding incarnation, and a validation failure leads to an abort. The scheduler ensures
/// that there is exactly one abort, changing the status to 'Aborting' in the process. Once the
/// thread that successfully aborted performs everything that's required, it sets the status
/// to 'ReadyToExecute(incarnation + 1)', allowing the scheduler to create an execution
/// task for the next incarnation of the transaction.
///
/// Status transition diagram:
/// Ready(i)
///    |  try_incarnate (incarnate successfully)
///    |
///    ↓         suspend (waiting on dependency)                resume
/// Executing(i) -----------------------------> Suspended(i) ------------> Ready(i)
///    |
///    |  finish_execution
///    ↓
/// Executed(i) (pending for (re)validations)
///    |
///    |  try_abort (abort successfully)
///    ↓                finish_abort
/// Aborting(i) ---------------------------------------------------------> Ready(i+1)
///
#[derive(Debug)]
enum TransactionStatus {
    ReadyToExecute(Incarnation, Option<DependencyCondvar>),
    Executing(Incarnation),
    Suspended(Incarnation, DependencyCondvar),
    Executed(Incarnation),
    Aborting(Incarnation),
}

impl PartialEq for TransactionStatus {
    fn eq(&self, other: &Self) -> bool {
        use TransactionStatus::*;
        match (self, other) {
            (&ReadyToExecute(ref a, _), &ReadyToExecute(ref b, _))
            | (&Executing(ref a), &Executing(ref b))
            | (&Suspended(ref a, _), &Suspended(ref b, _))
            | (&Executed(ref a), &Executed(ref b))
            | (&Aborting(ref a), &Aborting(ref b)) => a == b,
            _ => false,
        }
    }
}

pub struct Scheduler {
    /// Number of txns to execute, immutable.
    num_txns: usize,

    /// A shared index that tracks the minimum of all transaction indices that require execution.
    /// The threads increment the index and attempt to create an execution task for the corresponding
    /// transaction, if the status of the txn is 'ReadyToExecute'. This implements a counting-based
    /// concurrent ordered set. It is reduced as necessary when transactions become ready to be
    /// executed, in particular, when execution finishes and dependencies are resolved.
    execution_idx: AtomicUsize,
    /// A shared index that tracks the minimum of all transaction indices that require validation.
    /// The threads increment the index and attempt to create a validation task for the corresponding
    /// transaction, if the status of the txn is 'Executed'. This implements a counting-based
    /// concurrent ordered set. It is reduced as necessary when transactions require validation,
    /// in particular, after aborts and executions that write outside of the write set of the
    /// same transaction's previous incarnation.
    validation_idx: AtomicUsize,
    /// The number of times execution_idx and validation_idx are decreased.
    decrease_cnt: AtomicUsize,

    /// Number of tasks used to track when transactions can be committed, incremented / decremented
    /// as new validation or execution tasks are created and completed.
    num_active_tasks: AtomicUsize,
    /// Shared marker that is set when a thread detects that all txns can be committed.
    done_marker: AtomicBool,

    /// An index i maps to indices of other transactions that depend on transaction i, i.e. they
    /// should be re-executed once transaction i's next incarnation finishes.
    txn_dependency: Vec<CachePadded<Mutex<Vec<TxnIndex>>>>,
    /// An index i maps to the most up-to-date status of transaction i.
    txn_status: Vec<CachePadded<Mutex<TransactionStatus>>>,
}

/// Public Interfaces for the Scheduler
impl Scheduler {
    pub fn new(num_txns: usize) -> Self {
        Self {
            num_txns,
            execution_idx: AtomicUsize::new(0),
            validation_idx: AtomicUsize::new(0),
            decrease_cnt: AtomicUsize::new(0),
            num_active_tasks: AtomicUsize::new(0),
            done_marker: AtomicBool::new(false),
            txn_dependency: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(Vec::new())))
                .collect(),
            txn_status: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(TransactionStatus::ReadyToExecute(0, None))))
                .collect(),
        }
    }

    /// Return the number of transactions to be executed from the block.
    pub fn num_txn_to_execute(&self) -> usize {
        self.num_txns
    }

    /// Try to abort version = (txn_idx, incarnation), called upon validation failure.
    /// When the invocation manages to update the status of the transaction, it changes
    /// Executed(incarnation) => Aborting(incarnation), it returns true. Otherwise,
    /// returns false. Since incarnation numbers never decrease, this also ensures
    /// that the same version may not successfully abort more than once.
    pub fn try_abort(&self, txn_idx: TxnIndex, incarnation: Incarnation) -> bool {
        // lock the status.
        let mut status = self.txn_status[txn_idx].lock();

        if *status == TransactionStatus::Executed(incarnation) {
            *status = TransactionStatus::Aborting(incarnation);
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

            let idx_to_validate = self.validation_idx.load(Ordering::SeqCst);
            let idx_to_execute = self.execution_idx.load(Ordering::SeqCst);

            if idx_to_validate < idx_to_execute {
                if let Some((version_to_validate, guard)) = self.try_validate_next_version() {
                    return SchedulerTask::ValidationTask(version_to_validate, guard);
                }
            } else if let Some((version_to_execute, maybe_condvar, guard)) =
                self.try_execute_next_version()
            {
                return SchedulerTask::ExecutionTask(version_to_execute, maybe_condvar, guard);
            }
        }
    }

    /// When a txn depends on another txn, adds it to the dependency list of the other txn.
    /// Returns true if successful, or false, if the dependency got resolved in the meantime.
    /// If true is returned, Scheduler guarantees that later (dep_txn_idx will finish execution)
    /// transaction txn_idx will be resumed, and corresponding execution task created.
    /// If false is returned, it is caller's responsibility to repeat the read that caused the
    /// dependency and continue the ongoing execution of txn_idx.
    pub fn wait_for_dependency(
        &self,
        txn_idx: TxnIndex,
        dep_txn_idx: TxnIndex,
    ) -> Option<DependencyCondvar> {
        // Note: Could pre-check that txn dep_txn_idx isn't in an executed state, but the caller
        // usually has just observed the read dependency.

        // Create a condition variable associated with the dependency.
        let dep_condvar = Arc::new((Mutex::new(false), Condvar::new()));

        let mut stored_deps = self.txn_dependency[dep_txn_idx].lock();

        {
            if self.is_executed(dep_txn_idx).is_some() {
                // Current status of dep_txn_idx is 'executed', so the dependency got resolved.
                // To avoid zombie dependency (and losing liveness), must return here and
                // not add a (stale) dependency.

                // Note: acquires (a different, status) mutex, while holding (dependency) mutex.
                // Only place in scheduler where a thread may hold >1 mutexes, hence, such
                // acquisitions always happens in the same order (this function), may not deadlock.

                return None;
            }

            self.suspend(txn_idx, dep_condvar.clone());

            // Safe to add dependency here (still holding the lock) - finish_execution of txn
            // dep_txn_idx is guaranteed to acquire the same lock later and clear the dependency.
            stored_deps.push(txn_idx);
        }

        Some(dep_condvar)
    }

    /// After txn is executed, schedule its dependencies for re-execution.
    /// If revalidate_suffix is true, decrease validation_idx to schedule all higher transactions
    /// for (re-)validation. Otherwise, in some cases (if validation_idx not already lower),
    /// return a validation task of the transaction to the caller (otherwise NoTask).
    pub fn finish_execution<'a>(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        revalidate_suffix: bool,
        guard: TaskGuard<'a>,
    ) -> SchedulerTask<'a> {
        self.set_executed_status(txn_idx, incarnation);

        let txn_deps: Vec<TxnIndex> = {
            let mut stored_deps = self.txn_dependency[txn_idx].lock();
            // Holding the lock, take dependency vector.
            std::mem::take(&mut stored_deps)
        };

        // Mark dependencies as resolved and find the minimum index among them.
        let min_dep = txn_deps
            .into_iter()
            .map(|dep| {
                // Mark the status of dependencies as 'ReadyToExecute' since dependency on
                // transaction txn_idx is now resolved.
                self.resume(dep);

                dep
            })
            .min();
        if let Some(execution_target_idx) = min_dep {
            // Decrease the execution index as necessary to ensure resolved dependencies
            // get a chance to be re-executed.
            self.decrease_execution_idx(execution_target_idx);
        }

        // If validation_idx is already lower than txn_idx, all required transactions will be
        // considered for validation, and there is nothing to do.
        if self.validation_idx.load(Ordering::SeqCst) > txn_idx {
            if revalidate_suffix {
                // The transaction execution required revalidating all higher txns (not
                // only itself), currently happens when incarnation writes to a new path
                // (w.r.t. the write-set of its previous completed incarnation).
                self.decrease_validation_idx(txn_idx);
            } else {
                // Only transaction txn_idx requires validation. Return validation task
                // back to the caller. No need to change active tasks (-1 +1= 0)
                return SchedulerTask::ValidationTask((txn_idx, incarnation), guard);
            }
        }

        SchedulerTask::NoTask
    }

    /// Finalize a validation task of version (txn_idx, incarnation). In some cases,
    /// may return a re-execution task back to the caller (otherwise, NoTask).
    pub fn finish_abort<'a>(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        guard: TaskGuard<'a>,
    ) -> SchedulerTask<'a> {
        self.set_aborted_status(txn_idx, incarnation);

        // Schedule strictly higher txns for validation
        // (txn_idx needs to be re-executed first).
        self.decrease_validation_idx(txn_idx + 1);

        // txn_idx must be re-executed, and if execution_idx is lower, it will be.
        if self.execution_idx.load(Ordering::SeqCst) > txn_idx {
            // Optimization: execution_idx is higher than txn_idx, but decreasing it may
            // lead to wasted work for all indices between txn_idx and execution_idx.
            // Instead, attempt to create a new incarnation and return the corresponding
            // re-execution task back to the caller. If incarnation fails, there is
            // nothing to do, as another thread must have succeeded to incarnate and
            // obtain the task for re-execution.
            if let Some((new_incarnation, maybe_condvar)) = self.try_incarnate(txn_idx) {
                return SchedulerTask::ExecutionTask(
                    (txn_idx, new_incarnation),
                    maybe_condvar,
                    guard,
                );
            }
        }

        SchedulerTask::NoTask
    }
}

/// Public functions of the Scheduler
impl Scheduler {
    /// Decreases the validation index, increases the decrease counter if it actually decreased.
    fn decrease_validation_idx(&self, target_idx: TxnIndex) {
        if self.validation_idx.fetch_min(target_idx, Ordering::SeqCst) > target_idx {
            self.decrease_cnt.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Decreases the execution index, increases the decrease counter if it actually decreased.
    fn decrease_execution_idx(&self, target_idx: TxnIndex) {
        if self.execution_idx.fetch_min(target_idx, Ordering::SeqCst) > target_idx {
            self.decrease_cnt.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Try and incarnate a transaction. Only possible when the status is
    /// ReadyToExecute(incarnation), in which case Some(incarnation) is returned and the
    /// status is (atomically, due to the mutex) updated to Executing(incarnation).
    /// An unsuccessful incarnation returns None. Since incarnation numbers never decrease
    /// for each transaction, incarnate function may not succeed more than once per version.
    fn try_incarnate(&self, txn_idx: TxnIndex) -> Option<(Incarnation, Option<DependencyCondvar>)> {
        if txn_idx >= self.txn_status.len() {
            return None;
        }

        let mut status = self.txn_status[txn_idx].lock();
        if let TransactionStatus::ReadyToExecute(incarnation, maybe_condvar) = &*status {
            let ret = (*incarnation, maybe_condvar.clone());
            *status = TransactionStatus::Executing(*incarnation);
            Some(ret)
        } else {
            None
        }
    }

    /// If the status of transaction is Executed(incarnation), returns Some(incarnation),
    /// otherwise returns None. Useful to determine when a transaction can be validated,
    /// and to avoid a race in dependency resolution.
    fn is_executed(&self, txn_idx: TxnIndex) -> Option<Incarnation> {
        if txn_idx >= self.txn_status.len() {
            return None;
        }

        let status = self.txn_status[txn_idx].lock();
        if let TransactionStatus::Executed(incarnation) = *status {
            Some(incarnation)
        } else {
            None
        }
    }

    /// Grab an index to try and validate next (by fetch-and-incrementing validation_idx).
    /// - If the index is out of bounds, return None (and invoke a check of whethre
    /// all txns can be committed).
    /// - If the transaction is ready for validation (EXECUTED state), return the version
    /// to the caller together with a guard to be used for the corresponding ValidationTask.
    /// - Otherwise, return None.
    fn try_validate_next_version(&self) -> Option<(Version, TaskGuard)> {
        let idx_to_validate = self.validation_idx.load(Ordering::SeqCst);

        if idx_to_validate >= self.num_txns {
            if !self.check_done() {
                // Avoid pointlessly spinning, and give priority to other threads that may
                // be working to finish the remaining tasks.
                hint::spin_loop();
            }
            return None;
        }

        // Must create guard before incremeting validation_idx.
        let guard = TaskGuard::new(&self.num_active_tasks);
        let idx_to_validate = self.validation_idx.fetch_add(1, Ordering::SeqCst);

        // If incarnation was last executed, and thus ready for validation,
        // return version and guard for validation task, otherwise None.
        self.is_executed(idx_to_validate)
            .map(|incarnation| ((idx_to_validate, incarnation), guard))
    }

    /// Grab an index to try and execute next (by fetch-and-incrementing execution_idx).
    /// - If the index is out of bounds, return None (and invoke a check of whethre
    /// all txns can be committed).
    /// - If the transaction is ready for execution (ReadyToExecute state), attempt
    /// to create the next incarnation (should happen exactly once), and if successful,
    /// return the version to the caller together with a guard to be used for the
    /// corresponding ExecutionTask.
    /// - Otherwise, return None.
    fn try_execute_next_version(&self) -> Option<(Version, Option<DependencyCondvar>, TaskGuard)> {
        let idx_to_execute = self.execution_idx.load(Ordering::SeqCst);

        if idx_to_execute >= self.num_txns {
            if !self.check_done() {
                // Avoid pointlessly spinning, and give priority to other threads that may
                // be working to finish the remaining tasks.
                hint::spin_loop();
            }
            return None;
        }

        // Must create a guard before incrementing execution_idx.
        let guard = TaskGuard::new(&self.num_active_tasks);

        let idx_to_execute = self.execution_idx.fetch_add(1, Ordering::SeqCst);

        // If successfully incarnated (changed status from ready to executing),
        // return version and guard for execution task, otherwise None.
        self.try_incarnate(idx_to_execute)
            .map(|(incarnation, maybe_condvar)| {
                ((idx_to_execute, incarnation), maybe_condvar, guard)
            })
    }

    /// Put a transaction in a suspended state, with a condition variable that can be
    /// used to wake it up after the dependency is resolved.
    fn suspend(&self, txn_idx: TxnIndex, dep_condvar: DependencyCondvar) {
        let mut status = self.txn_status[txn_idx].lock();

        if let TransactionStatus::Executing(incarnation) = *status {
            *status = TransactionStatus::Suspended(incarnation, dep_condvar);
        } else {
            unreachable!();
        }
    }

    /// When a dependency is resolved, mark the transaction as ReadyToExecute with an
    /// incremented incarnation number.
    /// The caller must ensure that the transaction is in the Suspended state.
    fn resume(&self, txn_idx: TxnIndex) {
        let mut status = self.txn_status[txn_idx].lock();
        if let TransactionStatus::Suspended(incarnation, dep_condvar) = &*status {
            *status = TransactionStatus::ReadyToExecute(*incarnation, Some(dep_condvar.clone()));
        } else {
            unreachable!();
        }
    }

    /// Set status of the transaction to Executed(incarnation).
    fn set_executed_status(&self, txn_idx: TxnIndex, incarnation: Incarnation) {
        let mut status = self.txn_status[txn_idx].lock();

        // Only makes sense when the current status is 'Executing'.
        debug_assert!(*status == TransactionStatus::Executing(incarnation));

        *status = TransactionStatus::Executed(incarnation);
    }

    /// After a successful abort, mark the transaction as ready for re-execution with
    /// an incremented incarnation number.
    fn set_aborted_status(&self, txn_idx: TxnIndex, incarnation: Incarnation) {
        let mut status = self.txn_status[txn_idx].lock();

        // Only makes sense when the current status is 'Aborting'.
        debug_assert!(*status == TransactionStatus::Aborting(incarnation));

        *status = TransactionStatus::ReadyToExecute(incarnation + 1, None);
    }

    /// A lazy, check of whether the scheduler execution is completed.
    /// Updates the 'done_marker' so other threads can know by calling done() function below.
    ///
    /// 1. After the STM execution has completed:
    /// validation_idx >= num_txn, execution_idx >= num_txn, num_active_tasks == 0,
    /// and decrease_cnt does not change - so it will be successfully detected.
    /// 2. If done_marker is set, all of these must hold at the same time, implying completion.
    /// Proof: O.w. one of the indices must decrease from when it is read to be >= num_txns
    /// to when num_active_tasks is read to be 0, but decreasing thread is performing an active task,
    /// so it must first perform the next instruction in 'decrease_validation_idx' or
    /// 'decrease_execution_idx' functions, which is to increment the decrease_cnt++.
    /// Final check will then detect a change in decrease_cnt and not allow a false positive.
    fn check_done(&self) -> bool {
        let observed_cnt = self.decrease_cnt.load(Ordering::SeqCst);

        let val_idx = self.validation_idx.load(Ordering::SeqCst);
        let exec_idx = self.execution_idx.load(Ordering::SeqCst);
        let num_tasks = self.num_active_tasks.load(Ordering::SeqCst);
        if min(exec_idx, val_idx) < self.num_txns || num_tasks > 0 {
            // There is work remaining.
            return false;
        }

        // Re-read and make sure decrease_cnt hasn't changed.
        if observed_cnt == self.decrease_cnt.load(Ordering::SeqCst) {
            self.done_marker.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Checks whether the done marker is set. The marker can only be set by 'check_done'.
    fn done(&self) -> bool {
        self.done_marker.load(Ordering::Acquire)
    }
}
