// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt,
    ops::{Add, Sub},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub struct TxnStats {
    pub submitted: u64,
    pub committed: u64,
    pub expired: u64,
    pub failed_submission: u64,
    pub latency: u64,         // total milliseconds across all latency measurements
    pub latency_samples: u64, // number of events with latency measured
    pub latency_buckets: AtomicHistogramSnapshot, // millisecond snapshot buckets
    pub lasted: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct TxnStatsRate {
    pub submitted: f64,         // per second
    pub committed: f64,         // per second
    pub expired: f64,           // per second
    pub failed_submission: f64, // per second
    pub latency: f64,           // mean latency (milliseconds)
    pub latency_samples: u64,   // number latency-measured events
    pub p50_latency: u64,       // milliseconds, 50% this or better
    pub p70_latency: u64,       // milliseconds, 70% this or better
    pub p90_latency: u64,       // milliseconds, 90% this or better
    pub p99_latency: u64,       // milliseconds, 99% this or better
}

impl fmt::Display for TxnStatsRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "committed: {:.2} txn/s{}{}{}, latency: {:.2} ms, (p50: {} ms, p70: {}, p90: {} ms, p99: {} ms), latency samples: {}",
            self.committed,
            if self.submitted != self.committed { format!(", submitted: {:.2} txn/s", self.submitted) } else { "".to_string()},
            if self.failed_submission != 0.0 { format!(", failed submission: {:.2} txn/s", self.failed_submission) } else { "".to_string()},
            if self.expired != 0.0 { format!(", expired: {:.2} txn/s", self.expired) } else { "".to_string()},
            self.latency, self.p50_latency, self.p70_latency, self.p90_latency, self.p99_latency, self.latency_samples,
        )
    }
}

impl TxnStats {
    pub fn rate(&self) -> TxnStatsRate {
        let window_secs = self.lasted.as_secs_f64();
        TxnStatsRate {
            submitted: (self.submitted as f64) / window_secs,
            committed: (self.committed as f64) / window_secs,
            expired: (self.expired as f64) / window_secs,
            failed_submission: (self.failed_submission as f64) / window_secs,
            latency: if self.latency_samples == 0 {
                0.0
            } else {
                (self.latency as f64) / (self.latency_samples as f64)
            },
            latency_samples: self.latency_samples,
            p50_latency: self.latency_buckets.percentile(50, 100),
            p70_latency: self.latency_buckets.percentile(70, 100),
            p90_latency: self.latency_buckets.percentile(90, 100),
            p99_latency: self.latency_buckets.percentile(99, 100),
        }
    }
}

impl fmt::Display for TxnStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "submitted: {}, committed: {}, expired: {}, failed submission: {}",
            self.submitted, self.committed, self.expired, self.failed_submission,
        )
    }
}

impl Sub for &TxnStats {
    type Output = TxnStats;

    fn sub(self, other: &TxnStats) -> TxnStats {
        TxnStats {
            submitted: self.submitted - other.submitted,
            committed: self.committed - other.committed,
            expired: self.expired - other.expired,
            failed_submission: self.failed_submission - other.failed_submission,
            latency: self.latency - other.latency,
            latency_samples: self.latency_samples - other.latency_samples,
            latency_buckets: &self.latency_buckets - &other.latency_buckets,
            lasted: self.lasted - other.lasted,
        }
    }
}

impl Add for &TxnStats {
    type Output = TxnStats;

    fn add(self, other: &TxnStats) -> TxnStats {
        TxnStats {
            submitted: self.submitted + other.submitted,
            committed: self.committed + other.committed,
            expired: self.expired + other.expired,
            failed_submission: self.failed_submission + other.failed_submission,
            latency: self.latency + other.latency,
            latency_samples: self.latency_samples + other.latency_samples,
            latency_buckets: &self.latency_buckets + &other.latency_buckets,
            lasted: self.lasted + other.lasted,
        }
    }
}

#[derive(Debug, Default)]
pub struct StatsAccumulator {
    pub submitted: AtomicU64,
    pub committed: AtomicU64,
    pub expired: AtomicU64,
    pub failed_submission: AtomicU64,
    pub latency: AtomicU64, // total milliseconds across all latency measurements
    pub latency_samples: AtomicU64, // number of events with latency measured
    pub latencies: Arc<AtomicHistogramAccumulator>, // millisecond histogram buckets
}

impl StatsAccumulator {
    pub fn accumulate(&self, lasted: Duration) -> TxnStats {
        TxnStats {
            submitted: self.submitted.load(Ordering::Relaxed),
            committed: self.committed.load(Ordering::Relaxed),
            expired: self.expired.load(Ordering::Relaxed),
            failed_submission: self.failed_submission.load(Ordering::Relaxed),
            latency: self.latency.load(Ordering::Relaxed),
            latency_samples: self.latency_samples.load(Ordering::Relaxed),
            latency_buckets: self.latencies.snapshot(),
            lasted,
        }
    }
}

// have more slots than generally used txn expiration. (240s)
const DEFAULT_HISTOGRAM_CAPACITY: usize = 2400;
// we don't have better precision than ~300 ms anyways.
const DEFAULT_HISTOGRAM_STEP_WIDTH: u64 = 100;

#[derive(Debug)]
pub struct AtomicHistogramAccumulator {
    capacity: usize,
    step_width: u64,
    buckets: Vec<AtomicU64>,
}

impl Default for AtomicHistogramAccumulator {
    fn default() -> AtomicHistogramAccumulator {
        AtomicHistogramAccumulator::new(DEFAULT_HISTOGRAM_CAPACITY, DEFAULT_HISTOGRAM_STEP_WIDTH)
    }
}

impl AtomicHistogramAccumulator {
    pub fn new(size: usize, step: u64) -> AtomicHistogramAccumulator {
        let mut buf = Vec::with_capacity(size);
        for _i in 0..size {
            buf.push(AtomicU64::new(0));
        }
        Self {
            capacity: size,
            step_width: step,
            buckets: buf,
        }
    }

    pub fn snapshot(&self) -> AtomicHistogramSnapshot {
        let mut buf = Vec::with_capacity(self.capacity);
        for i in 0..self.capacity {
            buf.push(self.buckets[i].load(Ordering::Relaxed));
        }
        AtomicHistogramSnapshot {
            capacity: self.capacity,
            step_width: self.step_width,
            buckets: buf,
        }
    }

    fn get_bucket_num(&self, data_value: u64) -> usize {
        let bucket_num = data_value / self.step_width;
        if bucket_num >= self.capacity as u64 - 2 {
            return self.capacity - 1;
        }
        bucket_num as usize
    }

    pub fn record_data_point(&self, data_value: u64, data_num: u64) {
        let bucket_num = self.get_bucket_num(data_value);
        self.buckets[bucket_num].fetch_add(data_num, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct AtomicHistogramSnapshot {
    capacity: usize,
    step_width: u64,
    buckets: Vec<u64>,
}

impl Default for AtomicHistogramSnapshot {
    fn default() -> AtomicHistogramSnapshot {
        AtomicHistogramAccumulator::default().snapshot()
    }
}

impl Sub for &AtomicHistogramSnapshot {
    type Output = AtomicHistogramSnapshot;

    fn sub(self, other: &AtomicHistogramSnapshot) -> AtomicHistogramSnapshot {
        assert_eq!(
            self.buckets.len(),
            other.buckets.len(),
            "Histogram snapshots must have same size, prev: {}, cur: {}",
            self.buckets.len(),
            other.buckets.len()
        );
        let mut buf = Vec::with_capacity(self.capacity);
        for i in 0..self.buckets.len() {
            buf.push(self.buckets[i] - other.buckets[i]);
        }
        AtomicHistogramSnapshot {
            capacity: self.capacity,
            step_width: self.step_width,
            buckets: buf,
        }
    }
}

impl Add for &AtomicHistogramSnapshot {
    type Output = AtomicHistogramSnapshot;

    fn add(self, other: &AtomicHistogramSnapshot) -> AtomicHistogramSnapshot {
        assert_eq!(
            self.buckets.len(),
            other.buckets.len(),
            "Histogram snapshots must have same size, prev: {}, cur: {}",
            self.buckets.len(),
            other.buckets.len()
        );
        let mut buf = Vec::with_capacity(self.capacity);
        for i in 0..self.buckets.len() {
            buf.push(self.buckets[i] + other.buckets[i]);
        }
        AtomicHistogramSnapshot {
            capacity: self.capacity,
            step_width: self.step_width,
            buckets: buf,
        }
    }
}

impl AtomicHistogramSnapshot {
    pub fn percentile(&self, numerator: u64, denominator: u64) -> u64 {
        let committed: u64 = self.buckets.iter().sum();
        let p_count = committed * numerator / denominator;
        let mut counter = 0u64;
        for i in 0..self.buckets.len() {
            counter += self.buckets[i];
            if counter >= p_count {
                return i as u64 * self.step_width;
            }
        }
        unreachable!()
    }
}

#[derive(Debug)]
pub struct DynamicStatsTracking {
    num_phases: usize,
    cur_phase: Arc<AtomicUsize>,
    stats: Vec<StatsAccumulator>,
}

impl DynamicStatsTracking {
    pub fn new(num_phases: usize) -> DynamicStatsTracking {
        assert!(num_phases >= 1);
        Self {
            num_phases,
            cur_phase: Arc::new(AtomicUsize::new(0)),
            stats: (0..num_phases)
                .map(|_| StatsAccumulator::default())
                .collect(),
        }
    }

    pub fn start_next_phase(&self) -> usize {
        let cur_phase = self.cur_phase.fetch_add(1, Ordering::Relaxed) + 1;
        assert!(cur_phase < self.num_phases);
        cur_phase
    }

    pub fn get_cur(&self) -> &StatsAccumulator {
        self.stats.get(self.get_cur_phase()).unwrap()
    }

    pub fn get_cur_phase(&self) -> usize {
        self.cur_phase.load(Ordering::Relaxed)
    }

    pub fn get_cur_phase_obj(&self) -> Arc<AtomicUsize> {
        self.cur_phase.clone()
    }

    pub fn accumulate(&self, phase_starts: &[Instant]) -> Vec<TxnStats> {
        let now = Instant::now();
        self.stats
            .iter()
            .take(self.get_cur_phase() + 1)
            .enumerate()
            .map(|(i, s)| {
                s.accumulate(
                    (if i >= self.get_cur_phase() {
                        now
                    } else {
                        phase_starts[i + 1]
                    })
                    .duration_since(phase_starts[i]),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::emitter::stats::{
        AtomicHistogramAccumulator, AtomicHistogramSnapshot, TxnStats, DEFAULT_HISTOGRAM_CAPACITY,
        DEFAULT_HISTOGRAM_STEP_WIDTH,
    };
    use std::time::Duration;

    #[test]
    pub fn test_default_atomic_histogram() {
        let histogram = AtomicHistogramAccumulator::default();
        assert_eq!(histogram.step_width, DEFAULT_HISTOGRAM_STEP_WIDTH);
        assert_eq!(histogram.buckets.len(), DEFAULT_HISTOGRAM_CAPACITY);
    }

    #[test]
    pub fn test_get_bucket_num() {
        let histogram = AtomicHistogramAccumulator::default();
        assert_eq!(histogram.get_bucket_num(0), 0);
        assert_eq!(
            histogram.get_bucket_num(DEFAULT_HISTOGRAM_STEP_WIDTH - 1),
            0
        );
        assert_eq!(histogram.get_bucket_num(DEFAULT_HISTOGRAM_STEP_WIDTH), 1);
        assert_eq!(
            histogram.get_bucket_num(DEFAULT_HISTOGRAM_STEP_WIDTH + 1),
            1
        );
        assert_eq!(
            histogram.get_bucket_num(500_000),
            DEFAULT_HISTOGRAM_CAPACITY - 1
        );
    }

    #[test]
    pub fn test_sub() {
        let mut cur_snap = AtomicHistogramSnapshot::default();
        let mut cur_vec = Vec::new();
        for i in 10..20 {
            cur_vec.push(i);
        }
        cur_snap.buckets = cur_vec;

        let mut pre_snap = AtomicHistogramSnapshot::default();
        let mut prev_vec = Vec::new();
        for i in 0..10 {
            prev_vec.push(i);
        }
        pre_snap.buckets = prev_vec;
        let res = &cur_snap - &pre_snap;
        for &i in res.buckets.iter() {
            assert_eq!(i, 10);
        }
    }

    #[test]
    pub fn test_percentile_latency() {
        let histogram = AtomicHistogramAccumulator::default();
        // set 10 commits, with latencies as:
        // 100ms, 200ms, 300ms ... 900ms, 1000ms
        // for p90 count is 9
        // expected p90 = 900
        for i in 1..11 {
            histogram.record_data_point(i as u64 * 100, 1);
        }
        let stat = TxnStats {
            submitted: 0,
            committed: 10,
            expired: 0,
            failed_submission: 0,
            latency: 0,
            latency_samples: 0,
            latency_buckets: histogram.snapshot(),
            lasted: Duration::from_secs(10),
        };
        let res = stat.latency_buckets.percentile(9, 10);
        assert_eq!(res, 900);
    }
}
