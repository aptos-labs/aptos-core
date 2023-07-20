// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_vm::sharded_block_executor::{ShardedBlockExecutor};
use crate::remote_executor_client::RemoteExecutorClient;
use crate::test_utils;

#[test]
fn test_sharded_block_executor_no_conflict() {
    let num_shards = 8;
    let (mut controller, executor_client, _executor_services) = RemoteExecutorClient::create_thread_remote_executor_shards(num_shards, Some(2));
    controller.start();
    let sharded_block_executor = ShardedBlockExecutor::new(executor_client);
    test_utils::test_sharded_block_executor_no_conflict(sharded_block_executor);
}
