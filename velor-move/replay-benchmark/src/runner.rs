// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{execution::execute_workload, state_view::ReadSet, workload::Workload};
use velor_vm::{velor_vm::VelorVMBlockExecutor, VMBlockExecutor};
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
    pub(crate) fn run(&self, executor: &VelorVMBlockExecutor, concurrency_level: usize) {
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

        for _ in 0..self.num_repeats {
            let executor = VelorVMBlockExecutor::new();
            for (idx, block) in blocks.iter().enumerate() {
                let start_time = Instant::now();
                block.run(&executor, concurrency_level);
                let time = start_time.elapsed().as_micros();
                times[idx].push(time);
            }
        }

        println!("concurrency level, block, median (us), mean (us), min (us), max (us)",);
        for (idx, mut time) in times.into_iter().enumerate() {
            // Only report measurements for non-skipped blocks.
            if idx >= self.num_blocks_to_skip {
                time.sort();
                let min_time = *time.first().unwrap();
                let average_time = time.iter().sum::<u128>() as f64 / self.num_repeats as f64;
                let median_time = time[self.num_repeats / 2];
                let max_time = *time.last().unwrap();

                println!(
                    "{concurrency_level}, {idx}, {median_time}, {average_time:.2}, {min_time}, {max_time}",
                );
            }
        }
    }

    /// Runs the sequence of blocks, measuring the end-to-end execution time.
    fn measure_overall_execution_time(&self, blocks: &[ReplayBlock], concurrency_level: usize) {
        let mut times = Vec::with_capacity(self.num_repeats);
        for _ in 0..self.num_repeats {
            let executor = VelorVMBlockExecutor::new();

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
            times.push(time);
        }

        times.sort();
        let min_time = *times.first().unwrap();
        let average_time = times.iter().sum::<u128>() as f64 / self.num_repeats as f64;
        let median_time = times[self.num_repeats / 2];
        let max_time = *times.last().unwrap();

        println!("concurrency level, median (us), mean (us), min (us), max (us)",);
        println!("{concurrency_level}, {median_time}, {average_time:.2}, {min_time}, {max_time}",);
    }
}
