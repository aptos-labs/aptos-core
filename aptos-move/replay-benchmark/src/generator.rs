// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{block::Block, overrides::OverrideConfig, workload::Workload};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::transaction::{Transaction, Version};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

pub struct BenchmarkGenerator {
    debugger: AptosDebugger,
    begin_version: Version,
    end_version: Version,
    override_config: OverrideConfig,
}

impl BenchmarkGenerator {
    /// Generates a sequence of [Block] for benchmarking.
    pub async fn generate_blocks(
        debugger: AptosDebugger,
        begin_version: Version,
        end_version: Version,
        override_config: OverrideConfig,
    ) -> anyhow::Result<Vec<Block>> {
        let generator = Arc::new(Self {
            debugger,
            begin_version,
            end_version,
            override_config,
        });

        let limit = generator.end_version - generator.begin_version + 1;
        let (txns, _) = generator
            .debugger
            .get_committed_transactions(generator.begin_version, limit)
            .await?;
        let txn_blocks = generator.partition(txns);

        let num_generated = Arc::new(AtomicU64::new(0));
        let num_blocks = txn_blocks.len();

        let mut tasks = Vec::with_capacity(num_blocks);
        for (begin, txn_block) in txn_blocks {
            let task = tokio::task::spawn_blocking({
                let generator = generator.clone();
                let num_generated = num_generated.clone();
                move || {
                    let start_time = Instant::now();
                    let block = generator.generate_block(begin, txn_block);
                    let time = start_time.elapsed().as_secs();
                    println!(
                        "Generated block {}/{} in {}s",
                        num_generated.fetch_add(1, Ordering::SeqCst) + 1,
                        num_blocks,
                        time
                    );
                    block
                }
            });
            tasks.push(task);
        }

        let mut blocks = Vec::with_capacity(tasks.len());
        for task in tasks {
            blocks.push(task.await?);
        }

        Ok(blocks)
    }

    /// Generates a single [Block] for benchmarking.
    fn generate_block(&self, begin: Version, txns: Vec<Transaction>) -> Block {
        let workload = Workload::new(begin, txns);
        let state_view = self.debugger.state_view_at_version(begin);
        let state_override = self.override_config.get_state_override(&state_view);
        Block::new(workload, &state_view, state_override)
    }

    /// Partitions a sequence of transactions into blocks.
    fn partition(&self, txns: Vec<Transaction>) -> Vec<(Version, Vec<Transaction>)> {
        let mut begin_versions_and_blocks = Vec::with_capacity(txns.len());

        let mut curr_begin = self.begin_version;
        let mut curr_block = Vec::with_capacity(txns.len());

        for txn in txns {
            if txn.is_block_start() && !curr_block.is_empty() {
                let block_size = curr_block.len();
                begin_versions_and_blocks.push((curr_begin, std::mem::take(&mut curr_block)));
                curr_begin += block_size as Version;
            }
            curr_block.push(txn);
        }
        if !curr_block.is_empty() {
            begin_versions_and_blocks.push((curr_begin, curr_block));
        }

        begin_versions_and_blocks
    }
}
