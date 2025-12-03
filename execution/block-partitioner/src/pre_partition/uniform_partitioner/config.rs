// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
