// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::time::Instant;

pub struct BenchmarkRunner {
    concurrency_levels: Vec<usize>,
    num_repeats: usize,
    measure_block_time: bool,
}

impl BenchmarkRunner {
    pub fn new(
        concurrency_levels: Vec<usize>,
        num_repeats: Option<usize>,
        measure_block_time: bool,
    ) -> Self {
        assert!(
            !concurrency_levels.is_empty(),
            "At least one concurrency level must be provided"
        );

        let default_num_repeats = 3;
        let num_repeats = num_repeats.unwrap_or_else(|| {
            println!(
                "[WARN] Using default number of repeats: {}",
                default_num_repeats
            );
            default_num_repeats
        });
        assert!(
            num_repeats >= default_num_repeats,
            "Number of times to repeat the benchmark should be at least the default value {}",
            default_num_repeats
        );

        Self {
            concurrency_levels,
            num_repeats,
            measure_block_time,
        }
    }

    pub fn measure_execution_time(&self, blocks: &[Block]) {
        for concurrency_level in &self.concurrency_levels {
            if self.measure_block_time {
                self.measure_block_execution_time(blocks, *concurrency_level);
            } else {
                self.measure_overall_execution_time(blocks, *concurrency_level);
            }
        }
    }

    fn measure_block_execution_time(&self, blocks: &[Block], concurrency_level: usize) {
        let mut times = Vec::with_capacity(blocks.len());
        for _ in blocks {
            times.push(Vec::with_capacity(self.num_repeats));
        }

        for i in 0..self.num_repeats {
            let executor = AptosVMBlockExecutor::new();
            for (idx, block) in blocks.iter().enumerate() {
                let start_time = Instant::now();
                block.run(&executor, concurrency_level);
                let time = start_time.elapsed().as_millis();

                println!(
                    "[{}/{}] Block {} execution time is {}ms",
                    i + 1,
                    self.num_repeats,
                    idx + 1,
                    time,
                );
                times[idx].push(time);
            }
        }

        for (idx, mut time) in times.into_iter().enumerate() {
            time.sort();
            println!(
                "Block {} median execution time is {}ms\n",
                idx + 1,
                time[self.num_repeats / 2],
            );
        }
    }

    fn measure_overall_execution_time(&self, blocks: &[Block], concurrency_level: usize) {
        let mut times = Vec::with_capacity(self.num_repeats);
        for i in 0..self.num_repeats {
            let start_time = Instant::now();
            let executor = AptosVMBlockExecutor::new();
            for block in blocks {
                block.run(&executor, concurrency_level);
            }
            let time = start_time.elapsed().as_millis();
            println!(
                "[{}/{}] Overall execution time is {}ms",
                i + 1,
                self.num_repeats,
                time,
            );
            times.push(time);
        }
        times.sort();
        println!(
            "Overall median execution time is {}ms\n",
            times[self.num_repeats / 2],
        );
    }
}
