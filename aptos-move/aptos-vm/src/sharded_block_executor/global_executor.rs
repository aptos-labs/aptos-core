// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{
    local_executor_shard::GlobalCrossShardClient, sharded_executor_service::ShardedExecutorService,
};
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

    pub fn execute_global_txns(
        &self,
        transactions: Vec<TransactionWithDependencies<AnalyzedTransaction>>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        trace!("executing the last round in global executor",);
        if transactions.is_empty() {
            return Ok(vec![]);
        }
        ShardedExecutorService::execute_transactions_with_dependencies(
            None,
            self.executor_thread_pool.clone(),
            transactions,
            self.global_cross_shard_client.clone(),
            None,
            GLOBAL_ROUND_ID,
            state_view,
            concurrency_level,
            maybe_block_gas_limit,
        )
    }
}
