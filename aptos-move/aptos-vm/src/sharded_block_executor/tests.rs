// Copyright © Aptos Foundation

use crate::{
    sharded_block_executor::{local_executor_shard::LocalExecutorService, test_utils},
    ShardedBlockExecutor,
};
use aptos_block_partitioner::{
    pre_partition::{
        connected_component::config::ConnectedComponentPartitionerConfig,
        uniform_partitioner::config::UniformPartitionerConfig,
    },
    v2::config::PartitionerV2Config,
    PartitionerConfig,
};
use rand::{rngs::OsRng, Rng};

#[test]
fn test_partitioner_v2_uniform_sharded_block_executor_no_conflict() {
    for merge_discard in [false, true] {
        let num_shards = 8;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::test_sharded_block_executor_no_conflict(partitioner, sharded_block_executor);
    }
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_v2_uniform_sharded_block_executor_with_conflict_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 4);
    }
}

#[test]
fn test_partitioner_v2_uniform_sharded_block_executor_with_conflict_sequential() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 1)
    }
}

#[test]
fn test_partitioner_v2_uniform_sharded_block_executor_with_random_transfers_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 3;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(!merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            4,
        )
    }
}

#[test]
fn test_partitioner_v2_uniform_sharded_block_executor_with_random_transfers_sequential() {
    for merge_discard in [false, true] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            1,
        )
    }
}

#[test]
fn test_partitioner_v2_connected_component_sharded_block_executor_no_conflict() {
    for merge_discard in [false, true] {
        let num_shards = 8;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::test_sharded_block_executor_no_conflict(partitioner, sharded_block_executor);
    }
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_v2_connected_component_sharded_block_executor_with_conflict_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 4);
    }
}

#[test]
fn test_partitioner_v2_connected_component_sharded_block_executor_with_conflict_sequential() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 1)
    }
}

#[test]
fn test_partitioner_v2_connected_component_sharded_block_executor_with_random_transfers_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 3;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(!merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            4,
        )
    }
}

#[test]
fn test_partitioner_v2_connected_component_sharded_block_executor_with_random_transfers_sequential()
{
    for merge_discard in [false, true] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            1,
        )
    }
}
