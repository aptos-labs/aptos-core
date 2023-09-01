// Copyright © Aptos Foundation

use crate::pre_partition::{
    connected_component::ConnectedComponentPartitioner, PrePartitioner, PrePartitionerConfig,
};

#[derive(Clone, Debug)]
pub struct ConnectedComponentPartitionerConfig {
    pub load_imbalance_tolerance: f32,
}

impl Default for ConnectedComponentPartitionerConfig {
    fn default() -> Self {
        ConnectedComponentPartitionerConfig {
            load_imbalance_tolerance: 4.0,
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
