// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! What the process consumed over a measured interval, beyond wall-clock time:
//! CPU by thread group, the block executor's own worker accounting, and memory
//! and scheduling counters from `getrusage`. Wall-clock throughput alone cannot
//! tell more workers burning more CPU from workers sitting idle.

use aptos_block_executor::{counters as block_executor_counters, worker_phases::WorkerPhase};
use aptos_logger::info;
use std::collections::BTreeMap;

/// Phases a Block-STM worker charges its time to, with the clock each is read
/// on. `wall` is elapsed time, `cpu` is thread CPU time.
pub(crate) const WORKER_CLOCKS: [&str; 2] = ["wall", "cpu"];

#[derive(Debug, Clone, Default)]
pub(crate) struct ResourceUsage {
    /// `CLOCK_MONOTONIC` seconds when the snapshot was taken, so a profile
    /// recorded on the same clock can be windowed to the measured interval.
    /// `elapsed_delta` keeps the start, and `monotonic_end_secs` the end.
    monotonic_start_secs: f64,
    monotonic_end_secs: f64,
    /// Block executor wall seconds inside `execute_transactions_parallel*`.
    parallel_execution_secs: f64,
    /// Block executor wall seconds inside the worker scope, so excluding the
    /// per-block setup and finalization around it.
    worker_scope_secs: f64,
    /// Seconds workers spent parked on a dependency.
    dependency_wait_secs: f64,
    /// Worker microseconds by (phase, clock).
    worker_phase_micros: BTreeMap<(&'static str, &'static str), u64>,
    /// Process user and system CPU seconds.
    user_cpu_secs: f64,
    system_cpu_secs: f64,
    /// Peak resident set size, in KiB. Not a delta.
    max_rss_kib: u64,
    minor_faults: u64,
    major_faults: u64,
    voluntary_ctx_switches: u64,
    involuntary_ctx_switches: u64,
    /// CPU seconds by thread name with its trailing index stripped, so all
    /// `par_exec-N` workers land in one row. Linux only.
    thread_group_cpu_secs: BTreeMap<String, f64>,
}

impl ResourceUsage {
    pub fn now() -> Self {
        let worker_phase_micros = WorkerPhase::ALL
            .iter()
            .flat_map(|phase| {
                WORKER_CLOCKS.iter().map(move |clock| {
                    (
                        (phase.label(), *clock),
                        block_executor_counters::WORKER_PHASE_MICROS
                            .with_label_values(&[phase.label(), clock])
                            .get(),
                    )
                })
            })
            .collect();

        let mut usage = Self {
            monotonic_start_secs: monotonic_secs(),
            monotonic_end_secs: 0.0,
            parallel_execution_secs: block_executor_counters::PARALLEL_EXECUTION_SECONDS
                .get_sample_sum(),
            worker_scope_secs: block_executor_counters::RAYON_EXECUTION_SECONDS.get_sample_sum(),
            dependency_wait_secs: block_executor_counters::DEPENDENCY_WAIT_SECONDS.get_sample_sum(),
            worker_phase_micros,
            thread_group_cpu_secs: thread_group_cpu_secs(),
            ..Default::default()
        };
        usage.read_rusage();
        usage
    }

    #[cfg(unix)]
    fn read_rusage(&mut self) {
        // SAFETY: a zeroed rusage is a valid out-parameter for getrusage.
        let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: `ru` is a valid, writable rusage for the duration of the call.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) } != 0 {
            return;
        }
        let secs = |tv: libc::timeval| tv.tv_sec as f64 + tv.tv_usec as f64 * 1e-6;
        self.user_cpu_secs = secs(ru.ru_utime);
        self.system_cpu_secs = secs(ru.ru_stime);
        // Linux reports KiB, macOS bytes.
        self.max_rss_kib = if cfg!(target_os = "macos") {
            ru.ru_maxrss as u64 / 1024
        } else {
            ru.ru_maxrss as u64
        };
        self.minor_faults = ru.ru_minflt as u64;
        self.major_faults = ru.ru_majflt as u64;
        self.voluntary_ctx_switches = ru.ru_nvcsw as u64;
        self.involuntary_ctx_switches = ru.ru_nivcsw as u64;
    }

    #[cfg(not(unix))]
    fn read_rusage(&mut self) {}

    pub fn elapsed_delta(self) -> Self {
        let end = Self::now();
        Self {
            monotonic_start_secs: self.monotonic_start_secs,
            monotonic_end_secs: end.monotonic_start_secs,
            parallel_execution_secs: end.parallel_execution_secs - self.parallel_execution_secs,
            worker_scope_secs: end.worker_scope_secs - self.worker_scope_secs,
            dependency_wait_secs: end.dependency_wait_secs - self.dependency_wait_secs,
            worker_phase_micros: end
                .worker_phase_micros
                .iter()
                .map(|(k, v)| {
                    (
                        *k,
                        v - self.worker_phase_micros.get(k).copied().unwrap_or(0),
                    )
                })
                .collect(),
            user_cpu_secs: end.user_cpu_secs - self.user_cpu_secs,
            system_cpu_secs: end.system_cpu_secs - self.system_cpu_secs,
            max_rss_kib: end.max_rss_kib,
            minor_faults: end.minor_faults - self.minor_faults,
            major_faults: end.major_faults - self.major_faults,
            voluntary_ctx_switches: end.voluntary_ctx_switches - self.voluntary_ctx_switches,
            involuntary_ctx_switches: end.involuntary_ctx_switches - self.involuntary_ctx_switches,
            thread_group_cpu_secs: end
                .thread_group_cpu_secs
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v - self.thread_group_cpu_secs.get(k).copied().unwrap_or(0.0),
                    )
                })
                .filter(|(_, v)| *v > 0.0)
                .collect(),
        }
    }

    /// Logs everything per transaction, under `prefix`, in a fixed format the
    /// perf harness parses.
    pub fn print(&self, prefix: &str, num_txns: f64) {
        let per_txn_us = |secs: f64| secs * 1e6 / num_txns;
        info!(
            "{} resources: monotonic window secs: start={:.6} end={:.6}",
            prefix, self.monotonic_start_secs, self.monotonic_end_secs
        );
        info!(
            "{} resources: block executor wall us/txn: parallel_execution={:.2} worker_scope={:.2} dependency_wait={:.2}",
            prefix,
            per_txn_us(self.parallel_execution_secs),
            per_txn_us(self.worker_scope_secs),
            per_txn_us(self.dependency_wait_secs),
        );
        for clock in WORKER_CLOCKS {
            let phases = WorkerPhase::ALL
                .iter()
                .map(|phase| {
                    let micros = self
                        .worker_phase_micros
                        .get(&(phase.label(), clock))
                        .copied()
                        .unwrap_or(0);
                    format!("{}={:.2}", phase.label(), micros as f64 / num_txns)
                })
                .collect::<Vec<_>>()
                .join(" ");
            info!("{} resources: worker {} us/txn: {}", prefix, clock, phases);
        }
        info!(
            "{} resources: process cpu us/txn: user={:.2} system={:.2}",
            prefix,
            per_txn_us(self.user_cpu_secs),
            per_txn_us(self.system_cpu_secs),
        );
        info!(
            "{} resources: process counters per txn: minor_faults={:.3} major_faults={:.3} voluntary_ctx_switches={:.3} involuntary_ctx_switches={:.3}",
            prefix,
            self.minor_faults as f64 / num_txns,
            self.major_faults as f64 / num_txns,
            self.voluntary_ctx_switches as f64 / num_txns,
            self.involuntary_ctx_switches as f64 / num_txns,
        );
        info!("{} resources: max rss: {} KiB", prefix, self.max_rss_kib);
        for (group, secs) in &self.thread_group_cpu_secs {
            info!(
                "{} resources: thread group cpu us/txn: {}={:.2}",
                prefix,
                group,
                per_txn_us(*secs)
            );
        }
    }
}

#[cfg(unix)]
fn monotonic_secs() -> f64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec for the duration of the call.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9
}

#[cfg(not(unix))]
fn monotonic_secs() -> f64 {
    0.0
}

/// CPU seconds of every live thread, summed by thread name with the trailing
/// `-N` or `N` index removed. Empty where `/proc` is unavailable.
#[cfg(target_os = "linux")]
fn thread_group_cpu_secs() -> BTreeMap<String, f64> {
    // SAFETY: sysconf has no preconditions.
    let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let mut groups = BTreeMap::new();
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        return groups;
    };
    for task in tasks.flatten() {
        let Ok(stat) = std::fs::read_to_string(task.path().join("stat")) else {
            continue;
        };
        // `comm` is parenthesized and may contain spaces; fields after it are
        // fixed: state is field 3, utime and stime are fields 14 and 15.
        let Some(close) = stat.rfind(')') else {
            continue;
        };
        let Some(open) = stat.find('(') else {
            continue;
        };
        let comm = &stat[open + 1..close];
        let fields = stat[close + 2..].split_whitespace().collect::<Vec<_>>();
        let (Some(utime), Some(stime)) = (
            fields.get(11).and_then(|f| f.parse::<u64>().ok()),
            fields.get(12).and_then(|f| f.parse::<u64>().ok()),
        ) else {
            continue;
        };
        let group = comm
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches('-')
            .to_string();
        *groups.entry(group).or_insert(0.0) += (utime + stime) as f64 / ticks_per_sec;
    }
    groups
}

#[cfg(not(target_os = "linux"))]
fn thread_group_cpu_secs() -> BTreeMap<String, f64> {
    BTreeMap::new()
}
