// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_executor::AptosVMBlockExecutorWrapper,
    sharded_block_executor::{
        aggr_overridden_state_view::{AggregatorOverriddenStateView, TOTAL_SUPPLY_AGGR_BASE_VAL},
        coordinator_client::CoordinatorClient,
        counters::{
            SHARDED_BLOCK_EXECUTION_BY_ROUNDS_SECONDS, SHARDED_BLOCK_EXECUTOR_TXN_COUNT,
            SHARDED_EXECUTOR_SERVICE_SECONDS,
        },
        cross_shard_client::{CrossShardClient, CrossShardCommitReceiver, CrossShardCommitSender},
        cross_shard_state_view::CrossShardStateView,
        messages::CrossShardMsg,
        ExecutorShardCommand,
    },
};
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager, txn_provider::default::DefaultTxnProvider,
};
use aptos_logger::{info, trace};
use aptos_types::{
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorLocalConfig},
        partitioner::{ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::StateView,
    transaction::{
        analyzed_transaction::AnalyzedTransaction,
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput,
        TransactionOutput,
    },
};
use aptos_vm_logging::disable_speculative_logging;
use futures::{channel::oneshot, executor::block_on};
use move_core_types::vm_status::VMStatus;
use std::sync::Arc;

pub struct ShardedExecutorService<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    num_shards: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    coordinator_client: Arc<dyn CoordinatorClient<S>>,
    cross_shard_client: Arc<dyn CrossShardClient>,
}

impl<S: StateView + Sync + Send + 'static> ShardedExecutorService<S> {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        coordinator_client: Arc<dyn CoordinatorClient<S>>,
        cross_shard_client: Arc<dyn CrossShardClient>,
    ) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                // We need two extra threads for the cross-shard commit receiver and the thread
                // that is blocked on waiting for execute block to finish.
                .thread_name(move |i| format!("sharded-executor-shard-{}-{}", shard_id, i))
                .num_threads(num_threads + 2)
                .build()
                .unwrap(),
        );
        Self {
            shard_id,
            num_shards,
            executor_thread_pool,
            coordinator_client,
            cross_shard_client,
        }
    }

    fn execute_sub_block(
        &self,
        sub_block: SubBlock<AnalyzedTransaction>,
        round: usize,
        state_view: &S,
        config: BlockExecutorConfig,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        disable_speculative_logging();
        trace!(
            "executing sub block for shard {} and round {}",
            self.shard_id,
            round
        );
        let cross_shard_commit_sender =
            CrossShardCommitSender::new(self.shard_id, self.cross_shard_client.clone(), &sub_block);
        Self::execute_transactions_with_dependencies(
            Some(self.shard_id),
            self.executor_thread_pool.clone(),
            sub_block.into_transactions_with_deps(),
            self.cross_shard_client.clone(),
            Some(cross_shard_commit_sender),
            round,
            state_view,
            config,
        )
    }

    pub fn execute_transactions_with_dependencies(
        shard_id: Option<ShardId>, // None means execution on global shard
        executor_thread_pool: Arc<rayon::ThreadPool>,
        transactions: Vec<TransactionWithDependencies<AnalyzedTransaction>>,
        cross_shard_client: Arc<dyn CrossShardClient>,
        cross_shard_commit_sender: Option<CrossShardCommitSender>,
        round: usize,
        state_view: &S,
        config: BlockExecutorConfig,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let (callback, callback_receiver) = oneshot::channel();

        let cross_shard_state_view = Arc::new(CrossShardStateView::create_cross_shard_state_view(
            state_view,
            &transactions,
        ));

        let cross_shard_state_view_clone = cross_shard_state_view.clone();
        let cross_shard_client_clone = cross_shard_client.clone();

        let aggr_overridden_state_view = Arc::new(AggregatorOverriddenStateView::new(
            cross_shard_state_view.as_ref(),
            TOTAL_SUPPLY_AGGR_BASE_VAL,
        ));

        let signature_verified_transactions: Vec<SignatureVerifiedTransaction> = transactions
            .into_iter()
            .map(|txn| txn.into_txn().into_txn())
            .collect();
        let executor_thread_pool_clone = executor_thread_pool.clone();

        executor_thread_pool.clone().scope(|s| {
            s.spawn(move |_| {
                CrossShardCommitReceiver::start(
                    cross_shard_state_view_clone,
                    cross_shard_client,
                    round,
                );
            });
            s.spawn(move |_| {
                let txn_provider = DefaultTxnProvider::new(signature_verified_transactions);
                let ret = AptosVMBlockExecutorWrapper::execute_block_on_thread_pool(
                    executor_thread_pool,
                    &txn_provider,
                    aggr_overridden_state_view.as_ref(),
                    // Since we execute blocks in parallel, we cannot share module caches, so each
                    // thread has its own caches.
                    &AptosModuleCacheManager::new(),
                    config,
                    TransactionSliceMetadata::unknown(),
                    cross_shard_commit_sender,
                )
                .map(BlockOutput::into_transaction_outputs_forced);
                if let Some(shard_id) = shard_id {
                    trace!(
                        "executed sub block for shard {} and round {}",
                        shard_id,
                        round
                    );
                    // Send a self message to stop the cross-shard commit receiver.
                    cross_shard_client_clone.send_cross_shard_msg(
                        shard_id,
                        round,
                        CrossShardMsg::StopMsg,
                    );
                } else {
                    trace!("executed block for global shard and round {}", round);
                    // Send a self message to stop the cross-shard commit receiver.
                    cross_shard_client_clone.send_global_msg(CrossShardMsg::StopMsg);
                }
                callback.send(ret).unwrap();
                executor_thread_pool_clone.spawn(move || {
                    // Explicit async drop
                    drop(txn_provider);
                });
            });
        });

        block_on(callback_receiver).unwrap()
    }

    fn execute_block(
        &self,
        transactions: SubBlocksForShard<AnalyzedTransaction>,
        state_view: &S,
        config: BlockExecutorConfig,
    ) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        let mut result = vec![];
        for (round, sub_block) in transactions.into_sub_blocks().into_iter().enumerate() {
            let _timer = SHARDED_BLOCK_EXECUTION_BY_ROUNDS_SECONDS
                .with_label_values(&[&self.shard_id.to_string(), &round.to_string()])
                .start_timer();
            SHARDED_BLOCK_EXECUTOR_TXN_COUNT
                .with_label_values(&[&self.shard_id.to_string(), &round.to_string()])
                .observe(sub_block.transactions.len() as f64);
            info!(
                "executing sub block for shard {} and round {}, number of txns {}",
                self.shard_id,
                round,
                sub_block.transactions.len()
            );
            result.push(self.execute_sub_block(sub_block, round, state_view, config.clone())?);
            trace!(
                "Finished executing sub block for shard {} and round {}",
                self.shard_id,
                round
            );
        }
        Ok(result)
    }

    pub fn start(&self) {
        trace!(
            "Shard starting, shard_id={}, num_shards={}.",
            self.shard_id,
            self.num_shards
        );
        let mut num_txns = 0;
        loop {
            let command = self.coordinator_client.receive_execute_command();
            match command {
                ExecutorShardCommand::ExecuteSubBlocks(
                    state_view,
                    transactions,
                    concurrency_level_per_shard,
                    onchain_config,
                ) => {
                    num_txns += transactions.num_txns();
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        num_txns
                    );
                    let exe_timer = SHARDED_EXECUTOR_SERVICE_SECONDS
                        .with_label_values(&[&self.shard_id.to_string(), "execute_block"])
                        .start_timer();
                    let ret = self.execute_block(
                        transactions,
                        state_view.as_ref(),
                        BlockExecutorConfig {
                            local: BlockExecutorLocalConfig::default_with_concurrency_level(
                                concurrency_level_per_shard,
                            ),
                            onchain: onchain_config,
                        },
                    );
                    drop(state_view);
                    drop(exe_timer);

                    let _result_tx_timer = SHARDED_EXECUTOR_SERVICE_SECONDS
                        .with_label_values(&[&self.shard_id.to_string(), "result_tx"])
                        .start_timer();
                    self.coordinator_client.send_execution_result(ret);
                },
                ExecutorShardCommand::Stop => {
                    break;
                },
            }
        }
        let exe_time = SHARDED_EXECUTOR_SERVICE_SECONDS
            .get_metric_with_label_values(&[&self.shard_id.to_string(), "execute_block"])
            .unwrap()
            .get_sample_sum();
        info!(
            "Shard {} is shutting down; On shard execution tps {} txns/s ({} txns / {} s)",
            self.shard_id,
            (num_txns as f64 / exe_time),
            num_txns,
            exe_time
        );
    }
}
