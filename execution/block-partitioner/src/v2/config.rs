// Copyright © Aptos Foundation

use crate::{v2::PartitionerV2, BlockPartitioner};

#[derive(Clone, Copy, Debug)]
pub struct PartitionerV2Config {
    pub num_threads: usize,
    pub num_rounds_limit: usize,
    pub avoid_pct: u64,
    pub dashmap_num_shards: usize,
    pub partition_last_round: bool,
}

impl PartitionerV2Config {
    pub fn build(self) -> Box<dyn BlockPartitioner> {
        Box::new(PartitionerV2::new(
            self.num_threads,
            self.num_rounds_limit,
            self.avoid_pct,
            self.dashmap_num_shards,
            self.partition_last_round,
        ))
    }

    pub fn num_threads(mut self, val: usize) -> Self {
        self.num_threads = val;
        self
    }

    pub fn num_rounds_limit(mut self, val: usize) -> Self {
        self.num_rounds_limit = val;
        self
    }

    pub fn avoid_pct(mut self, val: u64) -> Self {
        self.avoid_pct = val;
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
            num_rounds_limit: 4,
            avoid_pct: 10,
            dashmap_num_shards: 64,
            partition_last_round: false,
        }
    }
}
