// Copyright © Aptos Foundation

use crate::pre_partition::{
    uniform_partitioner::UniformPartitioner, PrePartitioner, PrePartitionerConfig,
};

#[derive(Clone, Debug)]
pub struct UniformPartitionerConfig {}

impl PrePartitionerConfig for UniformPartitionerConfig {
    fn build(&self) -> Box<dyn PrePartitioner> {
        Box::new(UniformPartitioner {})
    }
}
