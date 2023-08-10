// Copyright © Aptos Foundation

use crate::v2::PartitionerV2;

#[derive(Clone, Copy, Debug)]
pub struct PartitionerV2Config {
    pub num_threads: usize,
    pub num_rounds_limit: usize,
    pub avoid_pct: u64,
    pub dashmap_num_shards: usize,
    pub merge_discarded: bool,
}

impl PartitionerV2Config {
    pub fn build(self) -> PartitionerV2 {
        PartitionerV2::new(
            self.num_threads,
            self.num_rounds_limit,
            self.avoid_pct,
            self.dashmap_num_shards,
            self.merge_discarded,
        )
    }
}

impl Default for PartitionerV2Config {
    fn default() -> Self {
        Self {
            num_threads: 8,
            num_rounds_limit: 4,
            avoid_pct: 10,
            dashmap_num_shards: 64,
            merge_discarded: true,
        }
    }
}
