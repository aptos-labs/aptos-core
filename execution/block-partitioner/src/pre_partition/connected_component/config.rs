// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pre_partition::{
    PrePartitioner, PrePartitionerConfig, connected_component::ConnectedComponentPartitioner,
};

#[derive(Clone, Debug)]
pub struct ConnectedComponentPartitionerConfig {
    /// If the size a connected component is larger than `load_imbalance_tolerance * block_size / num_shards`,
    /// this component will be broken up into smaller ones.
    ///
    /// See the comments of `aptos_block_partitioner::pre_partition::connected_component::ConnectedComponentPartitioner` for more details.
    pub load_imbalance_tolerance: f32,
}

impl Default for ConnectedComponentPartitionerConfig {
    fn default() -> Self {
        ConnectedComponentPartitionerConfig {
            load_imbalance_tolerance: 2.0,
        }
    }
}

impl PrePartitionerConfig for ConnectedComponentPartitionerConfig {
    fn build(&self) -> Box<dyn PrePartitioner> {
        Box::new(ConnectedComponentPartitioner {
            load_imbalance_tolerance: self.load_imbalance_tolerance,
        })
    }
}
