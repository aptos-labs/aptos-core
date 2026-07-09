// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Timing harness: warm up, then take multiple samples of a measured region, and summarize with a
//! central tendency (median) and spread.

use std::time::Duration;

/// How many times to warm up (discarded) and sample (recorded).
#[derive(Clone, Copy)]
pub struct TimingConfig {
    pub warmup: usize,
    pub samples: usize,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            warmup: 50,
            samples: 200,
        }
    }
}

/// The recorded per-run durations of a measured region.
pub struct Samples {
    /// Sorted sample durations, in nanoseconds.
    nanos: Vec<u128>,
}

impl Samples {
    pub fn len(&self) -> usize {
        self.nanos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nanos.is_empty()
    }

    /// Median duration (central tendency). Robust to the occasional slow outlier.
    pub fn median(&self) -> Duration {
        Duration::from_nanos(self.percentile(50.0) as u64)
    }

    pub fn min(&self) -> Duration {
        Duration::from_nanos(*self.nanos.first().unwrap_or(&0) as u64)
    }

    pub fn max(&self) -> Duration {
        Duration::from_nanos(*self.nanos.last().unwrap_or(&0) as u64)
    }

    pub fn mean(&self) -> Duration {
        if self.nanos.is_empty() {
            return Duration::ZERO;
        }
        let sum: u128 = self.nanos.iter().sum();
        Duration::from_nanos((sum / self.nanos.len() as u128) as u64)
    }

    /// Sample standard deviation, as a measure of spread.
    pub fn stddev(&self) -> Duration {
        if self.nanos.len() < 2 {
            return Duration::ZERO;
        }
        let mean = self.mean().as_nanos() as f64;
        let var = self
            .nanos
            .iter()
            .map(|&n| {
                let d = n as f64 - mean;
                d * d
            })
            .sum::<f64>()
            / (self.nanos.len() - 1) as f64;
        Duration::from_nanos(var.sqrt() as u64)
    }

    fn percentile(&self, p: f64) -> u128 {
        if self.nanos.is_empty() {
            return 0;
        }
        let rank = (p / 100.0 * (self.nanos.len() - 1) as f64).round() as usize;
        self.nanos[rank.min(self.nanos.len() - 1)]
    }
}

/// Warms up `config.warmup` times (discarded), then records `config.samples` samples.
///
/// `run_once` must perform any per-run state reset *itself* (untimed) and return only the duration
/// of the measured region, so setup is never charged to the measurement.
pub fn collect_samples(config: &TimingConfig, mut run_once: impl FnMut() -> Duration) -> Samples {
    for _ in 0..config.warmup {
        let _ = run_once();
    }
    let mut nanos = Vec::with_capacity(config.samples);
    for _ in 0..config.samples {
        nanos.push(run_once().as_nanos());
    }
    nanos.sort_unstable();
    Samples { nanos }
}
