// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{local_executor_shard::GlobalCrossShardClient, sharded_executor_service::ShardedExecutorService, TxnProviderArgs};
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{TransactionWithDependencies, GLOBAL_ROUND_ID},
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
};
use move_core_types::vm_status::VMStatus;
use std::sync::Arc;

pub struct GlobalExecutor<S: StateView + Sync + Send + 'static> {
    global_cross_shard_client: Arc<GlobalCrossShardClient>,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    phantom: std::marker::PhantomData<S>,
}

impl<S: StateView + Sync + Send + 'static> GlobalExecutor<S> {
    pub fn new(cross_shard_client: Arc<GlobalCrossShardClient>, num_threads: usize) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                // We need two extra threads for the cross-shard commit receiver and the thread
                // that is blocked on waiting for execute block to finish.
                .num_threads(num_threads + 2)
                .build()
                .unwrap(),
        );
        Self {
            global_cross_shard_client: cross_shard_client,
            executor_thread_pool,
            phantom: std::marker::PhantomData,
        }
    }
}
