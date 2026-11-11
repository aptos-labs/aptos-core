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
    /// The interpreter polling for an interrupt; counted, not timed.
    Poll,
    /// A storage read through the parallel provider. Part of VM time.
    Read,
    /// Thread CPU time of the VM call, next to its wall time in `VmFirst`
    /// and `VmRetry`.
    VmCpu,
    /// Minor page faults during the VM call; counted, not timed.
    VmMinorFaults,
    /// Voluntary context switches during the VM call; counted, not timed.
    VmVoluntarySwitches,
    /// Involuntary context switches during the VM call; counted, not timed.
    VmInvoluntarySwitches,
}

pub const PHASES: [Phase; 15] = [
    Phase::CommitHooks,
    Phase::NextTask,
    Phase::VmFirst,
    Phase::VmRetry,
    Phase::Apply,
    Phase::PostCommit,
    Phase::ModuleValidation,
    Phase::DependencyWait,
    Phase::SpeculativeFailure,
    Phase::Poll,
    Phase::Read,
    Phase::VmCpu,
    Phase::VmMinorFaults,
    Phase::VmVoluntarySwitches,
    Phase::VmInvoluntarySwitches,
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
            Phase::Poll => "poll",
            Phase::Read => "read",
            Phase::VmCpu => "vm_cpu",
            Phase::VmMinorFaults => "vm_minflt",
            Phase::VmVoluntarySwitches => "vm_nvcsw",
            Phase::VmInvoluntarySwitches => "vm_nivcsw",
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

fn add(phase: Phase, nanos: u64, count: u64) {
    NANOS[phase.index()].0.fetch_add(nanos, Ordering::Relaxed);
    COUNTS[phase.index()].0.fetch_add(count, Ordering::Relaxed);
}

/// The calling thread's CPU time, minor faults and context switches so far.
/// Zero outside Linux.
#[derive(Clone, Copy, Default)]
pub struct ThreadUsage {
    cpu_nanos: u64,
    minor_faults: u64,
    voluntary_switches: u64,
    involuntary_switches: u64,
}

impl ThreadUsage {
    #[cfg(target_os = "linux")]
    pub fn now() -> Self {
        // SAFETY: both calls only write into the zeroed structs passed in.
        unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            libc::getrusage(libc::RUSAGE_THREAD, &mut usage);
            let mut cpu: libc::timespec = std::mem::zeroed();
            libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut cpu);
            Self {
                cpu_nanos: cpu.tv_sec as u64 * 1_000_000_000 + cpu.tv_nsec as u64,
                minor_faults: usage.ru_minflt as u64,
                voluntary_switches: usage.ru_nvcsw as u64,
                involuntary_switches: usage.ru_nivcsw as u64,
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn now() -> Self {
        Self::default()
    }
}

/// Adds the calling thread's usage since `start` to the VM-call counters.
pub fn record_vm_usage(start: ThreadUsage) {
    let end = ThreadUsage::now();
    add(
        Phase::VmCpu,
        end.cpu_nanos.saturating_sub(start.cpu_nanos),
        1,
    );
    add(
        Phase::VmMinorFaults,
        0,
        end.minor_faults.saturating_sub(start.minor_faults),
    );
    add(
        Phase::VmVoluntarySwitches,
        0,
        end.voluntary_switches
            .saturating_sub(start.voluntary_switches),
    );
    add(
        Phase::VmInvoluntarySwitches,
        0,
        end.involuntary_switches
            .saturating_sub(start.involuntary_switches),
    );
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
