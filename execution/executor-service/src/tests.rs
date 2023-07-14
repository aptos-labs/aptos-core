// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_vm::sharded_block_executor::{ShardedBlockExecutor};
use crate::remote_executor_shard::RemoteExecutorShard;
use crate::test_utils;
use rand::rngs::OsRng;
use rand::Rng;

#[test]
fn test_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    let (mut controller, executor_shards, _executor_services) = RemoteExecutorShard::create_thread_remote_executor_shards(num_shards, Some(2));
    controller.start();
    let sharded_block_executor = ShardedBlockExecutor::new(executor_shards);
    test_utils::test_sharded_block_executor_no_conflict(sharded_block_executor);
}

#[test]
fn test_sharded_executor_with_conflict() {
    let num_shards = 8;
    let (mut controller, executor_shards, _executor_services) = RemoteExecutorShard::create_thread_remote_executor_shards(num_shards, Some(2));
    controller.start();
    let sharded_block_executor = ShardedBlockExecutor::new(executor_shards);
    test_utils::test_sharded_block_executor_with_conflict(sharded_block_executor, 4);
}


// #[test]
// fn test_sharded_block_executor_with_random_transfers_parallel() {
//     let mut rng = OsRng;
//     let max_num_shards = 32;
//     let num_shards = rng.gen_range(1, max_num_shards);
//     let (mut controller, executor_shards, _executor_services) = RemoteExecutorShard::create_thread_remote_executor_shards(num_shards, Some(4));
//     controller.start();
//     let sharded_block_executor = ShardedBlockExecutor::new(executor_shards);
//     test_utils::sharded_block_executor_with_random_transfers(sharded_block_executor, 4)
// }
//
// #[test]
// fn test_sharded_block_executor_with_random_transfers_sequential() {
//     let mut rng = OsRng;
//     let max_num_shards = 32;
//     let num_shards = rng.gen_range(1, max_num_shards);
//     let (mut controller, executor_shards, _executor_services) = RemoteExecutorShard::create_thread_remote_executor_shards(num_shards, Some(1));
//     controller.start();
//     let sharded_block_executor = ShardedBlockExecutor::new(executor_shards);
//     test_utils::sharded_block_executor_with_random_transfers(sharded_block_executor, 1)
// }
