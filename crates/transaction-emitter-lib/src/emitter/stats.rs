// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt,
    ops::Sub,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Debug, Default)]
pub struct TxnStats {
    pub submitted: u64,
    pub committed: u64,
    pub expired: u64,
    pub latency: u64,
    pub latency_buckets: AtomicHistogramSnapshot,
}

#[derive(Debug, Default)]
pub struct TxnStatsRate {
    pub submitted: u64,
    pub committed: u64,
    pub expired: u64,
    pub latency: u64,
    pub p99_latency: u64,
}

impl fmt::Display for TxnStatsRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "submitted: {} txn/s, committed: {} txn/s, expired: {} txn/s, latency: {} ms, p99 latency: {} ms",
            self.submitted, self.committed, self.expired, self.latency, self.p99_latency,
        )
    }
}

impl TxnStats {
    pub fn rate(&self, window: Duration) -> TxnStatsRate {
        TxnStatsRate {
            submitted: self.submitted / window.as_secs(),
            committed: self.committed / window.as_secs(),
            expired: self.expired / window.as_secs(),
            latency: if self.committed == 0 {
                0u64
            } else {
                self.latency / self.committed
            },
            p99_latency: self.latency_buckets.percentile(99, 100),
        }
    }
}

impl fmt::Display for TxnStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "submitted: {}, committed: {}, expired: {}",
            self.submitted, self.committed, self.expired,
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
            latency: self.latency - other.latency,
            latency_buckets: &self.latency_buckets - &other.latency_buckets,
        }
    }
}

#[derive(Debug, Default)]
pub struct StatsAccumulator {
    pub submitted: AtomicU64,
    pub committed: AtomicU64,
    pub expired: AtomicU64,
    pub latency: AtomicU64,
    pub latencies: Arc<AtomicHistogramAccumulator>,
}

impl StatsAccumulator {
    pub fn accumulate(&self) -> TxnStats {
        TxnStats {
            submitted: self.submitted.load(Ordering::Relaxed),
            committed: self.committed.load(Ordering::Relaxed),
            expired: self.expired.load(Ordering::Relaxed),
            latency: self.latency.load(Ordering::Relaxed),
            latency_buckets: self.latencies.snapshot(),
        }
    }
}

const DEFAULT_HISTOGRAM_CAPACITY: usize = 1024;
const DEFAULT_HISTOGRAM_STEP_WIDTH: u64 = 50;

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

    fn get_bucket_num(&self, data_value: u64) -> u64 {
        let bucket_num = data_value / self.step_width;
        if bucket_num >= self.capacity as u64 - 2 {
            return self.capacity as u64 - 1;
        }
        bucket_num
    }

    pub fn record_data_point(&self, data_value: u64, data_num: u64) {
        let bucket_num = self.get_bucket_num(data_value);
        self.buckets[bucket_num as usize].fetch_add(data_num as u64, Ordering::Relaxed);
    }
}

#[derive(Debug)]
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

#[cfg(test)]
mod test {
    use crate::emitter::stats::{
        AtomicHistogramAccumulator, AtomicHistogramSnapshot, TxnStats, DEFAULT_HISTOGRAM_CAPACITY,
        DEFAULT_HISTOGRAM_STEP_WIDTH,
    };

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
        assert_eq!(histogram.get_bucket_num(49), 0);
        assert_eq!(histogram.get_bucket_num(50), 1);
        assert_eq!(histogram.get_bucket_num(51), 1);
        assert_eq!(histogram.get_bucket_num(200_000), 1023);
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
            latency: 0,
            latency_buckets: histogram.snapshot(),
        };
        let res = stat.latency_buckets.percentile(9, 10);
        assert_eq!(res, 900);
    }
}
