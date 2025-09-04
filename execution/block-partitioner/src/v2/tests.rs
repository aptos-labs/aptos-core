// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use crate::{
    BlockPartitioner,
    pre_partition::{
        connected_component::ConnectedComponentPartitioner, uniform_partitioner::UniformPartitioner,
    },
    test_utils::{P2PBlockGenerator, assert_deterministic_result},
    v2::PartitionerV2,
};
use rand::{Rng, thread_rng};
use std::sync::Arc;

#[test]
fn test_partitioner_v2_uniform_correctness() {
    for merge_discarded in [false, true] {
        let block_generator = P2PBlockGenerator::new(100);
        let partitioner = PartitionerV2::new(
            8,
            4,
            0.9,
            64,
            merge_discarded,
            Box::new(UniformPartitioner {}),
        );
        let mut rng = thread_rng();
        for _run_id in 0..20 {
            let block_size = 10_u64.pow(rng.gen_range(0, 4)) as usize;
            let num_shards = rng.gen_range(1, 10);
            let block = block_generator.rand_block(&mut rng, block_size);
            let block_clone = block.clone();
            let partitioned = partitioner.partition(block, num_shards);
            crate::test_utils::verify_partitioner_output(&block_clone, &partitioned);
        }
    }
}

#[test]
fn test_partitioner_v2_uniform_determinism() {
    for merge_discarded in [false, true] {
        let partitioner = Arc::new(PartitionerV2::new(
            4,
            4,
            0.9,
            64,
            merge_discarded,
            Box::new(UniformPartitioner {}),
        ));
        assert_deterministic_result(partitioner);
    }
}

#[test]
fn test_partitioner_v2_connected_component_correctness() {
    for merge_discarded in [false, true] {
        let block_generator = P2PBlockGenerator::new(100);
        let partitioner = PartitionerV2::new(
            8,
            4,
            0.9,
            64,
            merge_discarded,
            Box::new(ConnectedComponentPartitioner {
                load_imbalance_tolerance: 2.0,
            }),
        );
        let mut rng = thread_rng();
        for _run_id in 0..20 {
            let block_size = 10_u64.pow(rng.gen_range(0, 4)) as usize;
            let num_shards = rng.gen_range(1, 10);
            let block = block_generator.rand_block(&mut rng, block_size);
            let block_clone = block.clone();
            let partitioned = partitioner.partition(block, num_shards);
            crate::test_utils::verify_partitioner_output(&block_clone, &partitioned);
        }
    }
}

#[test]
fn test_partitioner_v2_connected_component_determinism() {
    for merge_discarded in [false, true] {
        let partitioner = Arc::new(PartitionerV2::new(
            4,
            4,
            0.9,
            64,
            merge_discarded,
            Box::new(ConnectedComponentPartitioner {
                load_imbalance_tolerance: 2.0,
            }),
        ));
        assert_deterministic_result(partitioner);
    }
}
