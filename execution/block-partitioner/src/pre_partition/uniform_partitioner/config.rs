// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pre_partition::{
    PrePartitioner, PrePartitionerConfig, uniform_partitioner::UniformPartitioner,
};

#[derive(Clone, Debug)]
pub struct UniformPartitionerConfig {}

impl PrePartitionerConfig for UniformPartitionerConfig {
    fn build(&self) -> Box<dyn PrePartitioner> {
        Box::new(UniformPartitioner {})
    }
}
