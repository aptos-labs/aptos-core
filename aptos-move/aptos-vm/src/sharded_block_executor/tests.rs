// Copyright © Aptos Foundation

use crate::{
    sharded_block_executor::{local_executor_shard::LocalExecutorService, test_utils},
    ShardedBlockExecutor,
};
use rand::{rngs::OsRng, Rng};

#[test]
fn test_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    test_utils::test_sharded_block_executor_no_conflict(sharded_block_executor);
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_sharded_block_executor_with_conflict_parallel() {
    let num_shards = 7;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    test_utils::sharded_block_executor_with_conflict(sharded_block_executor, 4);
}

#[test]
fn test_sharded_block_executor_with_conflict_sequential() {
    let num_shards = 7;
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    test_utils::sharded_block_executor_with_conflict(sharded_block_executor, 1)
}

#[test]
fn test_sharded_block_executor_with_random_transfers_parallel() {
    let mut rng = OsRng;
    let max_num_shards = 32;
    let num_shards = rng.gen_range(1, max_num_shards);
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    test_utils::sharded_block_executor_with_random_transfers(sharded_block_executor, 4)
}

#[test]
fn test_sharded_block_executor_with_random_transfers_sequential() {
    let mut rng = OsRng;
    let max_num_shards = 32;
    let num_shards = rng.gen_range(1, max_num_shards);
    let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
    let sharded_block_executor = ShardedBlockExecutor::new(client);
    test_utils::sharded_block_executor_with_random_transfers(sharded_block_executor, 1)
}
