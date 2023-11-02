// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_vm::{
    sharded_block_executor::{local_executor_shard::LocalExecutorClient, ShardedBlockExecutor},
    AptosVM,
};
use once_cell::sync::Lazy;
use std::sync::Arc;

pub static SHARDED_BLOCK_EXECUTOR: Lazy<
    Arc<Mutex<ShardedBlockExecutor<CachedStateView, LocalExecutorClient<CachedStateView>>>>,
> = Lazy::new(|| {
    info!("LOCAL_SHARDED_BLOCK_EXECUTOR created");
    Arc::new(Mutex::new(
        LocalExecutorClient::create_local_sharded_block_executor(AptosVM::get_num_shards(), None),
    ))
});
