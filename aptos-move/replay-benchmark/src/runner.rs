// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::time::Instant;

/// Holds configuration for running the benchmarks and measuring the time taken.
pub struct BenchmarkRunner {
    concurrency_levels: Vec<usize>,
    num_repeats: usize,
    measure_per_block_instead_of_overall_time: bool,
    num_blocks_to_skip: usize,
}

impl BenchmarkRunner {
    pub fn new(
        concurrency_levels: Vec<usize>,
        num_repeats: usize,
        measure_per_block_instead_of_overall_time: bool,
        num_blocks_to_skip: usize,
    ) -> Self {
        Self {
            concurrency_levels,
            num_repeats,
            measure_per_block_instead_of_overall_time,
            num_blocks_to_skip,
        }
    }

    // TODO:
    //   This measures execution time from a cold-start. Ideally, we want to warm-up with executing
    //   1-2 blocks prior to selected range, but not timing them.
    pub fn measure_execution_time(&self, blocks: &[Block]) {
        for concurrency_level in &self.concurrency_levels {
            if self.measure_per_block_instead_of_overall_time {
                self.measure_block_execution_times(blocks, *concurrency_level);
            } else {
                self.measure_overall_execution_time(blocks, *concurrency_level);
            }
        }
    }

    /// Runs a sequence of blocks, measuring execution time for each block. The median is reported.
    fn measure_block_execution_times(&self, blocks: &[Block], concurrency_level: usize) {
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

    /// Runs the sequence of blocks, measuring end-to-end execution time.
    fn measure_overall_execution_time(&self, blocks: &[Block], concurrency_level: usize) {
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
