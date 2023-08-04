// Copyright © Aptos Foundation

use crate::{
    sharded_block_executor::{local_executor_shard::LocalExecutorService, test_utils},
    ShardedBlockExecutor,
};
use rand::{rngs::OsRng, Rng};
use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_block_partitioner::v2::PartitionerV2;

#[test]
fn test_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards));
    partitioner.max_rounds = Some(2);
    partitioner.xshard_dep_avoid_threshold = Some(0.9);
    test_utils::test_sharded_block_executor_no_conflict(partitioner, sharded_block_executor);
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_sharded_block_executor_with_conflict_parallel() {
    let num_shards = 7;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards));
    partitioner.max_rounds = Some(8);
    partitioner.xshard_dep_avoid_threshold = Some(0.9);
    test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 4);
}

#[test]
fn test_sharded_block_executor_with_conflict_sequential() {
    let num_shards = 7;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards));
    partitioner.max_rounds = Some(8);
    partitioner.xshard_dep_avoid_threshold = Some(0.9);
    test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 1);
}

#[test]
fn test_sharded_block_executor_with_random_transfers_parallel() {
    let mut rng = OsRng;
    let max_num_shards = 32;
    let num_shards = rng.gen_range(1, max_num_shards);
    let num_shards = 3;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards));
    partitioner.max_rounds = Some(8);
    partitioner.xshard_dep_avoid_threshold = Some(0.9);
    test_utils::sharded_block_executor_with_random_transfers(partitioner, sharded_block_executor, 4)
}

#[test]
fn test_sharded_block_executor_with_random_transfers_sequential() {
    let mut rng = OsRng;
    let max_num_shards = 32;
    let num_shards = rng.gen_range(1, max_num_shards);
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards));
    partitioner.max_rounds = Some(8);
    partitioner.xshard_dep_avoid_threshold = Some(0.9);
    test_utils::sharded_block_executor_with_random_transfers(partitioner, sharded_block_executor, 1)
}




#[test]
fn test_partitioner_v2_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(PartitionerV2::new());
    partitioner.num_rounds_limit = 2;
    partitioner.avoid_pct = 10;
    test_utils::test_sharded_block_executor_no_conflict(partitioner, sharded_block_executor);
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_v2_sharded_block_executor_with_conflict_parallel() {
    let num_shards = 7;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(PartitionerV2::new());
    partitioner.num_rounds_limit = 8;
    partitioner.avoid_pct = 10;
    test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 4);
}

#[test]
fn test_partitioner_v2_sharded_block_executor_with_conflict_sequential() {
    let num_shards = 7;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(PartitionerV2::new());
    partitioner.num_rounds_limit = 8;
    partitioner.avoid_pct = 10;
    test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 1)
}

#[test]
fn test_partitioner_v2_sharded_block_executor_with_random_transfers_parallel() {
    let mut rng = OsRng;
    let max_num_shards = 32;
    let num_shards = rng.gen_range(1, max_num_shards);
    let num_shards = 3;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(PartitionerV2::new());
    partitioner.num_rounds_limit = 8;
    partitioner.avoid_pct = 10;
    test_utils::sharded_block_executor_with_random_transfers(partitioner, sharded_block_executor, 4)
}

#[test]
fn test_partitioner_v2_sharded_block_executor_with_random_transfers_sequential() {
    let mut rng = OsRng;
    let max_num_shards = 32;
    let num_shards = rng.gen_range(1, max_num_shards);
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    let mut partitioner = Box::new(PartitionerV2::new());
    partitioner.num_rounds_limit = 8;
    partitioner.avoid_pct = 10;
    test_utils::sharded_block_executor_with_random_transfers(partitioner, sharded_block_executor, 1)
}
