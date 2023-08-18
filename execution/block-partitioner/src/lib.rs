// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_types::block_executor::partitioner::RoundId;

pub mod sharded_block_partitioner;
pub mod test_utils;

pub struct BlockPartitionerConfig {
    num_shards: usize,
    max_partitioning_rounds: RoundId,
    cross_shard_dep_avoid_threshold: f32,
    partition_last_round: bool,
}

impl BlockPartitionerConfig {
    pub fn new() -> Self {
        BlockPartitionerConfig {
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

    pub fn build(self) -> ShardedBlockPartitioner {
        ShardedBlockPartitioner::new(
            self.num_shards,
            self.max_partitioning_rounds,
            self.cross_shard_dep_avoid_threshold,
            self.partition_last_round,
        )
    }
}

impl Default for BlockPartitionerConfig {
    fn default() -> Self {
        Self::new()
    }
}
