// Copyright © Aptos Foundation

use crate::{
    sharded_block_executor::{local_executor_shard::LocalExecutorService, test_utils},
    ShardedBlockExecutor,
};
use aptos_block_partitioner::v2::config::PartitionerV2Config;
use rand::{rngs::OsRng, Rng};

#[test]
fn test_partitioner_sharded_block_executor_no_conflict() {
    for merge_discard in [false, true] {
        let num_shards = 8;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .num_threads(2)
            .max_partitioning_rounds(4)
            .cross_shard_dep_avoid_threshold(0.9)
            .dashmap_num_shards(64)
            .partition_last_round(merge_discard)
            .build();
        test_utils::test_sharded_block_executor_no_conflict(partitioner, sharded_block_executor);
    }
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_sharded_block_executor_with_conflict_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .num_threads(8)
            .max_partitioning_rounds(4)
            .cross_shard_dep_avoid_threshold(0.9)
            .dashmap_num_shards(64)
            .partition_last_round(merge_discard)
            .build();
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 4);
    }
}

#[test]
fn test_partitioner_sharded_block_executor_with_conflict_sequential() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .num_threads(8)
            .max_partitioning_rounds(4)
            .cross_shard_dep_avoid_threshold(0.9)
            .dashmap_num_shards(64)
            .partition_last_round(merge_discard)
            .build();
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 1)
    }
}

#[test]
fn test_partitioner_sharded_block_executor_with_random_transfers_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 3;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .num_threads(8)
            .max_partitioning_rounds(4)
            .cross_shard_dep_avoid_threshold(0.9)
            .dashmap_num_shards(64)
            .partition_last_round(merge_discard)
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            4,
        )
    }
}

#[test]
fn test_partitioner_sharded_block_executor_with_random_transfers_sequential() {
    for merge_discard in [false, true] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .num_threads(8)
            .max_partitioning_rounds(4)
            .cross_shard_dep_avoid_threshold(0.9)
            .dashmap_num_shards(64)
            .partition_last_round(merge_discard)
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            1,
        )
    }
}
