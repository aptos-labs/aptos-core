// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Where a Block-STM worker's time goes, by phase, in both wall-clock and
//! thread CPU time. The two differ by how long the worker was descheduled:
//! parked on a dependency, blocked on a mutex, or preempted.
//!
//! Each worker accumulates into a thread-local and flushes into the global
//! counters once per block, so the hot path pays two clock reads per phase
//! switch and no shared writes.

use crate::counters::WORKER_PHASE_MICROS;
use std::{cell::RefCell, time::Instant};

#[derive(Clone, Copy)]
pub enum WorkerPhase {
    /// The sequential commit path: validating delayed fields, block limits and
    /// hot state for commit-ready transactions.
    CommitHooks,
    /// Asking the scheduler for a task, including spinning or parking when
    /// there is none.
    Scheduler,
    /// Inside the VM: `SingleTransactionExecutor::execute`.
    ExecuteVm,
    /// Inside the VM, for an incarnation that was aborted by the time it
    /// returned, so the work is redone.
    ExecuteVmWasted,
    /// After the VM returns: applying the write set to the multi-version map,
    /// recording the read set, and notifying the scheduler.
    ExecuteApply,
    /// Converting a committed output to its storage form.
    Materialize,
    /// Re-validating module reads for a cold validation requirement.
    ModuleValidation,
}

impl WorkerPhase {
    pub const ALL: [WorkerPhase; 7] = [
        WorkerPhase::CommitHooks,
        WorkerPhase::Scheduler,
        WorkerPhase::ExecuteVm,
        WorkerPhase::ExecuteVmWasted,
        WorkerPhase::ExecuteApply,
        WorkerPhase::Materialize,
        WorkerPhase::ModuleValidation,
    ];

    fn index(self) -> usize {
        match self {
            WorkerPhase::CommitHooks => 0,
            WorkerPhase::Scheduler => 1,
            WorkerPhase::ExecuteVm => 2,
            WorkerPhase::ExecuteVmWasted => 3,
            WorkerPhase::ExecuteApply => 4,
            WorkerPhase::Materialize => 5,
            WorkerPhase::ModuleValidation => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WorkerPhase::CommitHooks => "commit_hooks",
            WorkerPhase::Scheduler => "scheduler",
            WorkerPhase::ExecuteVm => "execute_vm",
            WorkerPhase::ExecuteVmWasted => "execute_vm_wasted",
            WorkerPhase::ExecuteApply => "execute_apply",
            WorkerPhase::Materialize => "materialize",
            WorkerPhase::ModuleValidation => "module_validation",
        }
    }
}

struct Accumulator {
    wall_micros: [u64; WorkerPhase::ALL.len()],
    cpu_micros: [u64; WorkerPhase::ALL.len()],
    phase: WorkerPhase,
    last_wall: Instant,
    last_cpu: f64,
}

thread_local! {
    static ACCUMULATOR: RefCell<Option<Accumulator>> = const { RefCell::new(None) };
}

#[cfg(unix)]
fn thread_cpu_seconds() -> f64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec for the duration of the call.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9
}

#[cfg(not(unix))]
fn thread_cpu_seconds() -> f64 {
    0.0
}

/// Tracks the calling worker's phases until dropped, then flushes the totals
/// into [`WORKER_PHASE_MICROS`].
pub(crate) struct WorkerPhaseTracker;

impl WorkerPhaseTracker {
    pub(crate) fn start(phase: WorkerPhase) -> Self {
        ACCUMULATOR.with(|acc| {
            *acc.borrow_mut() = Some(Accumulator {
                wall_micros: [0; WorkerPhase::ALL.len()],
                cpu_micros: [0; WorkerPhase::ALL.len()],
                phase,
                last_wall: Instant::now(),
                last_cpu: thread_cpu_seconds(),
            });
        });
        Self
    }

    /// Charges the time since the last switch to the current phase and makes
    /// `phase` current. A no-op on a thread that is not a tracked worker.
    pub(crate) fn switch(phase: WorkerPhase) {
        ACCUMULATOR.with(|acc| {
            if let Some(acc) = acc.borrow_mut().as_mut() {
                acc.charge();
                acc.phase = phase;
            }
        });
    }

    /// Like [`Self::switch`], but charges the elapsed interval to `charge_to`
    /// instead of the current phase. The VM phase uses it once it knows
    /// whether the incarnation's work survives.
    pub(crate) fn switch_charging(charge_to: WorkerPhase, phase: WorkerPhase) {
        ACCUMULATOR.with(|acc| {
            if let Some(acc) = acc.borrow_mut().as_mut() {
                acc.phase = charge_to;
                acc.charge();
                acc.phase = phase;
            }
        });
    }
}

impl Accumulator {
    fn charge(&mut self) {
        let now_wall = Instant::now();
        let now_cpu = thread_cpu_seconds();
        let i = self.phase.index();
        self.wall_micros[i] += now_wall.duration_since(self.last_wall).as_micros() as u64;
        self.cpu_micros[i] += ((now_cpu - self.last_cpu) * 1e6).max(0.0) as u64;
        self.last_wall = now_wall;
        self.last_cpu = now_cpu;
    }
}

impl Drop for WorkerPhaseTracker {
    fn drop(&mut self) {
        ACCUMULATOR.with(|acc| {
            if let Some(mut acc) = acc.borrow_mut().take() {
                acc.charge();
                for phase in WorkerPhase::ALL {
                    let i = phase.index();
                    WORKER_PHASE_MICROS
                        .with_label_values(&[phase.label(), "wall"])
                        .inc_by(acc.wall_micros[i]);
                    WORKER_PHASE_MICROS
                        .with_label_values(&[phase.label(), "cpu"])
                        .inc_by(acc.cpu_micros[i]);
                }
            }
        });
    }
}
