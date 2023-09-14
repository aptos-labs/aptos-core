// Copyright © Aptos Foundation

use std::fmt::{Display, Formatter};
use std::ops::AddAssign;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

/// Statistics about a single run of `FastPathBlockExecutor`.
#[derive(Default, Debug)]
pub struct ExecutorStats {
    pub total_txn_count: usize,
    pub fallback_txn_count: usize,
    pub time_stats: ExecutorTimeStats,
    pub fast_path_stats: FastPathStats,
}

/// Statistics about the executor timings.
#[derive(Default, Debug)]
pub struct ExecutorTimeStats {
    pub total: Duration,
    pub init: Duration,
    pub fast_path: Duration,
    pub fallback: Duration,
    pub wait: Duration,
    pub final_output_reconstruction: Duration,
}

/// Statistics about the fast path.
#[derive(Default, Debug)]
pub struct FastPathStats {
    pub worker_threads: u32,
    // TODO: add vm_init_time
    pub total_batch_processing_time: Duration,
    pub batch_init_time: Duration,
    pub execution_time: Duration,
    pub validation_time: Duration,
    pub materialization_task_spawn_time: Duration,
    pub discard_reasons: DiscardReasonStats,
}

/// Statistics about the reasons for discarding a transaction.
#[derive(Default, Debug)]
pub struct DiscardReasonStats {
    pub read_write_conflict: u32,
    pub read_delta_conflict: u32,
    pub sequence_number: u32,
    pub vm_abort: u32,
}

/// Statistics for a single worker thread.
///
/// Due to the way [`ThreadLocal`] works, the worker only holds an immutable reference to
/// `WorkerStats` and the type must be `Send`, so the fields have to be atomic.
/// However, in practice, this object is only accessed by one thread at a time.
#[derive(Default)]
pub struct WorkerStats {
    pub discard_reasons: AtomicDiscardReasonStats,
}

/// Statistics about the reasons for discarding a transaction.
///
/// Due to the way [`ThreadLocal`] works, the worker only holds an immutable reference to
/// `WorkerStats` and the type must be `Send`, so the fields have to be atomic.
/// However, in practice, this object is only accessed by one thread at a time.
#[derive(Default, Debug)]
pub struct AtomicDiscardReasonStats {
    pub read_write_conflict: AtomicU32,
    pub read_delta_conflict: AtomicU32,
    pub sequence_number: AtomicU32,
    pub vm_abort: AtomicU32,
}

impl DiscardReasonStats {
    pub fn total(&self) -> u32 {
        self.read_write_conflict + self.read_delta_conflict + self.sequence_number + self.vm_abort
    }
}

impl AtomicDiscardReasonStats {
    pub fn add_read_write_conflict(&self) {
        add_assign_u32(&self.read_write_conflict, 1);
    }

    pub fn add_read_delta_conflict(&self) {
        add_assign_u32(&self.read_delta_conflict, 1);
    }

    pub fn add_sequence_number(&self) {
        add_assign_u32(&self.sequence_number, 1);
    }

    pub fn add_vm_abort(&self) {
        add_assign_u32(&self.vm_abort, 1);
    }
}

impl<'a> AddAssign<&'a AtomicDiscardReasonStats> for DiscardReasonStats {
    fn add_assign(&mut self, rhs: &'a AtomicDiscardReasonStats) {
        self.read_write_conflict += rhs.read_write_conflict.load(Ordering::Relaxed);
        self.read_delta_conflict += rhs.read_delta_conflict.load(Ordering::Relaxed);
        self.sequence_number += rhs.sequence_number.load(Ordering::Relaxed);
        self.vm_abort += rhs.vm_abort.load(Ordering::Relaxed);
    }
}

impl<'a> AddAssign<&'a WorkerStats> for FastPathStats {
    // Mutable reference tells to the compiler that there are no concurrent accesses to `rhs`.
    fn add_assign(&mut self, rhs: &'a WorkerStats) {
        self.worker_threads += 1;
        self.discard_reasons += &rhs.discard_reasons;
    }
}

impl Display for ExecutorStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ExecutorStats")?;
        writeln!(f, "    Total txn count:    {}", self.total_txn_count)?;
        writeln!(f, "    Fallback txn count: {}", self.fallback_txn_count)?;
        writeln!(f, "    Time stats:")?;
        writeln!(f, "        Total:         {:?}", self.time_stats.total)?;
        writeln!(f, "        Init:          {:?}", self.time_stats.init)?;
        writeln!(f, "        Fast path:     {:?}", self.time_stats.fast_path)?;
        writeln!(f, "        Fallback:      {:?}", self.time_stats.fallback)?;
        writeln!(f, "        Wait:          {:?}", self.time_stats.wait)?;
        writeln!(f, "        Final output:  {:?}", self.time_stats.final_output_reconstruction)?;
        writeln!(f, "    Fast path stats:")?;
        writeln!(f, "        Worker threads: {}", self.fast_path_stats.worker_threads)?;
        writeln!(f, "        Time stats:")?;
        writeln!(f, "            Total batch processing:     {:?}", self.fast_path_stats.total_batch_processing_time)?;
        writeln!(f, "            Batch init:                 {:?}", self.fast_path_stats.batch_init_time)?;
        writeln!(f, "            Execution:                  {:?}", self.fast_path_stats.execution_time)?;
        writeln!(f, "            Validation:                 {:?}", self.fast_path_stats.validation_time)?;
        writeln!(f, "            Materialization task spawn: {:?}", self.fast_path_stats.materialization_task_spawn_time)?;
        writeln!(f, "        Discard reasons:")?;
        writeln!(f, "            Read/write conflict: {}", self.fast_path_stats.discard_reasons.read_write_conflict)?;
        writeln!(f, "            Read/delta conflict: {}", self.fast_path_stats.discard_reasons.read_delta_conflict)?;
        writeln!(f, "            Sequence number:     {}", self.fast_path_stats.discard_reasons.sequence_number)?;
        writeln!(f, "            VM abort:            {}", self.fast_path_stats.discard_reasons.vm_abort)?;
        writeln!(f, "            Last transaction:    1")?;

        Ok(())
    }
}

/// Non-atomic `+=` for `AtomicU64`.
fn add_assign_u64(atomic: &AtomicU64, value: u64) {
    let prev = atomic.load(Ordering::Relaxed);
    atomic.store(prev + value, Ordering::Relaxed);
}

/// Non-atomic `+=` for `AtomicU32`.
fn add_assign_u32(atomic: &AtomicU32, value: u32) {
    let prev = atomic.load(Ordering::Relaxed);
    atomic.store(prev + value, Ordering::Relaxed);
}
