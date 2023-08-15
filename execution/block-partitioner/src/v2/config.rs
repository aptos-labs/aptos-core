// Copyright © Aptos Foundation

use crate::{v2::PartitionerV2, BlockPartitioner};

#[derive(Clone, Copy, Debug)]
pub struct PartitionerV2Config {
    pub num_threads: usize,
    pub max_partitioning_rounds: usize,
    pub cross_shard_dep_avoid_threshold: f32,
    pub dashmap_num_shards: usize,
    pub partition_last_round: bool,
}

impl PartitionerV2Config {
    pub fn build(self) -> Box<dyn BlockPartitioner> {
        Box::new(PartitionerV2::new(
            self.num_threads,
            self.max_partitioning_rounds,
            self.cross_shard_dep_avoid_threshold,
            self.dashmap_num_shards,
            self.partition_last_round,
        ))
    }

    pub fn num_threads(mut self, val: usize) -> Self {
        self.num_threads = val;
        self
    }

    pub fn max_partitioning_rounds(mut self, val: usize) -> Self {
        self.max_partitioning_rounds = val;
        self
    }

    pub fn cross_shard_dep_avoid_threshold(mut self, val: f32) -> Self {
        self.cross_shard_dep_avoid_threshold = val;
        self
    }

    pub fn dashmap_num_shards(mut self, val: usize) -> Self {
        self.dashmap_num_shards = val;
        self
    }

    pub fn partition_last_round(mut self, val: bool) -> Self {
        self.partition_last_round = val;
        self
    }
}

impl Default for PartitionerV2Config {
    fn default() -> Self {
        Self {
            num_threads: 8,
            max_partitioning_rounds: 4,
            cross_shard_dep_avoid_threshold: 0.9,
            dashmap_num_shards: 64,
            partition_last_round: false,
        }
    }
}
