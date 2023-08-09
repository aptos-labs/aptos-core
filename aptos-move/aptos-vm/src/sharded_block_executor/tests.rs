// Copyright © Aptos Foundation

use crate::{
    sharded_block_executor::{
        local_executor_shard::{LocalExecutorClient, LocalExecutorService},
        test_utils,
    },
    ShardedBlockExecutor,
};
use aptos_state_view::StateView;
use aptos_block_partitioner::{
    sharded_block_partitioner::ShardedBlockPartitioner, v2::PartitionerV2,
};
use rand::{rngs::OsRng, Rng};

fn setup_sharded_block_executor<S: StateView + Sync + Send + 'static>(
    num_shards: usize,
    num_threads_per_shard: Option<usize>,
) -> ShardedBlockExecutor<S, LocalExecutorClient<S>> {
    let client =
        LocalExecutorService::setup_local_executor_shards(num_shards, num_threads_per_shard);
    ShardedBlockExecutor::new(client)
}

#[test]
fn test_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    for last_round_partition in [true, false] {
        let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards, 2, 0.9, last_round_partition));
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(4));
        test_utils::test_sharded_block_executor_no_conflict(
            partitioner,
            sharded_block_executor,
        );
    }
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_sharded_block_executor_with_conflict_parallel() {
    let num_shards = 7;
    for last_round_partition in [true, false] {
        let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards, 8, 0.9, last_round_partition));
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(4));
        test_utils::sharded_block_executor_with_conflict(
            partitioner,
            sharded_block_executor,
            4,
        );
    }
}

#[test]
fn test_sharded_block_executor_with_conflict_sequential() {
    let num_shards = 7;
    for last_round_partition in [true, false] {
        let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards, 8, 0.9, last_round_partition));
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(1));
        test_utils::sharded_block_executor_with_conflict(
            partitioner,
            sharded_block_executor,
            1,
        )
    }
}

#[test]
fn test_sharded_block_executor_with_random_transfers_parallel() {
    for last_round_partition in [true, false] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(4));
        let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards, 8, 0.9, last_round_partition));
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            4,
        );
    }
}

#[test]
fn test_sharded_block_executor_with_random_transfers_sequential() {
    for last_round_partition in [true, false] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(1));
        let mut partitioner = Box::new(ShardedBlockPartitioner::new(num_shards, 8, 0.9, last_round_partition));
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            1,
        )
    }
}

#[test]
fn test_partitioner_v2_sharded_block_executor_no_conflict() {
    for merge_discard in [false, true] {
        let num_shards = 8;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = Box::new(PartitionerV2::new(2, 4, 10, 64, merge_discard));
        test_utils::test_sharded_block_executor_no_conflict(partitioner, sharded_block_executor);
    }
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_v2_sharded_block_executor_with_conflict_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = Box::new(PartitionerV2::new(8, 4, 10, 64, merge_discard));
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 4);
    }
}

#[test]
fn test_partitioner_v2_sharded_block_executor_with_conflict_sequential() {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = Box::new(PartitionerV2::new(8, 4, 10, 64, merge_discard));
        test_utils::sharded_block_executor_with_conflict(partitioner, sharded_block_executor, 1)
    }
}

#[test]
fn test_partitioner_v2_sharded_block_executor_with_random_transfers_parallel() {
    for merge_discard in [false, true] {
        let num_shards = 3;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = Box::new(PartitionerV2::new(8, 4, 10, 64, merge_discard));
        test_utils::sharded_block_executor_with_random_transfers(partitioner, sharded_block_executor, 4)
    }
}

#[test]
fn test_partitioner_v2_sharded_block_executor_with_random_transfers_sequential() {
    for merge_discard in [false, true] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = Box::new(PartitionerV2::new(8, 4, 10, 64, merge_discard));
        test_utils::sharded_block_executor_with_random_transfers(partitioner, sharded_block_executor, 1)
    }
}
