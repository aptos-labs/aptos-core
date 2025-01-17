// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{execution::execute_workload, state_view::ReadSet, workload::Workload};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::time::Instant;

/// Represents a block for benchmarking: a workload consisting of a block of transactions with the
/// input pre-block state.
pub struct ReplayBlock {
    /// Stores transactions to execute, corresponding to a single block.
    pub(crate) workload: Workload,
    /// Stores all data corresponding to the pre-block state.
    pub(crate) inputs: ReadSet,
}

impl ReplayBlock {
    /// Executes the workload using the specified concurrency level.
    pub(crate) fn run(&self, executor: &AptosVMBlockExecutor, concurrency_level: usize) {
        execute_workload(executor, &self.workload, &self.inputs, concurrency_level);
    }
}

/// Holds configuration for running the benchmarks and measuring the time taken.
pub struct BenchmarkRunner {
    concurrency_levels: Vec<usize>,
    num_repeats: usize,
    measure_overall_instead_of_per_block_time: bool,
    num_blocks_to_skip: usize,
}

impl BenchmarkRunner {
    pub fn new(
        concurrency_levels: Vec<usize>,
        num_repeats: usize,
        measure_overall_instead_of_per_block_time: bool,
        num_blocks_to_skip: usize,
    ) -> Self {
        Self {
            concurrency_levels,
            num_repeats,
            measure_overall_instead_of_per_block_time,
            num_blocks_to_skip,
        }
    }

    /// Runs a sequence of blocks, measuring the execution time.
    pub fn measure_execution_time(&self, blocks: &[ReplayBlock]) {
        for concurrency_level in &self.concurrency_levels {
            if self.measure_overall_instead_of_per_block_time {
                self.measure_overall_execution_time(blocks, *concurrency_level);
            } else {
                self.measure_block_execution_times(blocks, *concurrency_level);
            }
        }
    }

    /// Runs a sequence of blocks, measuring the execution time for each block.
    fn measure_block_execution_times(&self, blocks: &[ReplayBlock], concurrency_level: usize) {
        let mut times = (0..blocks.len())
            .map(|_| Vec::with_capacity(self.num_repeats))
            .collect::<Vec<_>>();

        for i in 0..self.num_repeats {
            let executor = AptosVMBlockExecutor::new();
            for (idx, block) in blocks.iter().enumerate() {
                let start_time = Instant::now();
                block.run(&executor, concurrency_level);
                let time = start_time.elapsed().as_micros();
                if idx >= self.num_blocks_to_skip {
                    println!(
                        "[{}/{}] Block {} execution time is {}us",
                        i + 1,
                        self.num_repeats,
                        idx + 1,
                        time,
                    );
                }
                times[idx].push(time);
            }
        }

        for (idx, mut time) in times.into_iter().enumerate() {
            // Only report measurements for non-skipped blocks.
            if idx >= self.num_blocks_to_skip {
                time.sort();
                let min_time = *time.first().unwrap();
                let average_time = time.iter().sum::<u128>() as f64 / self.num_repeats as f64;
                let median_time = time[self.num_repeats / 2];
                let max_time = *time.last().unwrap();

                println!(
                    "Block {} execution time: min {}us, average {:.2}us, median {}us, max {}us\n",
                    idx + 1,
                    min_time,
                    average_time,
                    median_time,
                    max_time,
                );
            }
        }
    }

    /// Runs the sequence of blocks, measuring the end-to-end execution time.
    fn measure_overall_execution_time(&self, blocks: &[ReplayBlock], concurrency_level: usize) {
        let mut times = Vec::with_capacity(self.num_repeats);
        for i in 0..self.num_repeats {
            let executor = AptosVMBlockExecutor::new();

            // Warm-up.
            for block in &blocks[..self.num_blocks_to_skip] {
                block.run(&executor, concurrency_level);
            }

            // Actual measurement.
            let start_time = Instant::now();
            for block in &blocks[self.num_blocks_to_skip..] {
                block.run(&executor, concurrency_level);
            }
            let time = start_time.elapsed().as_micros();

            println!(
                "[{}/{}] Overall execution time is {}us",
                i + 1,
                self.num_repeats,
                time,
            );
            times.push(time);
        }

        times.sort();
        let min_time = *times.first().unwrap();
        let average_time = times.iter().sum::<u128>() as f64 / self.num_repeats as f64;
        let median_time = times[self.num_repeats / 2];
        let max_time = *times.last().unwrap();

        println!(
            "Overall execution time (blocks {}-{}): min {}us, average {:.2}us, median {}us, max {}us\n",
            self.num_blocks_to_skip + 1, blocks.len(), min_time, average_time, median_time, max_time,
        );
    }
}
