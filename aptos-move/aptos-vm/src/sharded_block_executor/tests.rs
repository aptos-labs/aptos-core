// Copyright © Aptos Foundation

use crate::{
    sharded_block_executor::{
        local_executor_shard::{LocalExecutorClient, LocalExecutorService},
        test_utils,
    },
    ShardedBlockExecutor,
};
use aptos_state_view::StateView;
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
    for last_round_partition in [true, false].iter() {
        let sharded_block_executor = setup_sharded_block_executor(8, Some(4));
        test_utils::test_sharded_block_executor_no_conflict(
            sharded_block_executor,
            *last_round_partition,
        );
    }
}

#[test]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_sharded_block_executor_with_conflict_parallel() {
    for last_round_partition in [true, false].iter() {
        let sharded_block_executor = setup_sharded_block_executor(7, Some(4));
        test_utils::sharded_block_executor_with_conflict(
            sharded_block_executor,
            4,
            *last_round_partition,
        );
    }
}

#[test]
fn test_sharded_block_executor_with_conflict_sequential() {
    for last_round_partition in [true, false].iter() {
        let sharded_block_executor = setup_sharded_block_executor(7, Some(1));
        test_utils::sharded_block_executor_with_conflict(
            sharded_block_executor,
            1,
            *last_round_partition,
        )
    }
}

#[test]
fn test_sharded_block_executor_with_random_transfers_parallel() {
    for last_round_partition in [true, false].iter() {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(4));
        test_utils::sharded_block_executor_with_random_transfers(
            sharded_block_executor,
            4,
            *last_round_partition,
        );
    }
}

#[test]
fn test_sharded_block_executor_with_random_transfers_sequential() {
    for last_round_partition in [true, false].iter() {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let sharded_block_executor = setup_sharded_block_executor(num_shards, Some(1));
        test_utils::sharded_block_executor_with_random_transfers(
            sharded_block_executor,
            1,
            *last_round_partition,
        )
    }
}
