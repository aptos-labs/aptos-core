// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// TODO(BlockSTMv2): enable dead code lint.
#![allow(dead_code)]

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
     the `try_start_executing` method.

2. Abort Process:
   - A transaction incarnation may be aborted if it reads data that is later modified in a way
     that would cause the transaction to read different values if it executed again. This
     signals the need for re-execution with an incremented incarnation number.
   - In BlockSTMv2, a transaction can be aborted while executing or after execution finishes.
   - Abort happens in two distinct phases:

   a) Try Abort Phase:
      - `try_abort` is called with an incarnation number and succeeds if the incarnation has
        started executing and has not already been aborted.
      - This serves as an efficient test-and-set filter for multiple abort attempts (which
        can occur when a transaction makes multiple reads that may each be invalidated by
        different transactions).
      - Early detection allows the ongoing execution to stop immediately rather than continue
        work that will ultimately be discarded.

   b) Finish Abort Phase:
      - A successful `try_abort` must be followed by a `finish_abort` call.
      - When transaction T1 successfully aborts transaction T2 (where T2 > T1):
        • T2 stops executing as soon as possible
        • Subsequent scheduling of T2 may wait until T1 finishes, since T1 has higher
          priority (lower index)
        • After T1 completes, the worker can process all related aborts in batch, including
          calling `finish_abort`, tracking dependencies, and propagating stalls

3. Execution Completion:
   - When execution finishes, `finish_execution` is called on the status.
   - If the status was `Aborted`, it transitions to `PendingScheduling` for the next incarnation.
   - If the status was `Executing`, it transitions to `Executed`.

Status Transition Diagram:

PendingScheduling(i)
    |
    | try_start_executing
    |
    ↓                       finish_execution
Executing(i) ------------------------------> Executed(i)
    |                                           |
    | start_abort(i) + finish_abort(i)            | start_abort(i) + finish_abort(i)
    |                                           |
    ↓                    finish_execution       ↓
Aborted(i) ------------------------------> PendingScheduling(i+1)

Note: `try_abort` doesn't change the status directly but marks the transaction for
abort. The actual status change occurs during `finish_abort`. Both steps are
required to complete the abort process.

============================== Transaction Stall Mechanism ==============================

In the BlockSTMv2 scheduler, a transaction status can be "stalled," meaning there have been
more `add_stall` than `remove_stall` calls on its status. Each successful `add_stall` call
requires a guarantee that the corresponding `remove_stall` will eventually be performed.

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
**/

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StatusEnum {
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
pub(crate) struct InnerStatus {
    status: StatusEnum,
    incarnation: Incarnation,
}

impl InnerStatus {
    fn new() -> Self {
        Self {
            status: StatusEnum::PendingScheduling,
            incarnation: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(status: StatusEnum, incarnation: Incarnation) -> Self {
        Self {
            status,
            incarnation,
        }
    }

    fn try_start_executing(&mut self) -> Option<Incarnation> {
        if self.status == StatusEnum::PendingScheduling {
            self.status = StatusEnum::Executing;
            return Some(self.incarnation);
        }
        None
    }

    fn incarnation(&self) -> Incarnation {
        self.incarnation
    }

    fn never_started_execution(&self, incarnation: Incarnation) -> bool {
        self.incarnation < incarnation
            || (self.incarnation == incarnation && self.status == StatusEnum::PendingScheduling)
    }

    fn already_aborted(&self, incarnation: Incarnation) -> bool {
        self.incarnation > incarnation
            || (self.incarnation == incarnation && self.status == StatusEnum::Aborted)
    }

    fn pending_scheduling(&self) -> Option<Incarnation> {
        (self.status == StatusEnum::PendingScheduling).then_some(self.incarnation)
    }

    fn is_executed(&self) -> bool {
        self.status == StatusEnum::Executed
    }

    fn ever_executed(&self) -> bool {
        // Aborted w. incarnation 0 is not considered as ever executed, because aborted
        // is set on start_abort, and incarnation 0 is prioritized in the scheduler to
        // actually finish execution / not early abort (to produce a speculative write-set).
        self.incarnation > 0 || self.status == StatusEnum::Executed
    }
}

/// Flag values for dependency resolution stored in an `AtomicU8` to allow lock-free reads.
/// These values represent the state of a transaction that other transactions depend on.
/// The flags are updated while holding the status lock but provide a fast way to evaluate
/// a predicate associated with the status that enables the scheduler to make decisions about
/// stall propagation, transaction scheduling, and dependency resolution.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum DependencyFlag {
    /// The transaction has successfully executed and is not stalled.
    /// Reading values written by this dependency is safe.
    Safe = 0,
    /// The transaction is currently executing.
    ///
    /// In this case, it may be beneficial to wait for the execution to finish
    /// to obtain up-to-date values rather than proceeding with potentially
    /// stale data. This is especially relevant for pipelining high-priority
    /// transaction execution to avoid aborts from reading outdated values.
    Executing = 1,
    /// This occurs when:
    /// 1. The transaction is in Aborted or PendingScheduling state (not yet re-scheduled)
    /// 2. The transaction is Executed but stalled (has an active dependency chain
    ///    that previously triggered an abort and may do so again)
    Defer = 2,
}

impl DependencyFlag {
    fn from_u8(flag: u8) -> Result<Self, PanicError> {
        match flag {
            0 => Ok(Self::Safe),
            1 => Ok(Self::Executing),
            2 => Ok(Self::Defer),
            _ => Err(code_invariant_error(format!(
                "Invalid dependency flag: {}",
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
pub(crate) struct ExecutionStatus<'a> {
    /// Protects access to the incarnation and inner status.
    ///
    /// This mutex synchronizes writes to incarnation and status changes, as well
    /// as modifications that affect the dependency shortcut (e.g., when stall count
    /// changes between 0 and non-zero).
    inner_status: CachePadded<Mutex<InnerStatus>>,

    /// Counter to track and filter abort attempts.
    ///
    /// This counter is monotonically increasing and updated in a successful start_abort.
    /// It allows filtering fanned-out abort attempts when multiple workers executing
    /// different transactions invalidate different reads of the same transaction.
    /// Only one of these workers will successfully abort the transaction and perform
    /// the required processing.
    next_incarnation_to_abort: CachePadded<AtomicU32>,

    /// Part of inner status state summarized as a single flag that can be read lock-free.
    /// The allowed values are defined in the `dependency_flags` module.
    dependency_shortcut: CachePadded<AtomicU8>,

    /// Tracks the number of active stalls on this transaction.
    ///
    /// A transaction is considered "stalled" when this count is greater than 0.
    /// Each add_stall increments this counter, and each remove_stall decrements it.
    /// The status is "unstalled" when the counter returns to 0.
    num_stalls: CachePadded<AtomicU32>,

    /// Interface to manage the transaction execution queue.
    ///
    /// Allows adding or removing transactions from the execution queue based on
    /// their status changes. Used when stalls are added/removed or when
    /// a new incarnation is created.
    execution_queue_manager: &'a ExecutionQueueManager,

    /// Index of this transaction in the block.
    txn_idx: TxnIndex,
}

impl<'a> ExecutionStatus<'a> {
    pub(crate) fn new(
        execution_queue_manager: &'a ExecutionQueueManager,
        txn_idx: TxnIndex,
    ) -> Self {
        Self {
            inner_status: CachePadded::new(Mutex::new(InnerStatus::new())),
            next_incarnation_to_abort: CachePadded::new(AtomicU32::new(0)),
            dependency_shortcut: CachePadded::new(AtomicU8::new(DependencyFlag::Defer as u8)),
            num_stalls: CachePadded::new(AtomicU32::new(0)),
            execution_queue_manager,
            txn_idx,
        }
    }

    /// Attempts to transition a transaction from PendingScheduling to Executing state.
    ///
    /// This method is called by the scheduler when it selects a transaction for execution.
    /// It only succeeds if the transaction is currently in PendingScheduling state.
    ///
    /// # Returns
    /// - `Ok(Some(incarnation))` if the transition was successful, returning the incarnation number
    /// - `Ok(None)` if the transaction is not in PendingScheduling state
    /// - `Err(PanicError)` if there was an error during flag transition
    pub(crate) fn try_start_executing(&self) -> Result<Option<Incarnation>, PanicError> {
        let inner_status = &mut *self.inner_status.lock();
        let ret = inner_status.try_start_executing();

        if ret.is_some() {
            // When status is PendingScheduling the dependency shortcut flag should be
            // DEFER (default or set by abort under the inner status lock).
            self.swap_dependency_flag_any(
                &[DependencyFlag::Defer],
                DependencyFlag::Executing,
                "try_start_executing",
            )?;
        }

        Ok(ret)
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
        finished_incarnation: Incarnation,
    ) -> Result<bool, PanicError> {
        // TODO(BlockSMTv2): Handle waiting workers when supported (defer waking up).

        let inner_status = &mut *self.inner_status.lock();

        // An incarnation of a transaction can only increase when both finish_execution and
        // try_abort take effect for the prior incarnation. However, finish_execution is
        // invoked once per incarnation, and thus incarnations must always match.
        if inner_status.incarnation() != finished_incarnation {
            return Err(code_invariant_error(format!(
                "Finish execution of incarnation {}, but inner status {:?}",
                finished_incarnation, inner_status,
            )));
        }

        match inner_status.status {
            StatusEnum::Executing => {
                inner_status.status = StatusEnum::Executed;

                let new_flag = if self.num_stalls.load(Ordering::Relaxed) == 0 {
                    DependencyFlag::Safe
                } else {
                    DependencyFlag::Defer
                };
                self.swap_dependency_flag_any(
                    &[DependencyFlag::Executing],
                    new_flag,
                    "finish_execution",
                )?;

                Ok(true)
            },
            StatusEnum::Aborted => {
                self.incarnate(inner_status, finished_incarnation + 1, false);
                Ok(false)
            },
            StatusEnum::PendingScheduling | StatusEnum::Executed => {
                Err(code_invariant_error(format!(
                    "Status update to Executed failed, previous inner status {:?}",
                    inner_status
                )))
            },
        }
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
    pub(crate) fn start_abort(&self, incarnation: Incarnation) -> Result<bool, PanicError> {
        let prev_value = self
            .next_incarnation_to_abort
            .fetch_max(incarnation + 1, Ordering::Relaxed);
        match incarnation.cmp(&prev_value) {
            cmp::Ordering::Less => Ok(false),
            cmp::Ordering::Equal => Ok(true),
            cmp::Ordering::Greater => Err(code_invariant_error(format!(
                "Try abort incarnation {} > self.next_incarnation_to_try_abort = {}",
                incarnation, prev_value,
            ))),
        }
    }

    /// Completes the abort of a transaction incarnation. It is the second step of
    /// the two-step abort process. It must be called after a successful try_abort and
    /// updates the transaction's status (o.w. it is a PanicError):
    /// - If Executing → Aborted
    /// - If Executed → PendingScheduling with incremented incarnation
    ///
    /// # Parameters
    /// - `aborted_incarnation`: The incarnation being aborted
    /// - `caller_reexecuting`: Whether the calling worker will handle re-execution
    ///
    /// # Returns
    /// - `Ok(())` if abort was completed successfully
    /// - `Err` if the abort can't be completed (e.g., wrong incarnation or status)
    pub(crate) fn finish_abort(
        &self,
        aborted_incarnation: Incarnation,
        caller_reexecuting: bool,
    ) -> Result<(), PanicError> {
        let new_incarnation = aborted_incarnation + 1;
        if self.next_incarnation_to_abort.load(Ordering::Relaxed) != new_incarnation {
            // The caller must have already successfully performed a start_abort, while
            // higher incarnation may not have started until the abort finished (here).
            return Err(code_invariant_error(format!(
                "Finish abort of incarnation {}, self.next_incarnation_to_abort = {}",
                aborted_incarnation,
                self.next_incarnation_to_abort.load(Ordering::Relaxed),
            )));
        }

        {
            let inner_status = &mut *self.inner_status.lock();
            if inner_status.already_aborted(aborted_incarnation)
                || inner_status.never_started_execution(aborted_incarnation)
            {
                return Err(code_invariant_error(format!(
                    "Finish abort of incarnation {}, but inner status {:?}",
                    aborted_incarnation, inner_status
                )));
            }

            match inner_status.status {
                StatusEnum::Executing => {
                    inner_status.status = StatusEnum::Aborted;
                    self.swap_dependency_flag_any(
                        &[DependencyFlag::Executing],
                        DependencyFlag::Defer,
                        "finish_abort",
                    )?;
                },
                StatusEnum::Executed => {
                    self.incarnate(inner_status, new_incarnation, caller_reexecuting);
                },
                StatusEnum::PendingScheduling | StatusEnum::Aborted => {
                    return Err(code_invariant_error(format!(
                        "Status update to Aborted failed, previous inner status {:?}",
                        inner_status
                    )));
                },
            }
        }

        Ok(())
    }

    /// Adds a stall to the transaction, indicating it has dependencies that might cause re-execution.
    ///
    /// When a transaction is stalled, it is removed from the execution queue if in PendingScheduling
    /// state, or its dependency shortcut is updated from SAFE to DEFER if in Executed state.
    ///
    /// # Returns
    /// - `Ok(true)` if this call changed the state from unstalled to stalled (num_stalls 0→1)
    /// - `Ok(false)` if the transaction was already stalled or a race condition occurred
    /// - `Err(PanicError)` if there was an invalid state (e.g., inconsistent status and flag)
    ///
    /// # Note
    /// Each successful add_stall must be balanced by a corresponding remove_stall call that starts
    /// after add_stall finishes.
    pub(crate) fn add_stall(&self) -> Result<bool, PanicError> {
        if self.num_stalls.fetch_add(1, Ordering::SeqCst) == 0 {
            // Acquire write lock for (non-monitor) shortcut modifications.
            let inner_status = self.inner_status.lock();

            let dependency_flag =
                DependencyFlag::from_u8(self.dependency_shortcut.load(Ordering::Relaxed))?;

            match (inner_status.pending_scheduling(), dependency_flag) {
                (Some(0), DependencyFlag::Defer) => {
                    // Adding a stall requires being recorded in aborted depedencies in scheduler_v2,
                    // which in turn only happens in the scheduler after a successful abort (that must
                    // increment the incarnation of the status).
                    return Err(code_invariant_error("0-th incarnation in add_stall"));
                },
                (Some(_), DependencyFlag::Defer) => {
                    self.execution_queue_manager
                        .remove_from_schedule(self.txn_idx);
                    // Shortcut not affected.
                },
                (Some(_), DependencyFlag::Safe | DependencyFlag::Executing) => {
                    return Err(code_invariant_error(
                        "Inconsistent status and dependency shortcut in add_stall",
                    ));
                },
                (None, DependencyFlag::Safe) => {
                    // May not update SAFE flag at a future incorrect time (i.e. ABA), as observing
                    // num_stalls = 0 under status is required to set SAFE flag, but impossible
                    // until the corresponding remove_stall (that starts only after add_stall finishes).
                    self.dependency_shortcut
                        .store(DependencyFlag::Defer as u8, Ordering::Relaxed);
                },
                (None, DependencyFlag::Executing | DependencyFlag::Defer) => {
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
    pub(crate) fn remove_stall(&self) -> Result<bool, PanicError> {
        let prev_num_stalls = self.num_stalls.fetch_sub(1, Ordering::SeqCst);

        if prev_num_stalls == 0 {
            return Err(code_invariant_error(
                "remove_stall called when num_stalls == 0",
            ));
        }

        if prev_num_stalls == 1 {
            // Acquire write lock for (non-monitor) shortcut modifications.
            let inner_status = self.inner_status.lock();

            // num_stalls updates are not under the lock, so need to re-check (otherwise
            // a different add_stall might have already incremented the count).
            if self.num_stalls.load(Ordering::Relaxed) > 0 {
                return Ok(false);
            }

            if let Some(incarnation) = inner_status.pending_scheduling() {
                if incarnation == 0 {
                    // Invariant due to scheduler logic: for a successful remove_stall there
                    // must have been an add_stall for incarnation 0, which is impossible.
                    return Err(code_invariant_error("0-th incarnation in remove_stall"));
                }
                self.execution_queue_manager
                    .add_to_schedule(incarnation == 1, self.txn_idx);
            } else if inner_status.is_executed() {
                // TODO(BlockSMTv2): Here, when waiting is supported, if inner status is executed,
                // would need to notify waiting workers.

                // Status is Executed so the dependency shortcut flag may not be
                // EXECUTING (finish_execution sets Executed status and DEFER or SAFE flag).
                self.swap_dependency_flag_any(
                    &[DependencyFlag::Defer, DependencyFlag::Safe],
                    DependencyFlag::Safe,
                    "remove_stall",
                )?;
            }

            return Ok(true);
        }
        Ok(false)
    }

    /// Checks if an incarnation has already been marked for abort.
    ///
    /// This can be called during an ongoing execution to determine if the
    /// execution has been concurrently aborted. This allows the executor
    /// to return early and to discard the results.
    pub(crate) fn already_try_aborted(&self, incarnation: Incarnation) -> bool {
        self.next_incarnation_to_abort.load(Ordering::Relaxed) > incarnation
    }

    /// Checks that the shortcut flag is SAFE. This is used by the scheduler
    /// when removing a previously propagated stall signal, and it is safe to
    /// use the shortcutbecause of the best-effort nature of the stall mechanism.
    pub(crate) fn shortcut_executed_and_not_stalled(&self) -> bool {
        self.dependency_shortcut.load(Ordering::Relaxed) == DependencyFlag::Safe as u8
    }

    /// Checks if the transaction is ready for scheduling and not stalled.
    /// This is used to determine if a transaction should be considered
    /// for execution by the scheduler.
    pub(crate) fn pending_scheduling_and_not_stalled(&self) -> bool {
        let guard = self.inner_status.lock();
        guard.pending_scheduling().is_some() && self.num_stalls.load(Ordering::Relaxed) == 0
    }

    // === More inner status wrappers ===

    pub(crate) fn is_executed(&self) -> bool {
        self.inner_status.lock().is_executed()
    }

    pub(crate) fn ever_executed(&self) -> bool {
        self.inner_status.lock().ever_executed()
    }

    pub(crate) fn incarnation(&self) -> Incarnation {
        self.inner_status.lock().incarnation()
    }

    /// Gets the current status of this transaction.
    pub(crate) fn status(&self) -> StatusEnum {
        self.inner_status.lock().status.clone()
    }
}

// Private interfaces.
impl ExecutionStatus<'_> {
    /// Performs an atomic swap operation on the dependency shortcut flag and validates
    /// that the previous value matches one of the expected values.
    ///
    /// # Parameters
    /// - `expected_values`: Array of possible expected current flag values
    /// - `new_value`: The new flag value to set
    /// - `context`: A string describing the context for error messages
    ///
    /// # Returns
    /// - `Ok(prev)` if the swap was successful, returning the previous value
    /// - `Err(PanicError)` if the previous value didn't match any expected value
    fn swap_dependency_flag_any(
        &self,
        expected_values: &[DependencyFlag],
        new_value: DependencyFlag,
        context: &str,
    ) -> Result<DependencyFlag, PanicError> {
        let prev = DependencyFlag::from_u8(
            self.dependency_shortcut
                .swap(new_value as u8, Ordering::Relaxed),
        )?;
        if !expected_values.contains(&prev) {
            return Err(code_invariant_error(format!(
                "Incorrect dependency shortcut flag in {}: expected one of {:?}, found {:?}",
                context, expected_values, prev,
            )));
        }
        Ok(prev)
    }

    // Updates inner status to PendingScheduling with the new incarnation & update shortcuts.
    fn incarnate(
        &self,
        inner_status: &mut InnerStatus,
        new_incarnation: Incarnation,
        caller_reexecuting: bool,
    ) {
        // Update inner status.
        inner_status.status = StatusEnum::PendingScheduling;
        inner_status.incarnation = new_incarnation;

        // Under the lock, update the shortcuts.
        self.dependency_shortcut
            .store(DependencyFlag::Defer as u8, Ordering::Relaxed);

        if !caller_reexecuting && self.num_stalls.load(Ordering::Relaxed) == 0 {
            // Need to schedule the transaction for re-execution. If num_stalls > 0, then
            // scheduling is deferred to the remove_stall.
            self.execution_queue_manager
                .add_to_schedule(new_incarnation == 1, self.txn_idx);
        }
    }
}

// Testing interfaces.
#[cfg(test)]
impl<'a> ExecutionStatus<'a> {
    pub(crate) fn new_for_test(
        inner_status: InnerStatus,
        num_stalls: u32,
        manager: &'a ExecutionQueueManager,
        txn_idx: TxnIndex,
    ) -> Self {
        let incarnation = inner_status.incarnation();
        let shortcut = match inner_status.status {
            StatusEnum::PendingScheduling | StatusEnum::Aborted => DependencyFlag::Defer as u8,
            StatusEnum::Executing => DependencyFlag::Executing as u8,
            StatusEnum::Executed => {
                if num_stalls == 0 {
                    DependencyFlag::Safe as u8
                } else {
                    DependencyFlag::Defer as u8
                }
            },
        };
        Self {
            inner_status: CachePadded::new(Mutex::new(inner_status)),
            next_incarnation_to_abort: CachePadded::new(AtomicU32::new(incarnation)),
            dependency_shortcut: CachePadded::new(AtomicU8::new(shortcut)),
            num_stalls: CachePadded::new(AtomicU32::new(num_stalls)),
            execution_queue_manager: manager,
            txn_idx,
        }
    }

    pub(crate) fn is_stalled(&self) -> bool {
        self.num_stalls.load(Ordering::Relaxed) > 0
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
        assert_eq!(status.inner_status.lock().incarnation, exp_incarnation);
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
        status: &ExecutionStatus,
        expected_incarnation: Incarnation,
        manager: &ExecutionQueueManager,
        stall_before_finish: bool,
    ) {
        assert_eq!(
            status.inner_status.lock().status,
            StatusEnum::PendingScheduling
        );
        assert_simple_status_state(
            status,
            if stall_before_finish { 1 } else { 0 },
            expected_incarnation,
            DependencyFlag::Defer as u8,
        );

        if stall_before_finish {
            assert_ok_eq!(status.remove_stall(), true);
        }
        manager.assert_execution_queue(&vec![status.txn_idx]);
    }

    #[test_case(false)]
    #[test_case(true)]
    fn status_cycle_with_finish_and_resolve(stall_before_finish: bool) {
        let txn_idx = 50;
        let manager = ExecutionQueueManager::new_for_test(txn_idx);
        let status = ExecutionStatus::new(&manager, txn_idx);

        assert_eq!(
            status.inner_status.lock().status,
            StatusEnum::PendingScheduling
        );
        assert_simple_status_state(&status, 0, 0, DependencyFlag::Defer as u8);

        // Compatible with start (incompatible with abort and finish).
        for i in [0, 2] {
            assert_err!(status.finish_execution(i));
            assert_err!(status.finish_abort(i, false));
        }
        assert_some_eq!(status.try_start_executing().unwrap(), 0);

        assert_eq!(status.inner_status.lock().status, StatusEnum::Executing);
        assert_simple_status_state(&status, 0, 0, DependencyFlag::Executing as u8);

        // Compatible with finish(0) & finish_abort(0) only. Here, we test finish.
        assert_none!(status.try_start_executing().unwrap());
        assert_err!(status.finish_abort(1, false));
        assert_err!(status.finish_execution(1));
        if stall_before_finish {
            assert_ok_eq!(status.add_stall(), true);
        }
        assert_ok!(status.finish_execution(0));

        assert_eq!(status.inner_status.lock().status, StatusEnum::Executed);
        assert_simple_status_state(
            &status,
            if stall_before_finish { 1 } else { 0 },
            0,
            if stall_before_finish {
                DependencyFlag::Defer as u8
            } else {
                DependencyFlag::Safe as u8
            },
        );

        // Compatible with abort(0) only.
        assert_none!(status.try_start_executing().unwrap());
        assert_err!(status.finish_execution(0));
        assert_err!(status.finish_execution(1));
        assert_err!(status.finish_abort(1, false));

        manager.assert_execution_queue(&vec![]);
        assert_ok_eq!(status.start_abort(0), true);
        assert_ok!(status.finish_abort(0, false));
        if stall_before_finish {
            // Not rescheduled - deferred for remove_stall.
            manager.assert_execution_queue(&vec![]);
        }

        assert_ok_eq!(status.start_abort(0), false);
        check_after_finish_and_abort(&status, 1, &manager, stall_before_finish);
    }

    #[test_case(false)]
    #[test_case(true)]
    fn status_cycle_with_abort_and_resolve(stall_before_finish: bool) {
        let txn_idx = 50;
        let manager = ExecutionQueueManager::new_for_test(txn_idx);
        let status = ExecutionStatus::new(&manager, txn_idx);
        *status.inner_status.lock() = InnerStatus::new_for_test(StatusEnum::PendingScheduling, 5);
        status.next_incarnation_to_abort.store(5, Ordering::Relaxed);
        assert_simple_status_state(&status, 0, 5, DependencyFlag::Defer as u8);

        // Compatible with start (incompatible with abort and finish).
        for i in 0..5 {
            // Outdated call.
            assert_ok_eq!(status.start_abort(i), false);
            assert_err!(status.finish_abort(i, false));
            // Must have been called already to get to incarnation 5.
            assert_err!(status.finish_execution(i));
            // Impossible calls before 5 has even started execution.
            assert_err!(status.finish_execution(5 + i));
            assert_err!(status.finish_abort(5 + i, false));
        }
        assert_some_eq!(status.try_start_executing().unwrap(), 5);

        assert_eq!(
            *status.inner_status.lock(),
            InnerStatus::new_for_test(StatusEnum::Executing, 5)
        );
        assert_simple_status_state(&status, 0, 5, DependencyFlag::Executing as u8);

        // Compatible with finish(5) & finish_abort(5) only. Here, we test abort.
        assert_none!(status.try_start_executing().unwrap());
        assert_ok_eq!(status.start_abort(4), false);
        assert_err!(status.finish_abort(4, false));
        assert_err!(status.finish_execution(4));
        assert_err!(status.finish_execution(6));
        assert_err!(status.finish_abort(6, false));

        assert_eq!(status.next_incarnation_to_abort.load(Ordering::Relaxed), 5);
        assert_ok_eq!(status.start_abort(5), true);
        assert_eq!(status.next_incarnation_to_abort.load(Ordering::Relaxed), 6);
        assert_ok!(status.finish_abort(5, false));
        assert_eq!(status.next_incarnation_to_abort.load(Ordering::Relaxed), 6);
        assert_eq!(status.inner_status.lock().incarnation(), 5);
        // Not re-scheduled because finish_execution has not happened.
        manager.assert_execution_queue(&vec![]);

        assert_eq!(
            *status.inner_status.lock(),
            InnerStatus::new_for_test(StatusEnum::Aborted, 5)
        );
        // Compatible w. finish_execution(5) only.
        assert_none!(status.try_start_executing().unwrap());
        assert_ok_eq!(status.start_abort(5), false);
        assert_err!(status.finish_abort(5, false));
        assert_err!(status.finish_execution(4));
        assert_err!(status.finish_execution(6));
        assert_err!(status.finish_abort(6, false));

        if stall_before_finish {
            assert_ok_eq!(status.add_stall(), true);
        }
        // Finish execution from aborted, must return Ok(false).
        assert_ok_eq!(status.start_abort(5), false);
        assert_err!(status.finish_abort(5, false));
        assert_ok_eq!(status.finish_execution(5), false);
        assert_eq!(status.inner_status.lock().incarnation(), 6);

        check_after_finish_and_abort(&status, 6, &manager, stall_before_finish);
    }

    #[test]
    fn inner_status() {
        let status = InnerStatus::new_for_test(StatusEnum::PendingScheduling, 5);
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

        let status = InnerStatus::new_for_test(StatusEnum::Executing, 6);
        assert_eq!(status.incarnation(), 6);
        assert!(!status.is_executed());
        assert_none!(status.pending_scheduling());
        assert!(status.already_aborted(5));
        assert!(status.already_aborted(0));
        assert!(!status.already_aborted(6));
        assert!(status.never_started_execution(7));
        assert!(!status.never_started_execution(6));
        assert!(!status.never_started_execution(0));

        let status = InnerStatus::new_for_test(StatusEnum::Executed, 7);
        assert_eq!(status.incarnation(), 7);
        assert!(status.is_executed());
        assert_none!(status.pending_scheduling());
        assert!(status.already_aborted(6));
        assert!(status.already_aborted(2));
        assert!(!status.already_aborted(7));
        assert!(status.never_started_execution(8));
        assert!(!status.never_started_execution(7));
        assert!(!status.never_started_execution(0));

        let status = InnerStatus::new_for_test(StatusEnum::Aborted, 8);
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
        let manager = ExecutionQueueManager::new_for_test(10);
        let executed_status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executed, 5),
            0,
            &manager,
            10,
        );

        // Assert correct starting state - provided by new_for_test.
        executed_status
            .dependency_shortcut
            .store(DependencyFlag::Safe as u8, Ordering::Relaxed);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 0);

        assert_ok_eq!(executed_status.add_stall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 1);

        // Adding stalls to an on already stalled status: return false.
        assert_ok_eq!(executed_status.add_stall(), false);
        assert_ok_eq!(executed_status.add_stall(), false);
        assert_ok_eq!(executed_status.add_stall(), false);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 4);

        assert_ok_eq!(executed_status.remove_stall(), false);
        assert_ok_eq!(executed_status.remove_stall(), false);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 2);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        assert_ok_eq!(executed_status.remove_stall(), false);
        assert_ok_eq!(executed_status.remove_stall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Safe as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 0);

        assert_ok_eq!(executed_status.add_stall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(executed_status.remove_stall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Safe as u8
        );
        assert_ok_eq!(executed_status.add_stall(), true);
        assert_ok_eq!(executed_status.add_stall(), false);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 2);
        assert_ok_eq!(executed_status.remove_stall(), false);
        assert_ok_eq!(executed_status.remove_stall(), true);
        assert_err!(executed_status.remove_stall());
    }

    #[test_case(false)]
    #[test_case(true)]
    fn stall_executing_or_aborted(case: bool) {
        let manager = ExecutionQueueManager::new_for_test(10);
        let (status, expected_flag) = if case {
            (
                ExecutionStatus::new_for_test(
                    InnerStatus::new_for_test(StatusEnum::Executing, 5),
                    0,
                    &manager,
                    10,
                ),
                DependencyFlag::Executing as u8,
            )
        } else {
            (
                ExecutionStatus::new_for_test(
                    InnerStatus::new_for_test(StatusEnum::Aborted, 5),
                    0,
                    &manager,
                    10,
                ),
                DependencyFlag::Defer as u8,
            )
        };

        // add_stalls work normally, but without changing dependency shortcut flag.
        assert_ok_eq!(status.add_stall(), true);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(status.add_stall(), false);
        assert_ok_eq!(status.add_stall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 3);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );

        // remove_stalls work normally, but w.o. changing the dependency shortcut flag.
        assert_ok_eq!(status.remove_stall(), false);
        assert_ok_eq!(status.remove_stall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );
        assert_ok_eq!(status.remove_stall(), true);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 0);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );

        assert_err!(status.remove_stall());
    }

    #[test]
    fn add_remove_stall_simple_scheduling() {
        let manager = ExecutionQueueManager::new_for_test(10);
        let status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 0),
            0,
            &manager,
            10,
        );
        assert_err!(status.add_stall());
        let status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            1,
            &manager,
            11,
        );
        assert_ok_eq!(status.remove_stall(), true);
        // Should not have been re-scheduled (new incarnation = 1 with idx > 10)
        manager.assert_execution_queue(&vec![]);

        manager.add_to_schedule(false, 11);
        manager.assert_execution_queue(&vec![11]);
        assert_ok_eq!(status.add_stall(), true);
        manager.assert_execution_queue(&vec![]);
    }

    #[test_case(1, 9)]
    #[test_case(1, 10)]
    #[test_case(2, 8)]
    #[test_case(2, 12)]
    #[test_case(2, 10)]
    fn stall_pending_scheduling(incarnation: Incarnation, txn_idx: TxnIndex) {
        let manager = ExecutionQueueManager::new_for_test(10);
        let status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, incarnation),
            0,
            &manager,
            txn_idx,
        );
        // add_stalls work normally, but without changing dependency shortcut flag.
        manager.add_to_schedule(false, txn_idx);
        manager.assert_execution_queue(&vec![txn_idx]);
        assert_ok_eq!(status.add_stall(), true);
        manager.assert_execution_queue(&vec![]);

        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(status.add_stall(), false);
        assert_ok_eq!(status.add_stall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 3);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );

        // remove_stalls work normally, but w.o. changing the dependency shortcut flag.
        assert_ok_eq!(status.remove_stall(), false);
        assert_ok_eq!(status.remove_stall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        manager.assert_execution_queue(&vec![]);

        // Similar (inverted) test flow for remove_stalls.
        assert_ok_eq!(status.remove_stall(), true);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 0);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
        manager.assert_execution_queue(&vec![txn_idx]);
        assert_err!(status.remove_stall());
    }

    fn set_shortcut_to_safe_or_provided(
        status: &ExecutionStatus,
        case: bool,
        provided_shortcut: u8,
    ) {
        status.dependency_shortcut.store(
            if case {
                DependencyFlag::Safe as u8
            } else {
                provided_shortcut
            },
            Ordering::Relaxed,
        );
    }

    #[test_case(false)]
    #[test_case(true)]
    fn set_executing_flag_err(case: bool) {
        let manager = ExecutionQueueManager::new_for_test(10);
        let status = ExecutionStatus::new(&manager, 10);
        // Breaking the invariant, not changing status from PendingScheduling
        // but updating dependency shortcut flag.
        set_shortcut_to_safe_or_provided(&status, case, DependencyFlag::Executing as u8);

        // Should now panic.
        assert_err!(status.try_start_executing());
    }

    #[test]
    fn incarnate() {
        let txn_idx = 10;

        // Statuses for which txn should not get rescheduled:
        // - stalled,
        // - new_incarnation = 1 with idx > 10.
        let manager = ExecutionQueueManager::new_for_test(txn_idx);
        for status in [
            ExecutionStatus::new_for_test(
                InnerStatus::new_for_test(StatusEnum::Executed, 0),
                1,
                &manager,
                9,
            ),
            ExecutionStatus::new_for_test(
                InnerStatus::new_for_test(StatusEnum::Executed, 0),
                0,
                &manager,
                12,
            ),
        ] {
            assert!(!status.pending_scheduling_and_not_stalled());
            assert_eq!(status.inner_status.lock().incarnation(), 0);

            status.incarnate(&mut status.inner_status.lock(), 1, false);

            assert_eq!(status.inner_status.lock().incarnation(), 1);
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DependencyFlag::Defer as u8
            );
            assert_eq!(
                status.pending_scheduling_and_not_stalled(),
                !status.is_stalled()
            );
            manager.assert_execution_queue(&vec![]);
        }

        // Finally, should be scheduled:
        // - new incarnation > 1 with idx < 10
        // - new incarnation > 1 with idx > 10
        // - new incarnation = 1 with idx = 10
        let manager = ExecutionQueueManager::new_for_test(txn_idx);
        let mut expected_queue = vec![];
        for status in [
            ExecutionStatus::new_for_test(
                InnerStatus::new_for_test(StatusEnum::Executed, 5),
                0,
                &manager,
                8,
            ),
            ExecutionStatus::new_for_test(
                InnerStatus::new_for_test(StatusEnum::Executed, 5),
                0,
                &manager,
                13,
            ),
            ExecutionStatus::new_for_test(
                InnerStatus::new_for_test(StatusEnum::Executed, 0),
                0,
                &manager,
                10,
            ),
        ] {
            // Double-check expected state - provided by new_for_test
            assert!(status.is_executed());
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DependencyFlag::Safe as u8
            );
            assert!(!status.pending_scheduling_and_not_stalled());

            let new_incarnation = status.next_incarnation_to_abort.load(Ordering::Relaxed) + 1;
            status.incarnate(&mut status.inner_status.lock(), new_incarnation, false);

            assert_eq!(status.inner_status.lock().incarnation(), new_incarnation);
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DependencyFlag::Defer as u8
            );
            assert_eq!(
                status.pending_scheduling_and_not_stalled(),
                !status.is_stalled()
            );

            expected_queue.push(status.txn_idx);
            manager.assert_execution_queue(&expected_queue);
        }
    }

    #[test_case(false)]
    #[test_case(true)]
    fn assert_finish_execution_status(case: bool) {
        let manager = ExecutionQueueManager::new_for_test(10);
        let status = ExecutionStatus::new(&manager, 10);
        // Convert to Executing state
        assert_some_eq!(status.try_start_executing().unwrap(), 0);
        // Break the invariant: reset only the dependency shortcut flag.
        status.dependency_shortcut.store(
            if case {
                DependencyFlag::Safe as u8
            } else {
                DependencyFlag::Defer as u8
            },
            Ordering::Relaxed,
        );

        assert_err!(status.finish_execution(0));
    }

    #[test]
    fn remove_stall_err_senarios() {
        let manager = ExecutionQueueManager::new_for_test(10);

        for wrong_shortcut in [DependencyFlag::Executing as u8, 100] {
            let status = ExecutionStatus::new_for_test(
                InnerStatus::new_for_test(StatusEnum::Executed, 0),
                2,
                &manager,
                10,
            );
            // remove_stall succeeds as it should.
            assert_ok_eq!(status.remove_stall(), false);
            assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);

            status
                .dependency_shortcut
                .store(wrong_shortcut, Ordering::Relaxed);
            // Normal removal that would otherwise succeed should now return an error.
            assert_err!(status.remove_stall());
        }

        let status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 1),
            0,
            &manager,
            10,
        );
        // Number of stalls = 0.
        assert_err!(status.remove_stall());

        let status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::PendingScheduling, 0),
            1,
            &manager,
            10,
        );
        // Incarnation 0 / err.
        assert_err!(status.remove_stall());
    }

    #[test]
    fn remove_stall_recheck() {
        // Executed and stalled status.
        let manager = ExecutionQueueManager::new_for_test(10);
        let status = ExecutionStatus::new_for_test(
            InnerStatus::new_for_test(StatusEnum::Executed, 0),
            1,
            &manager,
            10,
        );

        rayon::scope(|s| {
            // Acquire the lock to stop remove_stall call.
            let inner_status_guard = status.inner_status.lock();

            s.spawn(|_| {
                // false due to the main thread incrementing stall count below.
                assert_ok_eq!(status.remove_stall(), false);
            });

            while status.num_stalls.load(Ordering::Relaxed) != 0 {}
            status.num_stalls.fetch_add(1, Ordering::Relaxed);
            drop(inner_status_guard);
        });

        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DependencyFlag::Defer as u8
        );
    }
}
