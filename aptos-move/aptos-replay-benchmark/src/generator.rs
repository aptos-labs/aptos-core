// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{block::Block, overrides::OverrideConfig, workload::Workload};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::transaction::{Transaction, Version};

pub struct BenchmarkGenerator {
    debugger: AptosDebugger,
    begin_version: Version,
    end_version: Version,
    override_config: OverrideConfig,
}

impl BenchmarkGenerator {
    pub fn new(
        debugger: AptosDebugger,
        begin_version: Version,
        end_version: Version,
        override_config: OverrideConfig,
    ) -> Self {
        assert!(
            begin_version <= end_version,
            "Transaction versions are not a valid closed interval: [{}, {}].",
            begin_version,
            end_version,
        );

        Self {
            debugger,
            begin_version,
            end_version,
            override_config,
        }
    }

    /// Generates a sequence of [Block] for benchmarking.
    pub async fn generate_blocks(&self) -> anyhow::Result<Vec<Block>> {
        let limit = self.end_version - self.begin_version + 1;
        let (txns, _) = self
            .debugger
            .get_committed_transactions(self.begin_version, limit)
            .await?;
        let txn_blocks = self.partition(txns);

        let mut blocks = vec![];
        for (begin, txn_block) in txn_blocks {
            blocks.push(self.generate_block(begin, txn_block)?);
        }
        Ok(blocks)
    }

    /// Generates a single [Block] for benchmarking.
    fn generate_block(&self, begin: Version, txns: Vec<Transaction>) -> anyhow::Result<Block> {
        let workload = Workload::new(begin, txns);

        let state_view = self.debugger.state_view_at_version(begin);
        let state_override = self.override_config.get_state_override(&state_view);

        let state_view = self.debugger.state_view_at_version(begin);
        Block::new(workload, &state_view, state_override, 32)
    }

    /// Partitions a sequence of transactions into blocks.
    fn partition(&self, txns: Vec<Transaction>) -> Vec<(Version, Vec<Transaction>)> {
        let mut begin_versions_and_blocks = Vec::with_capacity(txns.len());

        let mut curr_begin = self.begin_version;
        let mut curr_block = Vec::with_capacity(txns.len());

        for txn in txns {
            if txn.is_block_start() && !curr_block.is_empty() {
                let block = std::mem::take(&mut curr_block);
                let block_size = block.len();
                begin_versions_and_blocks.push((curr_begin, block));
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
