// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Wall time BlockSTMv2 workers spend in each phase, summed over workers.
//
// TODO(cleanup): experiment instrumentation, not for merge.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Clone, Copy, Debug)]
pub enum Phase {
    /// Sequential commit hooks, while holding the commit lock.
    CommitHooks,
    /// Asking the scheduler for the next task, including when it has none.
    NextTask,
    /// The VM, on a transaction's first incarnation.
    VmFirst,
    /// The VM, on a re-execution.
    VmRetry,
    /// Applying an execution's outputs to the multi-version map and
    /// notifying the scheduler.
    Apply,
    /// Post-commit materialization of a committed transaction.
    PostCommit,
    ModuleValidation,
    /// A read waiting for the transaction it depends on.
    DependencyWait,
    /// An execution that ended in a speculative failure; counted, not timed.
    SpeculativeFailure,
}

pub const PHASES: [Phase; 9] = [
    Phase::CommitHooks,
    Phase::NextTask,
    Phase::VmFirst,
    Phase::VmRetry,
    Phase::Apply,
    Phase::PostCommit,
    Phase::ModuleValidation,
    Phase::DependencyWait,
    Phase::SpeculativeFailure,
];

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::CommitHooks => "commit_hooks",
            Phase::NextTask => "next_task",
            Phase::VmFirst => "vm_first",
            Phase::VmRetry => "vm_retry",
            Phase::Apply => "apply",
            Phase::PostCommit => "post_commit",
            Phase::ModuleValidation => "module_validation",
            Phase::DependencyWait => "dependency_wait",
            Phase::SpeculativeFailure => "speculative_failure",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// One cache line per counter, so workers updating different phases do not
/// contend.
#[repr(align(128))]
struct Padded(AtomicU64);

const ZERO: Padded = Padded(AtomicU64::new(0));
static NANOS: [Padded; PHASES.len()] = [ZERO; PHASES.len()];
static COUNTS: [Padded; PHASES.len()] = [ZERO; PHASES.len()];

/// Adds the time since `start` to `phase`.
pub fn record(phase: Phase, start: Instant) {
    NANOS[phase.index()]
        .0
        .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    COUNTS[phase.index()].0.fetch_add(1, Ordering::Relaxed);
}

/// Counts one occurrence of `phase` without timing it.
pub fn count(phase: Phase) {
    COUNTS[phase.index()].0.fetch_add(1, Ordering::Relaxed);
}

/// Records the time until it is dropped.
pub struct Timer(Phase, Instant);

pub fn timer(phase: Phase) -> Timer {
    Timer(phase, Instant::now())
}

impl Drop for Timer {
    fn drop(&mut self) {
        record(self.0, self.1);
    }
}

/// Totals so far: for each phase, its name, seconds and count.
pub fn snapshot() -> Vec<(&'static str, f64, u64)> {
    PHASES
        .iter()
        .map(|phase| {
            (
                phase.name(),
                NANOS[phase.index()].0.load(Ordering::Relaxed) as f64 / 1e9,
                COUNTS[phase.index()].0.load(Ordering::Relaxed),
            )
        })
        .collect()
}
