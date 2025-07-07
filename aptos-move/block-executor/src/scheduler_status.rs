// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler_v2::ExecutionQueueManager;
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use crossbeam::utils::CachePadded;
use std::{
    cmp,
    sync::atomic::{AtomicU32, AtomicU8, Ordering},
};

/**
============================== Transaction Status Lifecycle ==============================

Each transaction status contains an incarnation number (starting with 0) and progresses
through a well-defined lifecycle:

1. Initial State:
   - A transaction begins in the `PendingScheduling` status, meaning it's ready to be picked up
     by the BlockSTMv2 scheduler.
   - When the scheduler selects a transaction, it transitions the status to `Executing` via
     the [ExecutionStatuses::start_executing] method.

2. Abort Process:
   - A transaction incarnation may be aborted if it reads data that is later modified in a way
     that would cause the transaction to read different values if it executed again. This
     signals the need for re-execution with an incremented incarnation number.
   - In BlockSTMv2, a transaction can be aborted while executing or after execution finishes.
   - Abort happens in two distinct phases:

   a) Start Abort Phase:
      - [ExecutionStatuses::start_abort] is called with an incarnation number and succeeds if
        the incarnation has started executing and has not already been aborted.
      - This serves as an efficient test-and-set filter for multiple abort attempts (which
        can occur when a transaction makes multiple reads that may each be invalidated by
        different transactions).
      - Early detection allows the ongoing execution to stop immediately rather than continue
        work that will ultimately be discarded.

   b) Finish Abort Phase:
      - A successful [ExecutionStatuses::start_abort] must be followed by a
        [ExecutionStatuses::finish_abort] call on the status.
        • If the status was 'Executed', it transitions to 'PendingScheduling' for the
          next incarnation.
        • If the status was 'Executing', it transitions to 'Aborted'.
      - When transaction T1 successfully aborts transaction T2 (where T2 > T1):
        • T2 stops executing as soon as possible,
        • Subsequent scheduling of T2 may wait until T1 finishes, since T1 has higher
          priority (lower index),
        • After T1 completes, the worker can process all related aborts in batch. e.g. calling
          [ExecutionStatuses::finish_abort], tracking dependencies, and propagating stalls.

3. Execution Completion:
   - When execution finishes, [ExecutionStatuses::finish_execution] is called on the status.
   - If the status was `Aborted`, it transitions to `PendingScheduling` for the next incarnation.
   - If the status was `Executing`, it transitions to `Executed`.

Status Transition Diagram:

PendingScheduling(i)
    |
    | start_executing
    |
    ↓                       finish_execution
Executing(i) ------------------------------> Executed(i)
    |                                           |
    | start_abort(i) + finish_abort(i)            | start_abort(i) + finish_abort(i)
    |                                           |
    ↓                    finish_execution       ↓
Aborted(i) ------------------------------> PendingScheduling(i+1)

Note: [ExecutionStatuses::start_abort] doesn't change the status directly but marks the
transaction for abort. The actual status change occurs during
[ExecutionStatuses::finish_abort]. Both steps are required to complete the abort process.

============================== Transaction Stall Mechanism ==============================

In the BlockSTMv2 scheduler, a transaction status can be "stalled," meaning there have been
more [ExecutionStatuses::add_stall] than [ExecutionStatuses::remove_stall] calls on its status.
Each successful [ExecutionStatuses::add_stall] call requires a guarantee that the
corresponding[ExecutionStatuses::remove_stall] will eventually be performed.

The stall mechanism can be conceptualized as balanced parentheses - `add_stall` represents
an opening bracket '(' and `remove_stall` represents a closing bracket ')'. A status becomes
"unstalled" when the brackets are balanced (equal number of calls).

Key aspects of the stall mechanism:

1. Purpose:
   - Records that a transaction has dependencies that are more likely to cause re-execution
   - Can be used to:
     a) Avoid scheduling transactions for re-execution until stalls are removed
     b) Guide handling when another transaction observes a dependency during execution
   - Helps constrain optimistic concurrency by limiting cascading aborts

2. Behavior:
   - Best-effort approach that allows flexibility in concurrency scenarios, but such that
     high-priority transactions may still be re-executed even in stalled state

============================== Notes on Method Call Concurrency ==============================

In general, most methods in this module can be called concurrently with the following exceptions:

1. Each successful [ExecutionStatuses::add_stall] call must be balanced by a
   corresponding [ExecutionStatuses::remove_stall] call that starts after the add_stall
   call completes. Multiple concurrent add_stall and remove_stall calls on the same
   transaction status are supported as long as this balancing property is maintained.

2. While multiple [ExecutionStatuses::start_executing] calls may be attempted
   concurrently, at most one can succeed for a given incarnation. A successful call
   must be followed by exactly one corresponding [ExecutionStatuses::finish_execution]
   call, which can execute concurrently with [ExecutionStatuses::start_abort] calls.
   Only one of these calls can succeed, leading to a single [ExecutionStatuses::finish_abort]
   call being performed for a given incarnation. There may be multiple concurrent
   calls for outdated incarnations
**/

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchedulingStatus {
    PendingScheduling,
    Executing,
    Aborted,
    Executed,
}

/// Represents the internal execution status of a transaction at a specific incarnation.
/// Tracks both the current state (via StatusEnum) and the incarnation number.
/// Incarnation number, starting at 0 and incremented after each abort, represents a
/// distinct execution attempt of the transaction.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StatusWithIncarnation {
    status: SchedulingStatus,
    incarnation: Incarnation,
}

impl StatusWithIncarnation {
    fn new() -> Self {
        Self {
            status: SchedulingStatus::PendingScheduling,
            incarnation: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(status: SchedulingStatus, incarnation: Incarnation) -> Self {
        Self {
            status,
            incarnation,
        }
    }

    fn start_executing(&mut self) -> Option<Incarnation> {
        if self.status == SchedulingStatus::PendingScheduling {
            self.status = SchedulingStatus::Executing;
            return Some(self.incarnation);
        }
        None
    }

    fn incarnation(&self) -> Incarnation {
        self.incarnation
    }

    fn never_started_execution(&self, incarnation: Incarnation) -> bool {
        self.incarnation < incarnation
            || (self.incarnation == incarnation
                && self.status == SchedulingStatus::PendingScheduling)
    }

    fn already_aborted(&self, incarnation: Incarnation) -> bool {
        self.incarnation > incarnation
            || (self.incarnation == incarnation && self.status == SchedulingStatus::Aborted)
    }

    fn pending_scheduling(&self) -> Option<Incarnation> {
        (self.status == SchedulingStatus::PendingScheduling).then_some(self.incarnation)
    }

    fn is_executed(&self) -> bool {
        self.status == SchedulingStatus::Executed
    }

    fn ever_executed(&self) -> bool {
        // Aborted w. incarnation 0 is not considered as ever executed, because aborted
        // is set on start_abort, and incarnation 0 is prioritized in the scheduler to
        // actually finish execution / not early abort (to produce a speculative write-set).
        self.incarnation > 0 || self.status == SchedulingStatus::Executed
    }
}

/// Flag values for dependency resolution stored in an `AtomicU8` to allow lock-free reads.
/// These values represent the state of a transaction that other transactions depend on.
/// The status flags are updated while holding the status lock but provide a fast way to
/// evaluate a predicate associated with the status that enables the scheduler to make
/// decisions about stall propagation, transaction scheduling, and dependency resolution.
///
/// DependencyStatus Transition Diagram:
///   +----------------+   ------------------>   +-------------------+
///   |                |     start_executing     |                   |
///   |  ShouldDefer   |                         |  WaitForExecution |
///   |                |   <------------------   |                   |
///   +----------------+      finish_abort       +-------------------+
///      ^        ^               OR                     |
///      |        |         finish_execution             |
///      |        |             (stalled)                |
///      |        |                                 finish_execution
///      |        |                                  (not stalled)
///      |        |                                      |
///      |        |                                      v
///      |        |                              +----------------+
///      |        |                              |                |
///      |        |                              |     IsSafe     |
///      |        |                              |                |
///      |        |                              +----------------+
///      |        |                                     |   ^
///      |        |                                     |   |
///      |        +-------------------------------------+   |
///      |          add_stall (0→1) OR finish_abort         |
///      |                                                  |
///      +--------------------------------------------------+
///                         remove_stall
///                  (1→0 stalls on Executed status)
///
/// Self-transitions (not shown in diagram): ShouldDefer → ShouldDefer:
/// - add_stall when already stalled (stall count > 0)
/// - remove_stall when stalls remain (stall count > 1)
///
/// Key transitions:
/// 1. When a transaction starts executing, its flag changes from ShouldDefer to WaitForExecution
/// 2. When execution finishes successfully and the transaction is not stalled, the flag changes
///    to IsSafe
/// 3. When execution finishes but the transaction is stalled, or when finish_abort is called,
///    the flag changes to ShouldDefer
/// 4. Adding a stall to an executed transaction with flag IsSafe changes it to ShouldDefer
/// 5. Removing the last stall from an executed transaction changes its flag from ShouldDefer to IsSafe
#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum DependencyStatus {
    /// The transaction has successfully executed and is not stalled.
    /// Reading values written by this dependency is safe.
    IsSafe = 0,
    /// The transaction is currently executing.
    ///
    /// In this case, it may be beneficial to wait for the execution to finish
    /// to obtain up-to-date values rather than proceeding with potentially
    /// stale data. This is especially relevant for pipelining high-priority
    /// transaction execution to avoid aborts from reading outdated values.
    WaitForExecution = 1,
    /// This occurs when:
    /// 1. The transaction is in Aborted or PendingScheduling state (not yet re-scheduled)
    /// 2. The transaction is Executed but stalled (has an active dependency chain
    ///    that previously triggered an abort and may do so again)
    ShouldDefer = 2,
}

impl DependencyStatus {
    fn from_u8(flag: u8) -> Result<Self, PanicError> {
        match flag {
            0 => Ok(Self::IsSafe),
            1 => Ok(Self::WaitForExecution),
            2 => Ok(Self::ShouldDefer),
            _ => Err(code_invariant_error(format!(
                "Invalid dependency status flag: {}",
                flag
            ))),
        }
    }
}

/// The primary structure for tracking and managing transaction execution status.
///
/// ExecutionStatus coordinates the lifecycle of transaction execution, handles
/// aborts and re-executions, manages dependencies between transactions, and
/// implements the stall mechanism to reduce cascading aborts.
///
/// Each transaction in the system has its own ExecutionStatus instance, which
/// persists across multiple execution attempts (incarnations) of that transaction.
pub(crate) struct ExecutionStatus {
    /// Protects access to the incarnation and inner status.
    ///
    /// This mutex synchronizes writes to incarnation and status changes, as well
    /// as modifications that affect the dependency shortcut (e.g., when stall count
    /// changes between 0 and non-zero).
    status_with_incarnation: CachePadded<Mutex<StatusWithIncarnation>>,

    /// Counter to track and filter abort attempts.
    ///
    /// This counter is monotonically increasing and updated in a successful start_abort.
    /// It allows filtering fanned-out abort attempts when multiple workers executing
    /// different transactions invalidate different reads of the same transaction.
    /// Only one of these workers will successfully abort the transaction and perform
    /// the required processing.
    next_incarnation_to_abort: CachePadded<AtomicU32>,

    /// Part of inner status state summarized as a single flag that can be read lock-free.
    /// The allowed values are defined in DependencyStatus shortcut.
    dependency_shortcut: CachePadded<AtomicU8>,

    /// Tracks the number of active stalls on this transaction.
    ///
    /// A transaction is considered "stalled" when this count is greater than 0.
    /// Each add_stall increments this counter, and each remove_stall decrements it.
    /// The status is "unstalled" when the counter returns to 0.
    num_stalls: CachePadded<AtomicU32>,
}

pub(crate) struct ExecutionStatuses {
    statuses: Vec<CachePadded<ExecutionStatus>>,
    /// Interface to manage the transaction execution queue.
    ///
    /// Allows adding or removing transactions from the execution queue based on
    /// their status changes. Used when stalls are added/removed or when
    /// a new incarnation is created.
    execution_queue_manager: CachePadded<ExecutionQueueManager>,
}

impl ExecutionStatuses {
    pub(crate) fn new(num_txns: TxnIndex) -> Self {
        Self {
            statuses: (0..num_txns)
                .map(|_| CachePadded::new(ExecutionStatus::new()))
                .collect(),
            execution_queue_manager: CachePadded::new(ExecutionQueueManager::new(num_txns)),
        }
    }

    pub(crate) fn get_execution_queue_manager(&self) -> &ExecutionQueueManager {
        &self.execution_queue_manager
    }

    /// Adds a stall to the transaction, indicating it has dependencies that might cause re-execution.
    /// This may be called for txn i, that has previously been aborted by txn j, after txn j has been
    /// aborted. The reasoning is that txn j may again abort txn i, so it makes sense to be pessimistic
    /// and hold on re-executing txn i until txn j has finished.
    ///
    /// When a transaction is stalled, it is removed from the execution queue if in PendingScheduling
    /// state, or its dependency shortcut is updated from SAFE to DEFER if in Executed state.
    ///
    /// # Returns
    /// - `Ok(true)` if this call changed the state from unstalled to stalled (num_stalls 0→1)
    /// - `Ok(false)` if the transaction was already stalled or a race condition occurred
    /// - `Err(PanicError)` if there was an invalid or inconsistent state.
    ///
    /// # Note
    /// Each successful add_stall must be balanced by a corresponding remove_stall call that starts
    /// after add_stall finishes.
    pub(crate) fn add_stall(&self, txn_idx: TxnIndex) -> Result<bool, PanicError> {
        let status = &self.statuses[txn_idx as usize];
        if status.num_stalls.fetch_add(1, Ordering::SeqCst) == 0 {
            // Acquire write lock for (non-monitor) shortcut modifications.
            let status_guard = status.status_with_incarnation.lock();

            let dependency_status =
                DependencyStatus::from_u8(status.dependency_shortcut.load(Ordering::Relaxed))?;

            match (status_guard.pending_scheduling(), dependency_status) {
                (Some(0), DependencyStatus::ShouldDefer) => {
                    // Adding a stall requires being recorded in aborted dependencies in scheduler_v2,
                    // which in turn only happens in the scheduler after a successful abort (that must
                    // increment the incarnation of the status).
                    return Err(code_invariant_error("0-th incarnation in add_stall"));
                },
                (Some(_), DependencyStatus::ShouldDefer) => {
                    self.execution_queue_manager.remove_from_schedule(txn_idx);
                    // Shortcut not affected.
                },
                (Some(_), DependencyStatus::IsSafe | DependencyStatus::WaitForExecution) => {
                    return Err(code_invariant_error(
                        "Inconsistent status and dependency shortcut in add_stall",
                    ));
                },
                (None, DependencyStatus::IsSafe) => {
                    // May not update IsSafe dependency status at an incorrect time in the future
                    // (i.e. ABA), as observing num_stalls = 0 under status is required to set
                    // IsSafe status, but impossible until the corresponding remove_stall (that
                    // starts only after add_stall finishes).
                    status
                        .dependency_shortcut
                        .store(DependencyStatus::ShouldDefer as u8, Ordering::Relaxed);
                },
                (None, DependencyStatus::WaitForExecution | DependencyStatus::ShouldDefer) => {
                    // Executing or aborted: shortcut not affected.
                },
            }

            return Ok(true);
        }
        Ok(false)
    }

    /// Removes a stall from the transaction, potentially allowing its re-execution.
    ///
    /// This is the counterpart to add_stall. When the number of stalls drops to zero,
    /// the transaction becomes "unstalled" and may be rescheduled for execution if
    /// it's in the PendingScheduling state.
    ///
    /// # Returns
    /// - `Ok(true)` if this call changed the state from stalled to unstalled (num_stalls 1→0)
    /// - `Ok(false)` if the transaction remains stalled after this call
    /// - `Err` if there was an error removing the stall (e.g., no matching add_stall)
    pub(crate) fn remove_stall(&self, txn_idx: TxnIndex) -> Result<bool, PanicError> {
        let status = &self.statuses[txn_idx as usize];
        let prev_num_stalls = status.num_stalls.fetch_sub(1, Ordering::SeqCst);

        if prev_num_stalls == 0 {
            return Err(code_invariant_error(
                "remove_stall called when num_stalls == 0",
            ));
        }

        if prev_num_stalls == 1 {
            // Acquire write lock for (non-monitor) shortcut modifications.
            let status_guard = status.status_with_incarnation.lock();

            // num_stalls updates are not under the lock, so need to re-check (otherwise
            // a different add_stall might have already incremented the count).
            if status.is_stalled() {
                return Ok(false);
            }

            if let Some(incarnation) = status_guard.pending_scheduling() {
                if incarnation == 0 {
                    // Invariant due to scheduler logic: for a successful remove_stall there
                    // must have been an add_stall for incarnation 0, which is impossible.
                    return Err(code_invariant_error("0-th incarnation in remove_stall"));
                }
                self.execution_queue_manager
                    .add_to_schedule(incarnation == 1, txn_idx);
            } else if status_guard.is_executed() {
                // TODO(BlockSMTv2): Here, when waiting is supported, if inner status is executed,
                // would need to notify waiting workers.

                // Status is Executed so the dependency status may not be WaitForExecution
                // (finish_execution sets ShouldDefer or IsSafe dependency status).
                status.swap_dependency_status_any(
                    &[DependencyStatus::ShouldDefer, DependencyStatus::IsSafe],
                    DependencyStatus::IsSafe,
                    "remove_stall",
                )?;
            }

            return Ok(true);
        }
        Ok(false)
    }

    /// Attempts to transition a transaction from PendingScheduling to Executing state.
    ///
    /// This method is called by the scheduler when it selects a transaction for execution.
    /// It only succeeds if the transaction is currently in PendingScheduling state.
    ///
    /// # Returns
    /// - `Ok(Some(incarnation))` if the transition was successful, returning the incarnation number
    /// - `Ok(None)` if the transaction is not in PendingScheduling state
    /// - `Err(PanicError)` if there was an error during dependency status update
    pub(crate) fn start_executing(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<Option<Incarnation>, PanicError> {
        let status = &self.statuses[txn_idx as usize];

        let status_guard = &mut *status.status_with_incarnation.lock();
        let ret = status_guard.start_executing();

        if ret.is_some() {
            // When status is PendingScheduling the dependency status should be
            // WaitForExecution (default or set by abort under lock).
            status.swap_dependency_status_any(
                &[DependencyStatus::ShouldDefer],
                DependencyStatus::WaitForExecution,
                "start_executing",
            )?;
        }

        Ok(ret)
    }

    /// Attempts to mark a transaction incarnation for abort.
    ///
    /// This is the first step of the two-step abort process. It serves as an efficient
    /// test-and-set filter for abort attempts, ensuring only one worker successfully
    /// initiates the abort of a specific incarnation.
    ///
    /// # Parameters
    /// - `incarnation`: The incarnation number to abort
    ///
    /// # Returns
    /// - `Ok(true)` if this call successfully marked the incarnation for abort
    /// - `Ok(false)` if the incarnation was already marked for abort
    /// - `Err` if the provided incarnation is invalid (greater than the current value)
    pub(crate) fn start_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<bool, PanicError> {
        let prev_value = self.statuses[txn_idx as usize]
            .next_incarnation_to_abort
            .fetch_max(incarnation + 1, Ordering::Relaxed);
        match incarnation.cmp(&prev_value) {
            cmp::Ordering::Less => Ok(false),
            cmp::Ordering::Equal => Ok(true),
            cmp::Ordering::Greater => Err(code_invariant_error(format!(
                "Try abort incarnation {} > self.next_incarnation_to_abort = {}",
                incarnation, prev_value,
            ))),
        }
    }

    /// Marks a transaction's execution as completed.
    ///
    /// Called when a transaction finishes executing, whether successfully or after being aborted.
    /// Updates the transaction's status based on its current state:
    /// - If Executing → Executed (successful execution)
    /// - If Aborted → PendingScheduling with incremented incarnation (ready for re-execution)
    ///
    /// # Parameters
    /// - `finished_incarnation`: The incarnation that has finished execution
    ///
    /// # Returns
    /// - 'Ok(is the execution still valid at the time finish was recorded)', in particular:
    ///     - `Ok(true)` if transitioned from Executing to Executed
    ///     - `Ok(false)` if transitioned from Aborted to PendingScheduling
    /// - `Err` if the current state doesn't allow finishing execution
    pub(crate) fn finish_execution(
        &self,
        txn_idx: TxnIndex,
        finished_incarnation: Incarnation,
    ) -> Result<bool, PanicError> {
        // TODO(BlockSMTv2): Handle waiting workers when supported (defer waking up).

        let status = &self.statuses[txn_idx as usize];
        let status_guard = &mut *status.status_with_incarnation.lock();

        // An incarnation of a transaction can only increase when both finish_execution and
        // start_abort take effect for the prior incarnation. However, finish_execution is
        // invoked once per incarnation, and thus incarnations must always match.
        if status_guard.incarnation() != finished_incarnation {
            return Err(code_invariant_error(format!(
                "Finish execution of incarnation {}, but inner status {:?}",
                finished_incarnation, status_guard,
            )));
        }

        match status_guard.status {
            SchedulingStatus::Executing => {
                status_guard.status = SchedulingStatus::Executed;

                let new_status_flag = if status.is_stalled() {
                    DependencyStatus::ShouldDefer
                } else {
                    DependencyStatus::IsSafe
                };
                status.swap_dependency_status_any(
                    &[DependencyStatus::WaitForExecution],
                    new_status_flag,
                    "finish_execution",
                )?;

                Ok(true)
            },
            SchedulingStatus::Aborted => {
                self.to_pending_scheduling(txn_idx, status_guard, finished_incarnation + 1, true);
                Ok(false)
            },
            SchedulingStatus::PendingScheduling | SchedulingStatus::Executed => {
                Err(code_invariant_error(format!(
                    "Status update to Executed failed, previous inner status {:?}",
                    status_guard
                )))
            },
        }
    }

    /// Completes the abort of a transaction incarnation. It is the second step of
    /// the two-step abort process. It must be called after a successful
    /// [ExecutionStatuses::start_abort] and updates the transaction's status.
    /// - If Executing → Aborted
    /// - If Executed → PendingScheduling with incremented incarnation
    ///
    /// # Parameters
    /// - `aborted_incarnation`: The incarnation being aborted
    /// - `add_to_schedule`: If applicable (i.e. not stalled and requiring re-execution)
    /// whether to add the transaction to the scheduler's execution queue. The parameter
    /// may be false, e.g., if the caller can re-execute the transaction itself.
    ///
    /// # Returns
    /// - `Ok(())` if abort was completed successfully
    /// - `Err` if the abort can't be completed (e.g., wrong incarnation or status)
    pub(crate) fn finish_abort(
        &self,
        txn_idx: TxnIndex,
        aborted_incarnation: Incarnation,
        add_to_schedule: bool,
    ) -> Result<(), PanicError> {
        let status = &self.statuses[txn_idx as usize];
        let new_incarnation = aborted_incarnation + 1;
        if status.next_incarnation_to_abort.load(Ordering::Relaxed) != new_incarnation {
            // The caller must have already successfully performed a start_abort, while
            // higher incarnation may not have started until the abort finished (here).
            return Err(code_invariant_error(format!(
                "Finish abort of incarnation {}, self.next_incarnation_to_abort = {}",
                aborted_incarnation,
                status.next_incarnation_to_abort.load(Ordering::Relaxed),
            )));
        }

        {
            let status_guard = &mut *status.status_with_incarnation.lock();
            if status_guard.already_aborted(aborted_incarnation)
                || status_guard.never_started_execution(aborted_incarnation)
            {
                return Err(code_invariant_error(format!(
                    "Finish abort of incarnation {}, but inner status {:?}",
                    aborted_incarnation, status_guard
                )));
            }

            match status_guard.status {
                SchedulingStatus::Executing => {
                    status_guard.status = SchedulingStatus::Aborted;
                    status.swap_dependency_status_any(
                        &[DependencyStatus::WaitForExecution],
                        DependencyStatus::ShouldDefer,
                        "finish_abort",
                    )?;
                },
                SchedulingStatus::Executed => {
                    self.to_pending_scheduling(
                        txn_idx,
                        status_guard,
                        new_incarnation,
                        add_to_schedule,
                    );
                },
                SchedulingStatus::PendingScheduling | SchedulingStatus::Aborted => {
                    return Err(code_invariant_error(format!(
                        "Status update to Aborted failed, previous inner status {:?}",
                        status_guard
                    )));
                },
            }
        }

        Ok(())
    }

    /// Checks if an incarnation has already been marked for abort.
    ///
    /// This can be called during an ongoing execution to determine if the
    /// execution has been concurrently aborted. This allows the executor
    /// to return early and to discard the results.
    #[inline]
    pub(crate) fn already_started_abort(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> bool {
        self.statuses[txn_idx as usize]
            .next_incarnation_to_abort
            .load(Ordering::Relaxed)
            > incarnation
    }

    /// Checks if the transaction is ready for scheduling and not stalled.
    /// This is used to determine if a transaction should be considered
    /// for execution by the scheduler.
    pub(crate) fn pending_scheduling_and_not_stalled(&self, txn_idx: TxnIndex) -> bool {
        let status = &self.statuses[txn_idx as usize];
        let guard = status.status_with_incarnation.lock();
        guard.pending_scheduling().is_some() && !status.is_stalled()
    }

    /// Checks that the dependency status is IsSafe. This is used by the scheduler
    /// when removing a previously propagated stall signal, and it is safe to
    /// use the shortcutbecause of the best-effort nature of the stall mechanism.
    pub(crate) fn shortcut_executed_and_not_stalled(&self, txn_idx: usize) -> bool {
        let status = &self.statuses[txn_idx];
        status.dependency_shortcut.load(Ordering::Relaxed) == DependencyStatus::IsSafe as u8
    }

    pub(crate) fn ever_executed(&self, txn_idx: TxnIndex) -> bool {
        self.statuses[txn_idx as usize]
            .status_with_incarnation
            .lock()
            .ever_executed()
    }

    pub(crate) fn is_executed(&self, txn_idx: TxnIndex) -> bool {
        self.statuses[txn_idx as usize]
            .status_with_incarnation
            .lock()
            .is_executed()
    }

    pub(crate) fn incarnation(&self, txn_idx: TxnIndex) -> Incarnation {
        self.statuses[txn_idx as usize]
            .status_with_incarnation
            .lock()
            .incarnation()
    }

    // If the txn is executing or executed, it might require module validation when a
    // lower txn that published a module is committed. The validation requirement applies
    // to the specific incarnation that is returned (i.e. if the incarnation gets aborted
    // before the validation is performed, then validation can be safely skipped). While
    // executing, a txn's read-set is only stored locally and can't be validated by other
    // workers. In this case, the boolean is set to true indicating that the caller may
    // want to call [ExecutionStatuses::add_module_validation_requirement] (which will
    // records the requirement to be performed after the execution finishes, but such
    // logic can also be implemented by the caller). If the incarnation is already
    // executed, then the boolean is set to false.
    //
    // # Returns
    // - `Some((incarnation, is_executing))` if the txn requires module validation
    // - `None` if the txn is not executing / executed.
    pub(crate) fn requires_module_validation(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<(Incarnation, bool)> {
        let status = &self.statuses[txn_idx as usize];
        let status_guard = status.status_with_incarnation.lock();

        match status_guard.status {
            SchedulingStatus::Executing => Some((status_guard.incarnation(), true)),
            SchedulingStatus::Executed => Some((status_guard.incarnation(), false)),
            SchedulingStatus::PendingScheduling | SchedulingStatus::Aborted => None,
        }
    }
}

// Private interfaces.
impl ExecutionStatuses {
    // Updates inner status to PendingScheduling with the new incarnation.
    // Also updates the dependency status shortcut to ShouldDefer.
    // The caller must hold the lock on InnerStatus (enforced by the &mut parameter).
    fn to_pending_scheduling(
        &self,
        txn_idx: TxnIndex,
        status_guard: &mut StatusWithIncarnation,
        new_incarnation: Incarnation,
        add_to_schedule: bool,
    ) {
        let status = &self.statuses[txn_idx as usize];
        // Update inner status.
        status_guard.status = SchedulingStatus::PendingScheduling;
        status_guard.incarnation = new_incarnation;

        // Under the lock, update the shortcuts.
        status
            .dependency_shortcut
            .store(DependencyStatus::ShouldDefer as u8, Ordering::Relaxed);

        if add_to_schedule && !status.is_stalled() {
            // Need to schedule the transaction for re-execution. If stalled, then
            // scheduling is deferred to the remove_stall.
            self.execution_queue_manager
                .add_to_schedule(new_incarnation == 1, txn_idx);
        }
    }
}

impl ExecutionStatus {
    pub(crate) fn new() -> Self {
        Self {
            status_with_incarnation: CachePadded::new(Mutex::new(StatusWithIncarnation::new())),
            next_incarnation_to_abort: CachePadded::new(AtomicU32::new(0)),
            dependency_shortcut: CachePadded::new(AtomicU8::new(
                DependencyStatus::ShouldDefer as u8,
            )),
            num_stalls: CachePadded::new(AtomicU32::new(0)),
        }
    }

    /// Performs an atomic swap operation on the dependency status and checks
    /// that the previous value matches one of the expected values.
    /// Note that in our implementation, all updates to the status are performed
    /// while holding the lock on InnerStatus, which is the responsibility
    /// of the caller.
    ///
    /// # Parameters
    /// - `expected_values`: Array of possible expected current status flags
    /// - `new_value`: The new status flag to set
    /// - `context`: A string describing the context for error messages
    ///
    /// # Returns
    /// - `Ok(prev)` if the swap was successful, returning the previous value
    /// - `Err(PanicError)` if the previous value didn't match any expected value
    fn swap_dependency_status_any(
        &self,
        expected_values: &[DependencyStatus],
        new_value: DependencyStatus,
        context: &'static str,
    ) -> Result<DependencyStatus, PanicError> {
        let prev = DependencyStatus::from_u8(
            self.dependency_shortcut
                .swap(new_value as u8, Ordering::Relaxed),
        )?;
        // Note: can avoid a lookup by optimizing expected values representation.
        if !expected_values.contains(&prev) {
            return Err(code_invariant_error(format!(
                "Incorrect dependency status in {}: expected one of {:?}, found {:?}",
                context, expected_values, prev,
            )));
        }
        Ok(prev)
    }

    pub(crate) fn is_stalled(&self) -> bool {
        self.num_stalls.load(Ordering::Relaxed) > 0
    }
}

// Testing interfaces.
#[cfg(test)]
impl ExecutionStatuses {
    pub(crate) fn new_for_test(
        execution_queue_manager: ExecutionQueueManager,
        statuses: Vec<ExecutionStatus>,
    ) -> Self {
        Self {
            statuses: statuses.into_iter().map(CachePadded::new).collect(),
            execution_queue_manager: CachePadded::new(execution_queue_manager),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.statuses.len()
    }

    pub(crate) fn get_status_mut(&mut self, txn_idx: TxnIndex) -> &mut ExecutionStatus {
        &mut self.statuses[txn_idx as usize]
    }

    pub(crate) fn get_status(&self, txn_idx: TxnIndex) -> &ExecutionStatus {
        &self.statuses[txn_idx as usize]
    }
}

#[cfg(test)]
impl ExecutionStatus {
    pub(crate) fn new_for_test(
        status_with_incarnation: StatusWithIncarnation,
        num_stalls: u32,
    ) -> Self {
        let incarnation = status_with_incarnation.incarnation();
        let shortcut = match status_with_incarnation.status {
            SchedulingStatus::PendingScheduling | SchedulingStatus::Aborted => {
                DependencyStatus::ShouldDefer
            },
            SchedulingStatus::Executing => DependencyStatus::WaitForExecution,
            SchedulingStatus::Executed => {
                if num_stalls == 0 {
                    DependencyStatus::IsSafe
                } else {
                    DependencyStatus::ShouldDefer
                }
            },
        };
        Self {
            status_with_incarnation: CachePadded::new(Mutex::new(status_with_incarnation)),
            next_incarnation_to_abort: CachePadded::new(AtomicU32::new(incarnation)),
            dependency_shortcut: CachePadded::new(AtomicU8::new(shortcut as u8)),
            num_stalls: CachePadded::new(AtomicU32::new(num_stalls)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some_eq};
    use test_case::test_case;

    fn assert_simple_status_state(
        status: &ExecutionStatus,
        exp_num_stalls: u32,
        exp_incarnation: Incarnation,
        exp_dependency_shortcut: u8,
    ) {
        assert_eq!(
            status.status_with_incarnation.lock().incarnation,
            exp_incarnation
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), exp_num_stalls);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            exp_dependency_shortcut
        );
        assert_eq!(
            status.next_incarnation_to_abort.load(Ordering::Relaxed),
            exp_incarnation
        );
        // TODO(BlockSMTv2): Testing waiting and resolving dependencies here
        // when support is added.
    }

    fn check_after_finish_and_abort(
        statuses: &ExecutionStatuses,
        txn_idx: TxnIndex,
        expected_incarnation: Incarnation,
        stall_before_finish: bool,
    ) {
        let status = &statuses.statuses[txn_idx as usize];
        assert_eq!(
            status.status_with_incarnation.lock().status,
            SchedulingStatus::PendingScheduling
        );
        assert_simple_status_state(
            status,
            if stall_before_finish { 1 } else { 0 },
            expected_incarnation,
            DependencyStatus::ShouldDefer as u8,
        );

        if stall_before_finish {
            assert_ok_eq!(statuses.remove_stall(txn_idx), true);
        }
        statuses
            .execution_queue_manager
            .assert_execution_queue(&vec![txn_idx]);
    }

    #[test_case(false)]
    #[test_case(true)]
    fn status_cycle_with_finish_and_resolve(stall_before_finish: bool) {
        let txn_idx = 0;
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new(),
            ]);
        let status = statuses.get_status(txn_idx);

        assert_eq!(
            status.status_with_incarnation.lock().status,
            SchedulingStatus::PendingScheduling
        );
        assert_simple_status_state(status, 0, 0, DependencyStatus::ShouldDefer as u8);

        // Compatible with start (incompatible with abort and finish).
        for i in [0, 2] {
            assert_err!(statuses.finish_execution(txn_idx, i));
            assert_err!(statuses.finish_abort(txn_idx, i, true));
        }
        assert_some_eq!(statuses.start_executing(txn_idx).unwrap(), 0);

        assert_eq!(
            status.status_with_incarnation.lock().status,
            SchedulingStatus::Executing
        );
        assert_simple_status_state(status, 0, 0, DependencyStatus::WaitForExecution as u8);

        // Compatible with finish(0) & finish_abort(0) only. Here, we test finish.
        assert_none!(statuses.start_executing(txn_idx).unwrap());
        assert_err!(statuses.finish_abort(txn_idx, 1, true));
        assert_err!(statuses.finish_execution(txn_idx, 1));
        if stall_before_finish {
            assert_ok_eq!(statuses.add_stall(txn_idx), true);
        }
        assert_ok!(statuses.finish_execution(txn_idx, 0));

        assert_eq!(
            status.status_with_incarnation.lock().status,
            SchedulingStatus::Executed
        );
        assert_simple_status_state(
            status,
            if stall_before_finish { 1 } else { 0 },
            0,
            if stall_before_finish {
                DependencyStatus::ShouldDefer as u8
            } else {
                DependencyStatus::IsSafe as u8
            },
        );

        // Compatible with abort(0) only.
        assert_none!(statuses.start_executing(txn_idx).unwrap());
        assert_err!(statuses.finish_execution(txn_idx, 0));
        assert_err!(statuses.finish_execution(txn_idx, 1));
        assert_err!(statuses.finish_abort(txn_idx, 1, true));

        statuses
            .execution_queue_manager
            .assert_execution_queue(&vec![]);
        assert_ok_eq!(statuses.start_abort(txn_idx, 0), true);
        assert_ok!(statuses.finish_abort(txn_idx, 0, true));
        if stall_before_finish {
            // Not rescheduled - deferred for remove_stall.
            statuses
                .execution_queue_manager
                .assert_execution_queue(&vec![]);
        }

        assert_ok_eq!(statuses.start_abort(txn_idx, 0), false);
        check_after_finish_and_abort(&statuses, txn_idx, 1, stall_before_finish);
    }

    #[test_case(false)]
    #[test_case(true)]
    fn status_cycle_with_abort_and_resolve(stall_before_finish: bool) {
        let txn_idx = 0;
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new(),
            ]);
        let status = statuses.get_status(txn_idx);

        *status.status_with_incarnation.lock() =
            StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 5);
        status.next_incarnation_to_abort.store(5, Ordering::Relaxed);
        assert_simple_status_state(status, 0, 5, DependencyStatus::ShouldDefer as u8);

        // Compatible with start (incompatible with abort and finish).
        for i in 0..5 {
            // Outdated call.
            assert_ok_eq!(statuses.start_abort(txn_idx, i), false);
            assert_err!(statuses.finish_abort(txn_idx, i, true));
            // Must have been called already to get to incarnation 5.
            assert_err!(statuses.finish_execution(txn_idx, i));
            // Impossible calls before 5 has even started execution.
            assert_err!(statuses.finish_execution(txn_idx, 5 + i));
            assert_err!(statuses.finish_abort(txn_idx, 5 + i, true));
        }
        assert_some_eq!(statuses.start_executing(txn_idx).unwrap(), 5);

        assert_eq!(
            *status.status_with_incarnation.lock(),
            StatusWithIncarnation::new_for_test(SchedulingStatus::Executing, 5)
        );
        assert_simple_status_state(status, 0, 5, DependencyStatus::WaitForExecution as u8);

        // Compatible with finish(5) & finish_abort(5) only. Here, we test abort.
        assert_none!(statuses.start_executing(txn_idx).unwrap());
        assert_ok_eq!(statuses.start_abort(txn_idx, 4), false);
        assert_err!(statuses.finish_abort(txn_idx, 4, true));
        assert_err!(statuses.finish_execution(txn_idx, 4));
        assert_err!(statuses.finish_execution(txn_idx, 6));
        assert_err!(statuses.finish_abort(txn_idx, 6, true));

        assert_eq!(status.next_incarnation_to_abort.load(Ordering::Relaxed), 5);
        assert_ok_eq!(statuses.start_abort(txn_idx, 5), true);
        assert_eq!(status.next_incarnation_to_abort.load(Ordering::Relaxed), 6);
        assert_ok!(statuses.finish_abort(txn_idx, 5, true));
        assert_eq!(status.next_incarnation_to_abort.load(Ordering::Relaxed), 6);
        assert_eq!(status.status_with_incarnation.lock().incarnation(), 5);
        // Not re-scheduled because finish_execution has not happened.
        statuses
            .execution_queue_manager
            .assert_execution_queue(&vec![]);

        assert_eq!(
            *status.status_with_incarnation.lock(),
            StatusWithIncarnation::new_for_test(SchedulingStatus::Aborted, 5)
        );
        // Compatible w. finish_execution(5) only.
        assert_none!(statuses.start_executing(txn_idx).unwrap());
        assert_ok_eq!(statuses.start_abort(txn_idx, 5), false);
        assert_err!(statuses.finish_abort(txn_idx, 5, true));
        assert_err!(statuses.finish_execution(txn_idx, 4));
        assert_err!(statuses.finish_execution(txn_idx, 6));
        assert_err!(statuses.finish_abort(txn_idx, 6, true));

        if stall_before_finish {
            assert_ok_eq!(statuses.add_stall(txn_idx), true);
        }
        // Finish execution from aborted, must return Ok(false).
        assert_ok_eq!(statuses.start_abort(txn_idx, 5), false);
        assert_err!(statuses.finish_abort(txn_idx, 5, true));
        assert_ok_eq!(statuses.finish_execution(txn_idx, 5), false);
        assert_eq!(status.status_with_incarnation.lock().incarnation(), 6);

        check_after_finish_and_abort(&statuses, 0, 6, stall_before_finish);
    }

    #[test]
    fn status_with_incarnation() {
        let status = StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 5);
        assert_eq!(status.incarnation(), 5);
        assert!(!status.is_executed());
        assert_some_eq!(status.pending_scheduling(), 5);
        assert!(status.already_aborted(4));
        assert!(status.already_aborted(1));
        assert!(!status.already_aborted(5));
        assert!(status.never_started_execution(5));
        assert!(status.never_started_execution(6));
        assert!(!status.never_started_execution(0));
        assert!(!status.never_started_execution(4));

        let status = StatusWithIncarnation::new_for_test(SchedulingStatus::Executing, 6);
        assert_eq!(status.incarnation(), 6);
        assert!(!status.is_executed());
        assert_none!(status.pending_scheduling());
        assert!(status.already_aborted(5));
        assert!(status.already_aborted(0));
        assert!(!status.already_aborted(6));
        assert!(status.never_started_execution(7));
        assert!(!status.never_started_execution(6));
        assert!(!status.never_started_execution(0));

        let status = StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 7);
        assert_eq!(status.incarnation(), 7);
        assert!(status.is_executed());
        assert_none!(status.pending_scheduling());
        assert!(status.already_aborted(6));
        assert!(status.already_aborted(2));
        assert!(!status.already_aborted(7));
        assert!(status.never_started_execution(8));
        assert!(!status.never_started_execution(7));
        assert!(!status.never_started_execution(0));

        let status = StatusWithIncarnation::new_for_test(SchedulingStatus::Aborted, 8);
        assert_eq!(status.incarnation(), 8);
        assert!(!status.is_executed());
        assert_none!(status.pending_scheduling());
        assert!(status.already_aborted(8));
        assert!(status.already_aborted(3));
        assert!(!status.already_aborted(9));
        assert!(status.never_started_execution(9));
        assert!(!status.never_started_execution(8));
        assert!(!status.never_started_execution(1));
    }

    #[test]
    fn stall_executed_status() {
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 5),
                    0,
                ),
            ]);
        let executed_status = statuses.get_status(0);

        // Assert correct starting state - provided by new_for_test.
        executed_status
            .dependency_shortcut
            .store(DependencyStatus::IsSafe as u8, Ordering::Relaxed);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 0);

        assert_ok_eq!(statuses.add_stall(0), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 1);

        // Adding stalls to an on already stalled status: return false.
        assert_ok_eq!(statuses.add_stall(0), false);
        assert_ok_eq!(statuses.add_stall(0), false);
        assert_ok_eq!(statuses.add_stall(0), false);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 4);

        assert_ok_eq!(statuses.remove_stall(0), false);
        assert_ok_eq!(statuses.remove_stall(0), false);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 2);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        assert_ok_eq!(statuses.remove_stall(0), false);
        assert_ok_eq!(statuses.remove_stall(0), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::IsSafe as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 0);

        assert_ok_eq!(statuses.add_stall(0), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(statuses.remove_stall(0), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::IsSafe as u8
        );
        assert_ok_eq!(statuses.add_stall(0), true);
        assert_ok_eq!(statuses.add_stall(0), false);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 2);
        assert_ok_eq!(statuses.remove_stall(0), false);
        assert_ok_eq!(statuses.remove_stall(0), true);
        assert_err!(statuses.remove_stall(0));
    }

    #[test_case(false)]
    #[test_case(true)]
    fn stall_executing_or_aborted(executing: bool) {
        let (status, expected_flag) = if executing {
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executing, 5),
                    0,
                ),
                DependencyStatus::WaitForExecution as u8,
            )
        } else {
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Aborted, 5),
                    0,
                ),
                DependencyStatus::ShouldDefer as u8,
            )
        };

        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![status]);
        let status = statuses.get_status(0);

        // add_stalls work normally, but without changing dependency shortcut flag.
        assert_ok_eq!(statuses.add_stall(0), true);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(statuses.add_stall(0), false);
        assert_ok_eq!(statuses.add_stall(0), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 3);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );

        // remove_stalls work normally, but w.o. changing the dependency shortcut flag.
        assert_ok_eq!(statuses.remove_stall(0), false);
        assert_ok_eq!(statuses.remove_stall(0), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );
        assert_ok_eq!(statuses.remove_stall(0), true);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 0);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );

        assert_err!(statuses.remove_stall(0));
    }

    #[test]
    fn add_remove_stall_simple_scheduling() {
        let executed_once_max_idx = 1;
        let statuses = ExecutionStatuses::new_for_test(
            ExecutionQueueManager::new_for_test(executed_once_max_idx),
            vec![
                ExecutionStatus::new(),
                ExecutionStatus::new(),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    1,
                ),
            ],
        );
        let manager = &statuses.get_execution_queue_manager();

        assert_err!(statuses.add_stall(0));
        assert_err!(statuses.add_stall(1));
        assert_ok_eq!(statuses.remove_stall(2), true);
        // Not re-scheduled (new incarnation = 1 with idx > executed_once_max_idx).
        manager.assert_execution_queue(&vec![]);

        manager.add_to_schedule(false, 2);
        manager.assert_execution_queue(&vec![2]);
        assert_ok_eq!(statuses.add_stall(2), true);
        manager.assert_execution_queue(&vec![]);
    }

    #[test_case(1, 9)]
    #[test_case(1, 10)]
    #[test_case(2, 8)]
    #[test_case(2, 12)]
    #[test_case(2, 10)]
    fn stall_pending_scheduling(incarnation: Incarnation, txn_idx: TxnIndex) {
        let mut statuses = ExecutionStatuses::new_for_test(
            ExecutionQueueManager::new_for_test(10),
            (0..20).map(|_| ExecutionStatus::new()).collect(),
        );

        *statuses.get_status_mut(txn_idx) = ExecutionStatus::new_for_test(
            StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, incarnation),
            0,
        );
        let manager = &statuses.get_execution_queue_manager();
        let status = &statuses.get_status(txn_idx);

        // add_stalls work normally, but without changing dependency shortcut flag.
        manager.add_to_schedule(false, txn_idx);
        manager.assert_execution_queue(&vec![txn_idx]);
        assert_ok_eq!(statuses.add_stall(txn_idx), true);
        manager.assert_execution_queue(&vec![]);

        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(statuses.add_stall(txn_idx), false);
        assert_ok_eq!(statuses.add_stall(txn_idx), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 3);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );

        // remove_stalls work normally, but w.o. changing the dependency shortcut flag.
        assert_ok_eq!(statuses.remove_stall(txn_idx), false);
        assert_ok_eq!(statuses.remove_stall(txn_idx), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        manager.assert_execution_queue(&vec![]);

        // Similar (inverted) test flow for remove_stalls.
        assert_ok_eq!(statuses.remove_stall(txn_idx), true);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 0);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
        manager.assert_execution_queue(&vec![txn_idx]);
        assert_err!(statuses.remove_stall(txn_idx));
    }

    fn set_shortcut_to_safe_or_provided(
        status: &ExecutionStatus,
        case: bool,
        provided_shortcut: u8,
    ) {
        status.dependency_shortcut.store(
            if case {
                DependencyStatus::IsSafe as u8
            } else {
                provided_shortcut
            },
            Ordering::Relaxed,
        );
    }

    #[test_case(false)]
    #[test_case(true)]
    fn set_executing_flag_err(case: bool) {
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new(),
            ]);
        let status = &statuses.get_status(0);
        // Breaking the invariant, not changing status from PendingScheduling
        // but updating dependency shortcut flag.
        set_shortcut_to_safe_or_provided(status, case, DependencyStatus::WaitForExecution as u8);
        assert_err!(statuses.start_executing(0));
    }

    #[test_case(true)]
    #[test_case(false)]
    fn to_pending_scheduling(add_to_schedule: bool) {
        let mut statuses = ExecutionStatuses::new_for_test(
            ExecutionQueueManager::new_for_test(10),
            (0..20).map(|_| ExecutionStatus::new()).collect(),
        );

        // Statuses for which txn should not get rescheduled:
        // - stalled,
        // - new_incarnation = 1 with idx > 10.
        for (status, txn_idx) in [
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 0),
                    1,
                ),
                9u32,
            ),
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 0),
                    0,
                ),
                12u32,
            ),
        ] {
            *statuses.get_status_mut(txn_idx) = status;
            let status = &statuses.get_status(txn_idx);
            assert!(!statuses.pending_scheduling_and_not_stalled(txn_idx));
            assert_eq!(status.status_with_incarnation.lock().incarnation(), 0);

            statuses.to_pending_scheduling(
                txn_idx,
                &mut status.status_with_incarnation.lock(),
                1,
                add_to_schedule,
            );

            assert_eq!(status.status_with_incarnation.lock().incarnation(), 1);
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DependencyStatus::ShouldDefer as u8
            );
            assert_eq!(
                statuses.pending_scheduling_and_not_stalled(txn_idx),
                !status.is_stalled()
            );
            statuses
                .get_execution_queue_manager()
                .assert_execution_queue(&vec![]);
        }

        // Finally, should be scheduled:
        // - new incarnation > 1 with idx < 10
        // - new incarnation > 1 with idx > 10
        // - new incarnation = 1 with idx = 10
        let mut expected_queue = vec![];
        for (status, txn_idx) in [
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 5),
                    0,
                ),
                8u32,
            ),
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 5),
                    0,
                ),
                13u32,
            ),
            (
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 0),
                    0,
                ),
                10u32,
            ),
        ] {
            *statuses.get_status_mut(txn_idx) = status;
            let status = &statuses.get_status(txn_idx);
            // Double-check expected state - provided by new_for_test
            assert!(statuses.is_executed(txn_idx));
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DependencyStatus::IsSafe as u8
            );
            assert!(!statuses.pending_scheduling_and_not_stalled(txn_idx));

            let new_incarnation = status.next_incarnation_to_abort.load(Ordering::Relaxed) + 1;
            statuses.to_pending_scheduling(
                txn_idx,
                &mut status.status_with_incarnation.lock(),
                new_incarnation,
                add_to_schedule,
            );

            assert_eq!(
                status.status_with_incarnation.lock().incarnation(),
                new_incarnation
            );
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DependencyStatus::ShouldDefer as u8
            );
            assert_eq!(
                statuses.pending_scheduling_and_not_stalled(txn_idx),
                !status.is_stalled()
            );

            if add_to_schedule {
                expected_queue.push(txn_idx);
            }
            statuses
                .get_execution_queue_manager()
                .assert_execution_queue(&expected_queue);
        }
    }

    #[test_case(DependencyStatus::IsSafe)]
    #[test_case(DependencyStatus::ShouldDefer)]
    fn assert_finish_execution_status(dependency_status: DependencyStatus) {
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new(),
            ]);
        let status = &statuses.get_status(0);
        // Convert to Executing state
        assert_some_eq!(statuses.start_executing(0).unwrap(), 0);
        // Break the invariant: reset only the dependency shortcut flag.
        status
            .dependency_shortcut
            .store(dependency_status as u8, Ordering::Relaxed);

        assert_err!(statuses.finish_execution(0, 0));
    }

    #[test]
    fn remove_stall_err_senarios() {
        let mut statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new(),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 1),
                    0,
                ),
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::PendingScheduling, 0),
                    1,
                ),
            ]);

        for wrong_shortcut in [DependencyStatus::WaitForExecution as u8, 100] {
            *statuses.get_status_mut(0) = ExecutionStatus::new_for_test(
                StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 0),
                2,
            );

            // remove_stall succeeds as it should.
            assert_ok_eq!(statuses.remove_stall(0), false);
            assert_eq!(statuses.get_status(0).num_stalls.load(Ordering::Relaxed), 1);

            statuses
                .get_status_mut(0)
                .dependency_shortcut
                .store(wrong_shortcut, Ordering::Relaxed);
            // Normal removal that would otherwise succeed should now return an error.
            assert_err!(statuses.remove_stall(0));
        }

        // Number of stalls = 0 for txn 1.
        assert_err!(statuses.remove_stall(1));
        // Incarnation 0 / err for txn 2.
        assert_err!(statuses.remove_stall(2));
    }

    #[test]
    fn remove_stall_recheck() {
        // Executed and stalled status.
        let statuses =
            ExecutionStatuses::new_for_test(ExecutionQueueManager::new_for_test(1), vec![
                ExecutionStatus::new_for_test(
                    StatusWithIncarnation::new_for_test(SchedulingStatus::Executed, 0),
                    1,
                ),
            ]);
        let status = &statuses.get_status(0);

        rayon::scope(|s| {
            // Acquire the lock to stop remove_stall call.
            let guard = status.status_with_incarnation.lock();

            s.spawn(|_| {
                // false due to the main thread incrementing stall count below.
                assert_ok_eq!(statuses.remove_stall(0), false);
            });

            while status.num_stalls.load(Ordering::Relaxed) != 0 {}
            status.num_stalls.fetch_add(1, Ordering::Relaxed);
            drop(guard);
        });

        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyStatus::ShouldDefer as u8
        );
    }
}
