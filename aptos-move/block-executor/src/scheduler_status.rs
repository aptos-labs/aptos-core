// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::{
    scheduler::{DependencyCondvar, DependencyStatus},
    scheduler_v2::SchedulerProxy,
};
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::error::{code_invariant_error, PanicError};
use crossbeam::utils::CachePadded;
use std::sync::{
    atomic::{AtomicU32, AtomicU8, Ordering},
    Arc, Condvar,
};

/**
In BlockSTMv2 scheduler, a transaction can be `stalled' and `unstalled'. A successful call
to the stall method requires a commitment from the caller to eventually perform the
corresponding `unstall' (remove the stall).

The intended use for the stall mechanism (as the name suggests, and similar to typical uses)
is for the caller to record that the given transaction has (a chain of) dependencies that
are likely to cause re-execution (e.g. have previously aborted and changed a value). This
information can then be used to (a) not reschedule the transaction for re-execution until
stall is cleared (corresponding unstalls have occurred), or (b) help determine the right
handling when another transaction observes a dependency during execution. In both cases,
the goal is to constraint optimistic concurrency by limiting the impact of cascading aborts.

The mechanism is best-effort, i.e. the caller can decide to incarnate / re-execute a
transaction even in the stalled state, which can be useful e.g. if the transaction has a
high enough priority (proximity to the committed prefix). The best-effort nature also
allows for a more relaxed handling of different concurrency scenarios.
**/

// Integer flag values stored in an atomic variable to allow lock-free reads. The flag is
// updated while holding the status lock and captures a predicate evaluated on the status.
// - SAFE flag means that the status is Executed and not stalled, implying that reading a
// value written by the dependency is safe.
const DEPENDENCY_SAFE_FLAG: u8 = 0;
// - EXECUTING flag means that the dependency (status) is executing. In this case, it is
// more likely that waiting for the dependency to finish the ongoing execution and produce
// up-to-date values could make sense (e.g. to pipeline high priority transaction execution
// and not risk aborting due to reading outdated values).
const DEPENDENCY_EXECUTING_FLAG: u8 = 1;
// - DEFER flag means that values written by the dependency are not safe, but if they were
// to change, the new incarnation has not yet started executing. This state can occur when
// status is Aborted or RequiresExecution (i.e. not yet re-scheduled for re-execution), or
// when status is Executed but stalled - i.e. there is an active dependency chain that has
// already triggered an abort earlier (and may again do so).
const DEPENDENCY_DEFER_FLAG: u8 = 2;

// Dependency instruction is provided by the caller (scheduler) to the resolve_dependency
// call on the status. Based on status and instruction, the call returns a resolution.
#[derive(Clone, Copy)]
pub(crate) enum DependencyInstruction {
    // Used for the first call when if the shortcut it SAFE (executed & not stalled status),
    // SafeToProceed resolution is returned. This is to speed up the good-case path, i.e.
    // not require the caller to query and provide additional information (via instruction).
    Default,
    // If the shortcut is not SAFE, return the wait resolution (condvar that notifies when
    // the next incarnation finishes execution, or a finished execution unstalls).
    Wait,
    // If the shortcut is EXECUTING, return the wait resolution.
    WaitForExecuting,
}

// When transaction A observes a R/W dependency on transaction B, it makes a call to
// resolve to B's status. DependencyResolution is returned, whereby it might be recommended
// to proceed with a read, wait, or let the caller (scheduler) use its own heuristics.
#[derive(Debug)]
pub(crate) enum DependencyResolution {
    // Transaction is executed, and no stalls: safe to proceed.
    SafeToProceed,
    // Transaction is executing and the caller has high priority: recommended to wait.
    // The provided conditional variable will be notified after the prescribed wait.
    Wait(DependencyCondvar),
    // Above conditions not met, the exact resolution deferred to the caller: this might
    // involve calling resolve_dependency again with an updated instruction.
    None,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InnerStatus {
    RequiresExecution(Incarnation),
    Executing(Incarnation),
    Aborted(Incarnation),
    Executed(Incarnation),
}

impl InnerStatus {
    fn incarnation(&self) -> Incarnation {
        use InnerStatus::*;
        match self {
            RequiresExecution(incarnation)
            | Executing(incarnation)
            | Aborted(incarnation)
            | Executed(incarnation) => *incarnation,
        }
    }

    fn never_started_execution(&self, incarnation: Incarnation) -> bool {
        let status_incarnation = self.incarnation();

        if status_incarnation < incarnation
            || (status_incarnation == incarnation
                && matches!(self, InnerStatus::RequiresExecution(_)))
        {
            return true;
        }
        false
    }

    fn already_aborted(&self, incarnation: Incarnation) -> bool {
        let status_incarnation = self.incarnation();

        if status_incarnation > incarnation
            || (status_incarnation == incarnation && matches!(self, InnerStatus::Aborted(_)))
        {
            return true;
        }
        false
    }

    fn requires_execution(&self) -> Option<Incarnation> {
        use InnerStatus::*;
        match self {
            RequiresExecution(incarnation) => Some(*incarnation),
            Executing(_) | Aborted(_) | Executed(_) => None,
        }
    }

    fn is_executed(&self) -> bool {
        matches!(self, InnerStatus::Executed(_))
    }
}

pub(crate) struct ExecutionStatus {
    // Synchronizes the writes to incarnation and inner status, as well as changes that affect
    // the dependency shortcut (e.g. when the number of stalls becomes 0 or >0). The dependency
    // shortcut and the number of stalls are separate atomic variables to allow lock-free reads.
    //
    // Inner status transition diagram:
    //
    // RequiresExecution(i)
    //     |
    //     | try_start_executing
    //     |
    //     ↓              finish_execution
    // Executing(i) ---------------------------> Executed(i)
    //     |                                          |
    //     | try_abort(i)                             | try_abort(i)
    //     |                                          |
    //     ↓           finish_execution               ↓
    // Aborted(i) ---------------------------> RequiresExecution(i+1)
    //
    inner_status: CachePadded<Mutex<InnerStatus>>,

    // It is guaranteed that each stall has a corresponding unstall, which occurs later.
    num_stalls: CachePadded<AtomicU32>,

    // Precomputated often-evaluated predicate provides a lock-free common path for the caller
    // (w. relaxed ordering and w.o. accessing more contended inner status) to decide on a proper
    // DependencyResolution. The shortcut value is updated while holding the inner_status
    // mutex, which acts as a write lock. To avoid lost wake ups in the the synchronization
    // between resolve dependency & finish_execution, relies on ordering with respect to the
    // waiting_queue mutex (below) acquisitions as well.
    dependency_shortcut: AtomicU8,
    // Abort incarnation shortcut is monotonically increasing, updated after inner status, and
    // allows lock-free fast-path for outdated aborts (on an already aborted version).
    abort_incarnation_shortcut: AtomicU32,

    // waiting queue is tied to the dependency shortcut. If a dependency resolution is to
    // wait, then a new condvar is stored in the queue and returned to the caller. When the
    // shortcut changes from wait / executing, the queue is drained, notifying the waiting
    // workers (after execution finishes, status changes, and shortcut is updated).
    waiting_queue: CachePadded<Mutex<Vec<DependencyCondvar>>>,

    // The proxy allows hooks to add or remove the corresponding transaction to the scheduler's
    // execution queue. Removing is needed after a stall, while after an unstall or creating a
    // new incarnation, the transaction is rescheduled for (re-)execution.
    // TODO: share add/remove implementation generically w. the scheduler.
    scheduler_proxy: Arc<SchedulerProxy>,
    txn_idx: TxnIndex,
}

impl ExecutionStatus {
    pub(crate) fn new(scheduler_proxy: Arc<SchedulerProxy>, txn_idx: TxnIndex) -> Self {
        Self {
            inner_status: CachePadded::new(Mutex::new(InnerStatus::RequiresExecution(0))),
            num_stalls: CachePadded::new(AtomicU32::new(0)),
            dependency_shortcut: AtomicU8::new(DEPENDENCY_DEFER_FLAG),
            abort_incarnation_shortcut: AtomicU32::new(0),
            waiting_queue: CachePadded::new(Mutex::new(Vec::new())),
            scheduler_proxy,
            txn_idx,
        }
    }

    // Does not check num_stalled, and tries updates RequiresExecution(incarnation) status
    // If successful, return Some(incarnation), o.w. None.
    // We do not provide incarnation as try_start_executing is assumed to be issued
    // sequentially: its precondition is RequiresExecution inner status, which itself
    // requires previous execution to have started and then aborted.
    pub(crate) fn try_start_executing(&self) -> Option<Incarnation> {
        let mut inner_status = self.inner_status.lock();
        if let InnerStatus::RequiresExecution(incarnation) = *inner_status {
            *inner_status = InnerStatus::Executing(incarnation);

            // When status is RequiresExecution the dependency shortcut flag ought to be
            // DEFER (default or set by abort under the inner status lock).
            assert_eq!(
                self.dependency_shortcut
                    .swap(DEPENDENCY_EXECUTING_FLAG, Ordering::Relaxed),
                DEPENDENCY_DEFER_FLAG,
                "Incorrect dependency shortcut flag in try_start_executing"
            );

            return Some(incarnation);
        }
        None
    }

    // Called once per transaction incarnation after its execution finishes. Does appropriate
    // checks and updates inner status (from Executing) to Executed, returning Ok(true), or
    // from Aborted to RequiresExecution (for the next incarnation), returning Ok(false).
    // Waiting dependencies are notified in all cases.
    pub(crate) fn finish_execution(
        &self,
        finished_incarnation: Incarnation,
    ) -> Result<bool, PanicError> {
        defer! {
            self.notify_waiting_workers();
        }

        {
            // It is important that all checks in this method that may early return without
            // updating the status occur within the inner_status lock (see resolve_dependency).
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

            match inner_status {
                InnerStatus::Executing(incarnation) => {
                    *inner_status = InnerStatus::Executed(*incarnation);

                    assert_eq!(
                        self.dependency_shortcut.swap(
                            if self.num_stalls.load(Ordering::Relaxed) == 0 {
                                DEPENDENCY_SAFE_FLAG
                            } else {
                                DEPENDENCY_DEFER_FLAG
                            },
                            Ordering::Relaxed,
                        ),
                        DEPENDENCY_EXECUTING_FLAG,
                        "Incorrect dependency shortcut flag in finish execution"
                    );
                    Ok(true)
                },
                InnerStatus::Aborted(_incarnation) => {
                    self.incarnate(inner_status, finished_incarnation + 1);
                    Ok(false)
                },
                InnerStatus::RequiresExecution(_) | InnerStatus::Executed(_) => {
                    Err(code_invariant_error(format!(
                        "Status update to Executed failed, previous inner status {:?}",
                        inner_status
                    )))
                },
            }
        }
    }

    // Returns whether the abort succeeded (the first try_abort), or PanicError.
    pub(crate) fn try_abort(&self, aborted_incarnation: Incarnation) -> Result<bool, PanicError> {
        if self.abort_incarnation_shortcut.load(Ordering::Relaxed) > aborted_incarnation {
            // Shortcut path: already aborted.
            return Ok(false);
        }

        {
            let inner_status = &mut *self.inner_status.lock();
            if inner_status.already_aborted(aborted_incarnation) {
                return Ok(false);
            }
            if inner_status.never_started_execution(aborted_incarnation) {
                return Err(code_invariant_error(format!(
                    "Status flag update to RequiresExecution failed, previous flag {:?}",
                    inner_status
                )));
            }

            match inner_status {
                InnerStatus::Executing(incarnation) => {
                    assert_eq!(*incarnation, aborted_incarnation);
                    *inner_status = InnerStatus::Aborted(*incarnation);
                    assert_eq!(
                        self.dependency_shortcut
                            .swap(DEPENDENCY_DEFER_FLAG, Ordering::Relaxed),
                        DEPENDENCY_EXECUTING_FLAG,
                        "Incorrect dependency shortcut flag in finish execution"
                    );
                    Ok(true)
                },
                InnerStatus::Executed(incarnation) => {
                    assert_eq!(*incarnation, aborted_incarnation);
                    self.incarnate(inner_status, aborted_incarnation + 1);
                    Ok(true)
                },
                InnerStatus::RequiresExecution(_) | InnerStatus::Aborted(_) => {
                    Err(code_invariant_error(format!(
                        "Status update to Executed failed, previous inner status {:?}",
                        inner_status
                    )))
                },
            }
        }
    }

    // If DependencyResolution::Wait(condvar) is returned, the caller is expected to make
    // another call after the condvar is notified to get an updated resolution. When the
    // resolution is SAFE_TO_PROCEED, this call has succeeded in providing a cheap decision.
    // Note that the call does not acquire status mutex, and in the common case (when it
    // is safe to proceed), it requires a single relaxed read.
    pub(crate) fn resolve_dependency(
        &self,
        instruction: DependencyInstruction,
    ) -> Result<DependencyResolution, PanicError> {
        let mut shortcut = self.dependency_shortcut.load(Ordering::Relaxed);
        for _ in 0..2 {
            use DependencyInstruction::*;

            match (shortcut, instruction) {
                (DEPENDENCY_SAFE_FLAG, Default | Wait | WaitForExecuting) => {
                    // Shortcut path: default proceed.
                    return Ok(DependencyResolution::SafeToProceed);
                },
                (DEPENDENCY_EXECUTING_FLAG, Wait | WaitForExecuting)
                | (DEPENDENCY_DEFER_FLAG, Wait) => {
                    // Create a condvar and push to the local queue for later notifying.
                    let dep_condvar =
                        Arc::new((Mutex::new(DependencyStatus::Unresolved), Condvar::new()));

                    let mut waiting = self.waiting_queue.lock();
                    // Re-check after acquiring the waiting queue lock to avoid lost wake-ups.
                    // Suppose the check below observes an 'executing' status. Then we must show
                    // that the corresponding finish_execution has not yet locked the queue and
                    // woken up contained dependencies.
                    //   - If finish_execution updates the status from 'executing', it does so
                    //   before locking the waiting queue, giving the desired contradiction.
                    //   - Otherwise, it must observe an already aborted state by another worker.
                    //   Since the observation & updates happen while holding the inner status
                    //   mutex, their ordering is transitive and the load below may not observe
                    //   already changed 'executing' status.
                    // If the status is Aborted or RequiresExecution, then the next incarnation
                    // has not even started executing (finish execution will happen afterwards).
                    // Finally, if status is executed but stalled (corresponding to DEFER flag),
                    // then an unstall must occur afterwards and will wake up dependencies.
                    shortcut = self.dependency_shortcut.load(Ordering::Relaxed);
                    if matches!(shortcut, DEPENDENCY_SAFE_FLAG)
                        || matches!(
                            (shortcut, instruction),
                            (DEPENDENCY_DEFER_FLAG, WaitForExecuting)
                        )
                    {
                        // Try again - has to return.
                        continue;
                    }
                    waiting.push(dep_condvar.clone());

                    return Ok(DependencyResolution::Wait(dep_condvar));
                },
                (DEPENDENCY_EXECUTING_FLAG, Default)
                | (DEPENDENCY_DEFER_FLAG, Default | WaitForExecuting) => {
                    return Ok(DependencyResolution::None)
                },
                (3..=u8::MAX, _) => {
                    return Err(code_invariant_error(format!(
                        "Incorrect value in dependency shortcut {}",
                        shortcut
                    )))
                },
            }
        }
        unreachable!("Must return from loop in 2 iterations");
    }

    // Returns true if this stall call changed the state, i.e. incremented num_stalls
    // from 0 to 1. Additionally, the scheduling hook is invoked on proxy status requires execution.
    // The corresponding unstall may not be called until the method returns.
    pub(crate) fn stall(&self) -> Result<bool, PanicError> {
        if self.num_stalls.fetch_add(1, Ordering::SeqCst) == 0 {
            // Acquire write lock for (non-monitor) shortcut modifications.
            let inner_status = self.inner_status.lock();

            match (
                inner_status.requires_execution(),
                self.dependency_shortcut.load(Ordering::Relaxed),
            ) {
                (Some(0), DEPENDENCY_DEFER_FLAG) => {
                    // Impossible stalling a status requires it to be recorded in aborted depedencies,
                    // which in turn only happens in the scheduler after a successful abort (that must
                    // increment the incarnation of the status).
                    return Err(code_invariant_error("0-th incarnation may not be stalled"));
                },
                (Some(_), DEPENDENCY_DEFER_FLAG) => {
                    self.scheduler_proxy.remove_from_schedule(self.txn_idx);
                    // Shortcut not affected.
                },
                (None, DEPENDENCY_SAFE_FLAG) => {
                    // May not update SAFE flag at a future incorrect time (i.e. ABA), as observing
                    // num_stalls = 0 under status is required to set SAFE flag, but impossible
                    // until the corresponding unstall (that starts only after this stall finishes).
                    self.dependency_shortcut
                        .store(DEPENDENCY_DEFER_FLAG, Ordering::Relaxed);
                },
                (None, DEPENDENCY_EXECUTING_FLAG | DEPENDENCY_DEFER_FLAG) => {
                    // Executing or aborted: shortcut not affected.
                },
                (Some(_), DEPENDENCY_SAFE_FLAG | DEPENDENCY_EXECUTING_FLAG) => {
                    return Err(code_invariant_error(
                        "Inconsistent status and dependency shortcut in stall",
                    ));
                },
                (_, unsupported_flag_value) => {
                    return Err(code_invariant_error(format!(
                        "Unsupported flag value {unsupported_flag_value} in stall",
                    )));
                },
            }

            return Ok(true);
        }
        Ok(false)
    }

    // Returns true if this unstall call changed the state, i.e. decreased num_stalls to 0.
    // If so, scheduling hook is also invoked on the proxy.
    pub(crate) fn unstall(&self) -> Result<bool, PanicError> {
        let prev_num_stalls = self.num_stalls.fetch_sub(1, Ordering::SeqCst);

        if prev_num_stalls == 0 {
            return Err(code_invariant_error("unstall called when num_stalls == 0"));
        }

        if prev_num_stalls == 1 {
            // Acquire write lock for (non-monitor) shortcut modifications.
            let inner_status = self.inner_status.lock();

            // num_stalls updates are not under the lock, so need to re-check (otherwise
            // a different stall might have already incremented the count.
            let not_stalled = self.num_stalls.load(Ordering::Relaxed) == 0;

            if not_stalled {
                if let Some(incarnation) = inner_status.requires_execution() {
                    if incarnation == 0 {
                        // Invariant due to scheduler logic: for a successful unstall there must
                        // have been a stall for incarnation 0, which is impossible (see above).
                        return Err(code_invariant_error(
                            "0-th incarnation may not be unstalled",
                        ));
                    }
                    self.scheduler_proxy
                        .add_to_schedule(incarnation == 1, self.txn_idx);
                } else if inner_status.is_executed() {
                    defer! {
                        self.notify_waiting_workers();
                    }

                    // Status is Executed so the dependency shortcut flag may not be
                    // EXECUTING (finish_execution sets Executed status and DEFER or SAFE flag).
                    let prev_flag = self
                        .dependency_shortcut
                        .swap(DEPENDENCY_SAFE_FLAG, Ordering::Relaxed);
                    if prev_flag != DEPENDENCY_SAFE_FLAG && prev_flag != DEPENDENCY_DEFER_FLAG {
                        return Err(code_invariant_error(format!(
                            "Incorrect flag value {prev_flag} in unstall",
                        )));
                    }
                }
            }
            return Ok(not_stalled);
        }
        Ok(false)
    }

    // Returns true if dependency shortcut is SAFE, i.e. executed & not stalled, and false
    // otherwise. This is useful to the caller for controlling recursive stalling / unstalling.
    pub(crate) fn shortcut_executed_and_not_stalled(&self) -> bool {
        matches!(
            self.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_SAFE_FLAG
        )
    }

    pub(crate) fn is_executed(&self) -> bool {
        self.inner_status.lock().is_executed()
    }

    pub(crate) fn requires_execution(&self) -> bool {
        matches!(*self.inner_status.lock(), InnerStatus::RequiresExecution(_))
    }

    pub(crate) fn ever_executed(&self) -> bool {
        use InnerStatus::*;
        match *self.inner_status.lock() {
            RequiresExecution(incarnation) | Executing(incarnation) | Aborted(incarnation) => {
                incarnation > 0
            },
            Executed(_) => true,
        }
    }
}

// Private interfaces.
impl ExecutionStatus {
    fn notify_waiting_workers(&self) {
        // Notify all workers that might be waiting, which will trigger a follow-up call
        // from the scheduler to re-attempt processing the dependency for the worker,
        // at which point the inner status mutex is already released and flag updated.
        let waiting: Vec<DependencyCondvar> = {
            let mut stored = self.waiting_queue.lock();
            // Holding the lock, take the vector.
            std::mem::take(&mut *stored)
        };
        for condvar in waiting {
            let (lock, cvar) = &*condvar;
            let mut lock = lock.lock();
            *lock = DependencyStatus::Resolved;
            cvar.notify_one();
        }
    }

    // Utility function updates inner status to RequiredExecution with the new incarnation, only
    // asserting that new incarnation > previously stored abort shortcut.
    fn incarnate(&self, inner_status: &mut InnerStatus, new_incarnation: Incarnation) {
        // Update inner status.
        *inner_status = InnerStatus::RequiresExecution(new_incarnation);

        // Under the lock, update the shortcuts.
        self.dependency_shortcut
            .store(DEPENDENCY_DEFER_FLAG, Ordering::Relaxed);
        let prev_incarnation = self
            .abort_incarnation_shortcut
            .swap(new_incarnation, Ordering::Relaxed);
        assert!(
            new_incarnation > prev_incarnation,
            "incarnate called for {}, but abort incarnation shortcut = {}",
            new_incarnation,
            prev_incarnation,
        );

        if self.num_stalls.load(Ordering::Relaxed) == 0 {
            // Need to schedule the transaction for re-execution. If num_stalls > 0, then
            // scheduling is deferred to the unstall.
            self.scheduler_proxy
                .add_to_schedule(new_incarnation == 1, self.txn_idx);
        }
    }
}

// Testing interfaces.
impl ExecutionStatus {
    #[cfg(test)]
    pub(crate) fn new_for_test(
        inner_status: InnerStatus,
        num_stalls: u32,
        proxy: &Arc<SchedulerProxy>,
        txn_idx: TxnIndex,
    ) -> Self {
        let incarnation = inner_status.incarnation();
        use InnerStatus::*;
        let shortcut = match inner_status {
            RequiresExecution(_) | Aborted(_) => DEPENDENCY_DEFER_FLAG,
            Executing(_) => DEPENDENCY_EXECUTING_FLAG,
            Executed(_) => {
                if num_stalls == 0 {
                    DEPENDENCY_SAFE_FLAG
                } else {
                    DEPENDENCY_DEFER_FLAG
                }
            },
        };
        Self {
            inner_status: CachePadded::new(Mutex::new(inner_status)),
            num_stalls: CachePadded::new(AtomicU32::new(num_stalls)),
            dependency_shortcut: AtomicU8::new(shortcut),
            abort_incarnation_shortcut: AtomicU32::new(incarnation),
            waiting_queue: CachePadded::new(Mutex::new(Vec::new())),
            scheduler_proxy: proxy.clone(),
            txn_idx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{
        assert_err, assert_matches, assert_none, assert_ok, assert_ok_eq, assert_some_eq,
    };
    use std::sync::atomic::AtomicBool;
    use test_case::test_case;

    fn assert_simple_status_state(
        status: &ExecutionStatus,
        exp_num_stalls: u32,
        exp_incarnation: Incarnation,
        exp_dependency_shortcut: u8,
    ) {
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), exp_num_stalls);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            exp_dependency_shortcut,
        );
        assert_eq!(
            status.abort_incarnation_shortcut.load(Ordering::Relaxed),
            exp_incarnation
        );
        assert_eq!(status.waiting_queue.lock().len(), 0);

        use DependencyInstruction::*;
        match exp_dependency_shortcut {
            DEPENDENCY_SAFE_FLAG => {
                for b in [Default, Wait, WaitForExecuting] {
                    assert_matches!(
                        status.resolve_dependency(b),
                        Ok(DependencyResolution::SafeToProceed)
                    );
                }
            },
            DEPENDENCY_DEFER_FLAG => {
                for b in [Default, WaitForExecuting] {
                    assert_matches!(status.resolve_dependency(b), Ok(DependencyResolution::None));
                }
                assert_matches!(
                    status.resolve_dependency(Wait),
                    Ok(DependencyResolution::Wait(_))
                );
            },
            DEPENDENCY_EXECUTING_FLAG => {
                for b in [Wait, WaitForExecuting] {
                    assert_matches!(
                        status.resolve_dependency(b),
                        Ok(DependencyResolution::Wait(_))
                    );
                }
                assert_matches!(
                    status.resolve_dependency(Default),
                    Ok(DependencyResolution::None)
                );
            },
            _ => unreachable!("Unused resolution code"),
        }
    }

    fn check_after_finish_and_abort(
        status: &ExecutionStatus,
        expected_incarnation: Incarnation,
        proxy: &SchedulerProxy,
        stall_before_finish: bool,
    ) {
        assert_eq!(
            *status.inner_status.lock(),
            InnerStatus::RequiresExecution(expected_incarnation)
        );
        assert_simple_status_state(
            &status,
            if stall_before_finish { 1 } else { 0 },
            expected_incarnation,
            DEPENDENCY_DEFER_FLAG,
        );

        if stall_before_finish {
            assert_ok_eq!(status.unstall(), true);
        }
        proxy.assert_execution_queue(&vec![status.txn_idx]);
    }

    #[test_case(false)]
    #[test_case(true)]
    fn status_cycle_with_finish_and_resolve(stall_before_finish: bool) {
        let txn_idx = 50;
        let proxy = Arc::new(SchedulerProxy::new_for_test(txn_idx));
        let status = ExecutionStatus::new(proxy.clone(), txn_idx);

        assert_eq!(
            *status.inner_status.lock(),
            InnerStatus::RequiresExecution(0)
        );
        assert_simple_status_state(&status, 0, 0, DEPENDENCY_DEFER_FLAG);

        // Compatible with start (incompatible with abort and finish).
        for i in [0, 2] {
            assert_err!(status.finish_execution(i));
            assert_err!(status.try_abort(i));
        }
        assert_some_eq!(status.try_start_executing(), 0);

        assert_eq!(*status.inner_status.lock(), InnerStatus::Executing(0));
        assert_simple_status_state(&status, 0, 0, DEPENDENCY_EXECUTING_FLAG);

        // Compatible with finish(0) & try_abort(0) only. Here, we test finish.
        assert_none!(status.try_start_executing());
        assert_err!(status.try_abort(1));
        assert_err!(status.finish_execution(1));
        if stall_before_finish {
            assert_ok_eq!(status.stall(), true);
        }
        assert_ok!(status.finish_execution(0));

        assert_eq!(*status.inner_status.lock(), InnerStatus::Executed(0));
        assert_simple_status_state(
            &status,
            if stall_before_finish { 1 } else { 0 },
            0,
            if stall_before_finish {
                DEPENDENCY_DEFER_FLAG
            } else {
                DEPENDENCY_SAFE_FLAG
            },
        );

        // Compatible with abort(0) only.
        assert_none!(status.try_start_executing());
        assert_err!(status.finish_execution(0));
        assert_err!(status.finish_execution(1));
        assert_err!(status.try_abort(1));

        proxy.assert_execution_queue(&vec![]);
        assert_ok_eq!(status.try_abort(0), true);
        if stall_before_finish {
            // Not rescheduled - deferred for unstall.
            proxy.assert_execution_queue(&vec![]);
        }

        assert_ok_eq!(status.try_abort(0), false);
        check_after_finish_and_abort(&status, 1, &proxy, stall_before_finish);
    }

    #[test_case(false)]
    #[test_case(true)]
    fn status_cycle_with_abort_and_resolve(stall_before_finish: bool) {
        let txn_idx = 50;
        let proxy = Arc::new(SchedulerProxy::new_for_test(txn_idx));
        let status = ExecutionStatus::new(proxy.clone(), txn_idx);
        *status.inner_status.lock() = InnerStatus::RequiresExecution(5);
        status
            .abort_incarnation_shortcut
            .store(5, Ordering::Relaxed);
        assert_simple_status_state(&status, 0, 5, DEPENDENCY_DEFER_FLAG);

        // Compatible with start (incompatible with abort and finish).
        for i in 0..5 {
            // Outdated call.
            assert_ok_eq!(status.try_abort(i), false);
            // Must have been called already to get to incarnation 5.
            assert_err!(status.finish_execution(i));
            // Impossible calls before 5 has even started execution.
            assert_err!(status.finish_execution(5 + i));
            assert_err!(status.try_abort(5 + i));
        }
        assert_some_eq!(status.try_start_executing(), 5);

        assert_eq!(*status.inner_status.lock(), InnerStatus::Executing(5));
        assert_simple_status_state(&status, 0, 5, DEPENDENCY_EXECUTING_FLAG);

        // Compatible with finish(5) & try_abort(5) only. Here, we test abort.
        assert_none!(status.try_start_executing());
        assert_ok_eq!(status.try_abort(4), false);
        assert_err!(status.finish_execution(4));
        assert_err!(status.finish_execution(6));
        assert_err!(status.try_abort(6));

        assert_ok_eq!(status.try_abort(5), true);
        // Not re-scheduled because finish_execution has not happened.
        proxy.assert_execution_queue(&vec![]);

        assert_eq!(*status.inner_status.lock(), InnerStatus::Aborted(5));
        // Compatible w. finish_execution(5) only.
        assert_none!(status.try_start_executing());
        assert_ok_eq!(status.try_abort(5), false);
        assert_err!(status.finish_execution(4));
        assert_err!(status.finish_execution(6));
        assert_err!(status.try_abort(6));

        if stall_before_finish {
            assert_ok_eq!(status.stall(), true);
        }
        // Finish execution from aborted, must return Ok(false).
        assert_ok_eq!(status.finish_execution(5), false);

        check_after_finish_and_abort(&status, 6, &proxy, stall_before_finish);
    }

    #[test_case(0)]
    #[test_case(1)]
    #[test_case(2)]
    fn status_waiting_queue(finish_scenario: u8) {
        let txn_idx = 10;
        let proxy = Arc::new(SchedulerProxy::new_for_test(txn_idx));
        let status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(5), 0, &proxy, txn_idx);

        assert_some_eq!(status.try_start_executing(), 5);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_EXECUTING_FLAG
        );

        assert_matches!(
            status.resolve_dependency(DependencyInstruction::Default),
            Ok(DependencyResolution::None)
        );
        assert_eq!(status.waiting_queue.lock().len(), 0);

        let barrier = AtomicU8::new(0);
        let case = AtomicBool::new(false);

        rayon::scope(|s| {
            for _ in 0..2 {
                s.spawn(|_| {
                    match status.resolve_dependency(if case.load(Ordering::Relaxed) {
                        DependencyInstruction::Wait
                    } else {
                        DependencyInstruction::WaitForExecuting
                    }) {
                        Ok(DependencyResolution::Wait(condvar)) => {
                            let (lock, cvar) = &*condvar;
                            let mut dep_resolved = lock.lock();
                            assert_matches!(&*dep_resolved, DependencyStatus::Unresolved);
                            barrier.fetch_add(1, Ordering::SeqCst);

                            while matches!(*dep_resolved, DependencyStatus::Unresolved) {
                                dep_resolved = cvar.wait(dep_resolved).unwrap();
                            }
                            assert_matches!(&*dep_resolved, DependencyStatus::Resolved);
                        },
                        _ => unreachable!("Incorrect resolution"),
                    }
                });
                case.store(true, Ordering::Relaxed);
            }

            while barrier.load(Ordering::SeqCst) < 2 {}

            if finish_scenario == 0 {
                assert_ok_eq!(status.try_abort(5), true);
            }
            assert_eq!(status.waiting_queue.lock().len(), 2);

            match finish_scenario {
                0 => {
                    assert_ok_eq!(status.finish_execution(5), false);
                },
                1 => {
                    assert_ok_eq!(status.finish_execution(5), true);
                },
                2 => {
                    assert_err!(status.finish_execution(6));
                },
                _ => unreachable!("Unsupported test scenario"),
            };
            assert_eq!(status.waiting_queue.lock().len(), 0);
        });
    }

    #[test]
    fn inner_status() {
        let status = InnerStatus::RequiresExecution(5);
        assert_eq!(status.incarnation(), 5);
        assert!(!status.is_executed());
        assert_some_eq!(status.requires_execution(), 5);
        assert!(status.already_aborted(4));
        assert!(status.already_aborted(1));
        assert!(!status.already_aborted(5));
        assert!(status.never_started_execution(5));
        assert!(status.never_started_execution(6));
        assert!(!status.never_started_execution(0));
        assert!(!status.never_started_execution(4));

        let status = InnerStatus::Executing(6);
        assert_eq!(status.incarnation(), 6);
        assert!(!status.is_executed());
        assert_none!(status.requires_execution());
        assert!(status.already_aborted(5));
        assert!(status.already_aborted(0));
        assert!(!status.already_aborted(6));
        assert!(status.never_started_execution(7));
        assert!(!status.never_started_execution(6));
        assert!(!status.never_started_execution(0));

        let status = InnerStatus::Executed(7);
        assert_eq!(status.incarnation(), 7);
        assert!(status.is_executed());
        assert_none!(status.requires_execution());
        assert!(status.already_aborted(6));
        assert!(status.already_aborted(2));
        assert!(!status.already_aborted(7));
        assert!(status.never_started_execution(8));
        assert!(!status.never_started_execution(7));
        assert!(!status.never_started_execution(0));

        let status = InnerStatus::Aborted(8);
        assert_eq!(status.incarnation(), 8);
        assert!(!status.is_executed());
        assert_none!(status.requires_execution());
        assert!(status.already_aborted(8));
        assert!(status.already_aborted(3));
        assert!(!status.already_aborted(9));
        assert!(status.never_started_execution(9));
        assert!(!status.never_started_execution(8));
        assert!(!status.never_started_execution(1));
    }

    #[test]
    fn stall_executed_status() {
        let proxy = Arc::new(SchedulerProxy::new_for_test(10));
        let executed_status =
            ExecutionStatus::new_for_test(InnerStatus::Executed(5), 0, &proxy, 10);

        // Assert correct starting state - provided by new_for_test.
        executed_status
            .dependency_shortcut
            .store(DEPENDENCY_SAFE_FLAG, Ordering::Relaxed);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 0);

        assert_ok_eq!(executed_status.stall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 1);

        // Adding stalls to an on already stalled status: return false.
        assert_ok_eq!(executed_status.stall(), false);
        assert_ok_eq!(executed_status.stall(), false);
        assert_ok_eq!(executed_status.stall(), false);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 4);

        assert_ok_eq!(executed_status.unstall(), false);
        assert_ok_eq!(executed_status.unstall(), false);
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 2);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        assert_ok_eq!(executed_status.unstall(), false);
        assert_ok_eq!(executed_status.unstall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_SAFE_FLAG
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 0);

        assert_ok_eq!(executed_status.stall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(executed_status.unstall(), true);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_SAFE_FLAG
        );
        assert_ok_eq!(executed_status.stall(), true);
        assert_ok_eq!(executed_status.stall(), false);
        assert_eq!(
            executed_status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        assert_eq!(executed_status.num_stalls.load(Ordering::Relaxed), 2);
        assert_ok_eq!(executed_status.unstall(), false);
        assert_ok_eq!(executed_status.unstall(), true);
        assert_err!(executed_status.unstall());
    }

    #[test_case(false)]
    #[test_case(true)]
    fn stall_executing_or_aborted(case: bool) {
        let proxy = Arc::new(SchedulerProxy::new_for_test(10));
        let (status, expected_flag) = if case {
            (
                ExecutionStatus::new_for_test(InnerStatus::Executing(5), 0, &proxy, 10),
                DEPENDENCY_EXECUTING_FLAG,
            )
        } else {
            (
                ExecutionStatus::new_for_test(InnerStatus::Aborted(5), 0, &proxy, 10),
                DEPENDENCY_DEFER_FLAG,
            )
        };

        // Stalls work normally, but without changing dependency shortcut flag.
        assert_ok_eq!(status.stall(), true);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(status.stall(), false);
        assert_ok_eq!(status.stall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 3);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );

        // Same with unstall.
        assert_ok_eq!(status.unstall(), false);
        assert_ok_eq!(status.unstall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );
        assert_ok_eq!(status.unstall(), true);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 0);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            expected_flag
        );

        assert_err!(status.unstall());
    }

    #[test]
    fn stall_unstall_simple_scheduling() {
        let proxy = Arc::new(SchedulerProxy::new_for_test(10));
        let status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(0), 0, &proxy, 10);
        assert_err!(status.stall());
        let status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(1), 1, &proxy, 11);
        assert_ok_eq!(status.unstall(), true);
        // Should not have been re-scheduled (new incarnation = 1 with idx > 10)
        proxy.assert_execution_queue(&vec![]);

        proxy.add_to_schedule(false, 11);
        proxy.assert_execution_queue(&vec![11]);
        assert_ok_eq!(status.stall(), true);
        proxy.assert_execution_queue(&vec![]);
    }

    #[test_case(1, 9)]
    #[test_case(1, 10)]
    #[test_case(2, 8)]
    #[test_case(2, 12)]
    #[test_case(2, 10)]
    fn stall_requires_execution(incarnation: Incarnation, txn_idx: TxnIndex) {
        let proxy = Arc::new(SchedulerProxy::new_for_test(10));
        let status = ExecutionStatus::new_for_test(
            InnerStatus::RequiresExecution(incarnation),
            0,
            &proxy,
            txn_idx,
        );
        // Stalls work normally, but without changing dependency shortcut flag.
        proxy.add_to_schedule(false, txn_idx);
        proxy.assert_execution_queue(&vec![txn_idx]);
        assert_ok_eq!(status.stall(), true);
        proxy.assert_execution_queue(&vec![]);

        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_ok_eq!(status.stall(), false);
        assert_ok_eq!(status.stall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 3);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );

        // Unstalls work normally, but w.o. changing the dependency shortcut flag.
        assert_ok_eq!(status.unstall(), false);
        assert_ok_eq!(status.unstall(), false);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        proxy.assert_execution_queue(&vec![]);

        // Similar (inverted) test flow for unstalls.
        assert_ok_eq!(status.unstall(), true);
        assert_eq!(status.num_stalls.load(Ordering::Relaxed), 0);
        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
        proxy.assert_execution_queue(&vec![txn_idx]);
        assert_err!(status.unstall());
    }

    fn set_shortcut_to_safe_or_provided(
        status: &ExecutionStatus,
        case: bool,
        provided_shortcut: u8,
    ) {
        status.dependency_shortcut.store(
            if case {
                DEPENDENCY_SAFE_FLAG
            } else {
                provided_shortcut
            },
            Ordering::Relaxed,
        );
    }

    #[should_panic]
    #[test_case(false)]
    #[test_case(true)]
    fn assert_set_executing_flag(case: bool) {
        let status = ExecutionStatus::new(Arc::new(SchedulerProxy::new_for_test(10)), 10);
        // Breaking the invariant, not changing status from RequiresExecution
        // but updating dependency shortcut flag.
        set_shortcut_to_safe_or_provided(&status, case, DEPENDENCY_EXECUTING_FLAG);

        // Should now panic.
        status.try_start_executing();
    }

    #[should_panic]
    fn incarnate_check() {
        let status = ExecutionStatus::new(Arc::new(SchedulerProxy::new_for_test(10)), 10);
        status.incarnate(&mut *status.inner_status.lock(), 0);
    }

    #[test]
    fn incarnate() {
        let txn_idx = 10;

        // Statuses for which txn should not get rescheduled:
        // - stalled,
        // - new_incarnation = 1 with idx > 10.
        let proxy = Arc::new(SchedulerProxy::new_for_test(txn_idx));
        for status in [
            ExecutionStatus::new_for_test(InnerStatus::Executed(0), 1, &proxy, 9),
            ExecutionStatus::new_for_test(InnerStatus::Executed(0), 0, &proxy, 12),
        ] {
            assert!(!status.requires_execution());
            assert_eq!(status.abort_incarnation_shortcut.load(Ordering::Relaxed), 0);
            assert_eq!(status.inner_status.lock().incarnation(), 0);

            status.incarnate(&mut *status.inner_status.lock(), 1);

            assert_eq!(status.inner_status.lock().incarnation(), 1);
            assert_eq!(status.abort_incarnation_shortcut.load(Ordering::Relaxed), 1);
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DEPENDENCY_DEFER_FLAG
            );
            assert!(status.requires_execution());
            proxy.assert_execution_queue(&vec![]);
        }

        // Finally, should be scheduled:
        // - new incarnation > 1 with idx < 10
        // - new incarnation > 1 with idx > 10
        // - new incarnation = 1 with idx = 10
        let proxy = Arc::new(SchedulerProxy::new_for_test(txn_idx));
        let mut expected_queue = vec![];
        for status in [
            ExecutionStatus::new_for_test(InnerStatus::Executed(5), 0, &proxy, 8),
            ExecutionStatus::new_for_test(InnerStatus::Executed(5), 0, &proxy, 13),
            ExecutionStatus::new_for_test(InnerStatus::Executed(0), 0, &proxy, 10),
        ] {
            // Double-check expected state - provided by new_for_test
            assert!(status.is_executed());
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DEPENDENCY_SAFE_FLAG
            );
            assert!(!status.requires_execution());

            let new_incarnation = status.abort_incarnation_shortcut.load(Ordering::Relaxed) + 1;
            status.incarnate(&mut *status.inner_status.lock(), new_incarnation);

            assert_eq!(status.inner_status.lock().incarnation(), new_incarnation);
            assert_eq!(
                status.abort_incarnation_shortcut.load(Ordering::Relaxed),
                new_incarnation
            );
            assert_eq!(
                status.dependency_shortcut.load(Ordering::Relaxed),
                DEPENDENCY_DEFER_FLAG
            );
            assert!(status.requires_execution());

            expected_queue.push(status.txn_idx);
            proxy.assert_execution_queue(&expected_queue);
        }
    }

    #[should_panic]
    #[test_case(false)]
    #[test_case(true)]
    fn assert_finish_execution_status(case: bool) {
        let status = ExecutionStatus::new(Arc::new(SchedulerProxy::new_for_test(10)), 10);
        // Convert to Executing state
        assert_some_eq!(status.try_start_executing(), 0);
        // Break the invariant: reset only the dependency shortcut flag.
        status.dependency_shortcut.store(
            if case {
                DEPENDENCY_SAFE_FLAG
            } else {
                DEPENDENCY_DEFER_FLAG
            },
            Ordering::Relaxed,
        );

        // Should now panic.
        let _ = status.finish_execution(0);
    }

    #[test]
    fn unstall_err_senarios() {
        let proxy = Arc::new(SchedulerProxy::new_for_test(10));

        for wrong_shortcut in [DEPENDENCY_EXECUTING_FLAG, 100] {
            let status = ExecutionStatus::new_for_test(InnerStatus::Executed(0), 2, &proxy, 10);
            // Unstall succeeds as it should.
            assert_ok_eq!(status.unstall(), false);
            assert_eq!(status.num_stalls.load(Ordering::Relaxed), 1);

            status
                .dependency_shortcut
                .store(wrong_shortcut, Ordering::Relaxed);
            // Normal unstall that would otherwise succeed should now return an error.
            assert_err!(status.unstall());
        }

        let status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(1), 0, &proxy, 10);
        // Number of stalls = 0.
        assert_err!(status.unstall());

        let status =
            ExecutionStatus::new_for_test(InnerStatus::RequiresExecution(0), 1, &proxy, 10);
        // Incarnation 0 may not be unstalled.
        assert_err!(status.unstall());
    }

    #[test_case(false)]
    #[test_case(true)]
    fn resolve_dependency_executing_recheck(case: bool) {
        // Test that after acquiring the waiting queue mutex, resolve dependency call
        // re-checks the dependency shortcut to avoid any lost wake-ups in case the
        // dependencies have been drained in between. To test easily (w.o. failpoints)
        // the main thread simply locks the waiting queue then updates the shortcut.

        let proxy = Arc::new(SchedulerProxy::new_for_test(10));
        let status = ExecutionStatus::new_for_test(InnerStatus::Executing(0), 0, &proxy, 10);
        rayon::scope(|s| {
            // lock waiting queue, Executing status.
            let queue_guard = status.waiting_queue.lock();
            s.spawn(|_| {
                if case {
                    assert_matches!(
                        status.resolve_dependency(DependencyInstruction::Wait),
                        Ok(DependencyResolution::SafeToProceed)
                    )
                } else {
                    assert_matches!(
                        status.resolve_dependency(DependencyInstruction::WaitForExecuting),
                        Ok(DependencyResolution::None)
                    )
                };
            });

            // Update dependency shortcut flag based on test case, using relaxed ordering
            // as waiting mutex should provide the sufficient barriers.
            set_shortcut_to_safe_or_provided(&status, case, DEPENDENCY_DEFER_FLAG);
            drop(queue_guard);
        });
    }

    #[test]
    fn unstall_recheck() {
        // Executed and stalled status.
        let proxy = Arc::new(SchedulerProxy::new_for_test(10));
        let status = ExecutionStatus::new_for_test(InnerStatus::Executed(0), 1, &proxy, 10);

        rayon::scope(|s| {
            // Acquire the lock to stop unstall call.
            let inner_status_guard = status.inner_status.lock();

            s.spawn(|_| {
                // false due to the main thread incrementing stall count below.
                assert_ok_eq!(status.unstall(), false);
            });

            while status.num_stalls.load(Ordering::Relaxed) != 0 {}
            status.num_stalls.fetch_add(1, Ordering::Relaxed);
            drop(inner_status_guard);
        });

        assert_eq!(
            status.dependency_shortcut.load(Ordering::Relaxed),
            DEPENDENCY_DEFER_FLAG
        );
    }
}
