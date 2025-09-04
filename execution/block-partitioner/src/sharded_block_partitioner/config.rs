// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sharded_block_partitioner::ShardedBlockPartitioner, BlockPartitioner, PartitionerConfig,
};
use velor_types::block_executor::partitioner::RoundId;
use clap::Parser;

/// The configuration for `velor_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner`,
/// which is also referred to as `V1` in executor-benchmark after `velor_block_partitioner::v2` is added.
#[derive(Clone, Copy, Debug, Parser)]
pub struct PartitionerV1Config {
    pub num_shards: usize,
    pub max_partitioning_rounds: RoundId,
    pub cross_shard_dep_avoid_threshold: f32,
    pub partition_last_round: bool,
}

impl PartitionerV1Config {
    pub fn new() -> Self {
        PartitionerV1Config {
            num_shards: 0,
            max_partitioning_rounds: 3,
            cross_shard_dep_avoid_threshold: 0.9,
            partition_last_round: false,
        }
    }

    pub fn num_shards(mut self, num_shards: usize) -> Self {
        self.num_shards = num_shards;
        self
    }

    pub fn max_partitioning_rounds(mut self, max_partitioning_rounds: RoundId) -> Self {
        self.max_partitioning_rounds = max_partitioning_rounds;
        self
    }

    pub fn cross_shard_dep_avoid_threshold(mut self, threshold: f32) -> Self {
        self.cross_shard_dep_avoid_threshold = threshold;
        self
    }

    pub fn partition_last_round(mut self, partition_last_round: bool) -> Self {
        self.partition_last_round = partition_last_round;
        self
    }
}

impl Default for PartitionerV1Config {
    fn default() -> Self {
        Self::new()
    }
}

impl PartitionerConfig for PartitionerV1Config {
    fn build(&self) -> Box<dyn BlockPartitioner> {
        Box::new(ShardedBlockPartitioner::new(
            self.num_shards,
            self.max_partitioning_rounds,
            self.cross_shard_dep_avoid_threshold,
            self.partition_last_round,
        ))
    }
}
