// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_infallible::Mutex;
use velor_logger::info;
use velor_storage_interface::state_store::state_view::cached_state_view::CachedStateView;
use velor_vm::{
    sharded_block_executor::{local_executor_shard::LocalExecutorClient, ShardedBlockExecutor},
    VelorVM,
};
use once_cell::sync::Lazy;
use std::sync::Arc;

pub static SHARDED_BLOCK_EXECUTOR: Lazy<
    Arc<Mutex<ShardedBlockExecutor<CachedStateView, LocalExecutorClient<CachedStateView>>>>,
> = Lazy::new(|| {
    info!("LOCAL_SHARDED_BLOCK_EXECUTOR created");
    Arc::new(Mutex::new(
        LocalExecutorClient::create_local_sharded_block_executor(VelorVM::get_num_shards(), None),
    ))
});
