// Copyright © Eiger
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

/// A benchmark for a specific operation.
#[derive(Debug, Clone)]
pub struct Benchmark {
    /// Start time of the operation.
    pub start_time: Instant,
    /// Duration of the operation.
    pub elapsed: Duration,
}

impl Benchmark {
    /// Creates a new benchmark.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            elapsed: Duration::new(0, 0),
        }
    }

    /// Starts the benchmark.
    pub fn start(&mut self) {
        self.start_time = Instant::now();
    }

    /// Stops the benchmark.
    pub fn stop(&mut self) {
        self.elapsed = self.start_time.elapsed();
    }
}

/// A collection of benchmarks for the specification tester.
pub struct Benchmarks {
    /// Benchmark for the spec test.
    pub spec_test: Benchmark,
    /// Benchmark for the mutator.
    pub mutator: Benchmark,
    /// Benchmark for the prover.
    pub prover: Benchmark,
    /// Benchmark for the prover results.
    pub prover_results: Vec<Benchmark>,
}

impl Benchmarks {
    /// Creates a new collection of benchmarks.
    pub fn new() -> Self {
        Self {
            spec_test: Benchmark::new(),
            mutator: Benchmark::new(),
            prover: Benchmark::new(),
            prover_results: Vec::new(),
        }
    }

    /// Displays the benchmarks with the `RUST_LOG` info level.
    pub fn display(&self) {
        info!(
            "Specification testing took {} msecs",
            self.spec_test.elapsed.as_millis()
        );
        info!(
            "Generating mutants took {} msecs",
            self.mutator.elapsed.as_millis()
        );
        info!("Proving took {} msecs", self.prover.elapsed.as_millis());
        if !self.prover_results.is_empty() {
            info!(
                "Min proving time for a mutant: {} msecs",
                self.prover_results
                    .iter()
                    .map(|f| f.elapsed.as_millis())
                    .min()
                    .unwrap()
            );
            info!(
                "Max proving time for a mutant: {} msecs",
                self.prover_results
                    .iter()
                    .map(|f| f.elapsed.as_millis())
                    .max()
                    .unwrap()
            );
            info!(
                "Average proving time for each mutant: {} msecs",
                self.prover_results
                    .iter()
                    .map(|f| f.elapsed.as_millis())
                    .sum::<u128>()
                    / self.prover_results.len() as u128
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn benchmark_records_correct_elapsed_time() {
        let mut benchmark = Benchmark::new();
        benchmark.start();
        thread::sleep(Duration::from_millis(100));
        benchmark.stop();
        assert!(benchmark.elapsed >= Duration::from_millis(100));
    }

    #[test]
    fn benchmark_does_not_record_time_before_start() {
        let mut benchmark = Benchmark::new();
        thread::sleep(Duration::from_millis(100));
        benchmark.start();
        thread::sleep(Duration::from_millis(100));
        benchmark.stop();
        assert!(benchmark.elapsed < Duration::from_millis(200));
    }

    #[test]
    fn benchmark_does_not_record_time_after_stop() {
        let mut benchmark = Benchmark::new();
        benchmark.start();
        thread::sleep(Duration::from_millis(100));
        benchmark.stop();
        thread::sleep(Duration::from_millis(100));
        assert!(benchmark.elapsed < Duration::from_millis(200));
    }

    #[test]
    fn benchmarks_records_multiple_benchmarks() {
        let mut benchmarks = Benchmarks {
            spec_test: Benchmark::new(),
            mutator: Benchmark::new(),
            prover: Benchmark::new(),
            prover_results: Vec::new(),
        };

        benchmarks.spec_test.start();
        thread::sleep(Duration::from_millis(100));
        benchmarks.spec_test.stop();

        benchmarks.mutator.start();
        thread::sleep(Duration::from_millis(100));
        benchmarks.mutator.stop();

        benchmarks.prover.start();
        thread::sleep(Duration::from_millis(100));
        benchmarks.prover.stop();

        assert!(benchmarks.spec_test.elapsed >= Duration::from_millis(100));
        assert!(benchmarks.mutator.elapsed >= Duration::from_millis(100));
        assert!(benchmarks.prover.elapsed >= Duration::from_millis(100));
    }
}
