// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use crate::{AptosVM, block_executor::BlockAptosVM, sharded_block_executor::{
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
    streamed_transactions_provider::StreamedTransactionsProvider,
}};
use aptos_logger::{info, trace};
use aptos_types::{
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorLocalConfig},
        partitioner::{ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies},
    },
    state_store::StateView,
    transaction::{
        analyzed_transaction::AnalyzedTransaction,
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput,
        TransactionOutput,
    },
};
use aptos_vm_logging::disable_speculative_logging;
use crossbeam_channel::{unbounded, Receiver, Sender};
use futures::{channel::oneshot, executor::block_on};
use move_core_types::vm_status::VMStatus;
use std::sync::{Arc, Mutex};
use std::thread;
use rayon::prelude::IntoParallelIterator;
use serde::{Deserialize, Serialize};
use aptos_block_executor::transaction_provider::TxnProvider;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use crate::sharded_block_executor::streamed_transactions_provider::BlockingTransactionsProvider;
use crate::sharded_block_executor::StreamedExecutorShardCommand;

pub struct ShardedExecutorService<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    num_shards: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    coordinator_client: Arc<Mutex<dyn CoordinatorClient<S>>>,
    cross_shard_client: Arc<dyn CrossShardClient>,
}

impl<S: StateView + Sync + Send + 'static> ShardedExecutorService<S> {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        coordinator_client: Arc<Mutex<dyn CoordinatorClient<S>>>,
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
        streamed_transactions_provider: Arc<BlockingTransactionsProvider>,
        round: usize,
        state_view: &S,
        config: BlockExecutorConfig,
        shard_txns_start_index: TxnIndex,
        stream_result_tx: Sender<TransactionIdxAndOutput>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        disable_speculative_logging();
        trace!(
            "executing sub block for shard {} and round {}",
            self.shard_id,
            round
        );
        let cross_shard_commit_sender =
            CrossShardCommitSender::create_cross_shard_commit_sender_with_no_dependent_edges(self.shard_id, self.cross_shard_client.clone(), shard_txns_start_index, stream_result_tx);
        Self::execute_transactions_with_dependencies(
            Some(self.shard_id),
            self.executor_thread_pool.clone(),
            streamed_transactions_provider,
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
        streamed_transactions_provider: Arc<BlockingTransactionsProvider>,
        cross_shard_client: Arc<dyn CrossShardClient>,
        cross_shard_commit_sender: Option<CrossShardCommitSender>,
        round: usize,
        state_view: &S,
        config: BlockExecutorConfig,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let (callback, callback_receiver) = oneshot::channel();

        /*let cross_shard_state_view = Arc::new(CrossShardStateView::create_cross_shard_state_view(
            state_view,
            &transactions,
        ));*/
        let cross_shard_state_view = Arc::new(CrossShardStateView::new(
            HashSet::new(),
            state_view,
        ));

        let cross_shard_state_view_clone = cross_shard_state_view.clone();
        let cross_shard_client_clone = cross_shard_client.clone();

        let aggr_overridden_state_view = Arc::new(AggregatorOverriddenStateView::new(
            cross_shard_state_view.as_ref(),
            TOTAL_SUPPLY_AGGR_BASE_VAL,
        ));

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
                let ret = BlockAptosVM::execute_block(
                    executor_thread_pool,
                    streamed_transactions_provider.as_ref(),
                    aggr_overridden_state_view.as_ref(),
                    config,
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
                    drop(streamed_transactions_provider);
                });
            });
        });

        block_on(callback_receiver).unwrap()
    }

    fn execute_block(
        &self,
        streamed_transactions_provider: Arc<BlockingTransactionsProvider>,
        state_view: &S,
        config: BlockExecutorConfig,
        shard_txns_start_index: TxnIndex,
        stream_result_tx: Sender<TransactionIdxAndOutput>,
    ) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        let mut result = vec![];
        //for (round, sub_block) in transactions.into_sub_blocks().into_iter().enumerate() {
            /*let _timer = SHARDED_BLOCK_EXECUTION_BY_ROUNDS_SECONDS
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
            );*/
            result.push(self.execute_sub_block(
                streamed_transactions_provider,
                0,
                state_view,
                config.clone(),
                shard_txns_start_index,
                stream_result_tx.clone(),
            )?);
            trace!(
                "Finished executing sub block for shard {} and round {}",
                self.shard_id,
                0//round
            );
        //}
        Ok(result)
    }

    pub fn start(&mut self) {
        trace!(
            "Shard starting, shard_id={}, num_shards={}.",
            self.shard_id,
            self.num_shards
        );

        let mut cumulative_txns = 0;
        loop {
            let mut command = self.coordinator_client.lock().unwrap().receive_execute_command_stream();
            let (state_view, num_txns_in_the_block, shard_txns_start_index, onchain_config, blocking_transactions_provider) = match command {
                StreamedExecutorShardCommand::InitBatch(
                    state_view,
                    transactions,
                    num_txns_in_the_block,
                    shard_txns_start_index,
                    onchain_config,
                    batch_start_index,
                    blocking_transactions_provider,
                ) => {
                    if transactions.len() == num_txns_in_the_block {
                        self.coordinator_client.lock().unwrap().reset_block_init();
                    }
                    let _ = transactions.into_iter().enumerate().for_each(|(idx, txn)| {
                        blocking_transactions_provider.set_txn(idx + batch_start_index, txn);
                    });
                    (state_view, num_txns_in_the_block, shard_txns_start_index, onchain_config, blocking_transactions_provider)
                },
                StreamedExecutorShardCommand::Stop => {
                    break;
                },
            };
            cumulative_txns += num_txns_in_the_block;
            /*let blocking_transactions_provider_clone = blocking_transactions_provider.clone();

            let coordinator_client_clone_2 = self.coordinator_client.clone();
            thread::spawn(move || {
                let mut num_txns_processed = 0;
                loop {
                    num_txns_processed += transactions.len();
                    let _ = transactions.into_iter().enumerate().for_each(|(idx, txn)| {
                        blocking_transactions_provider_clone.set_txn(idx + batch_start_index, txn);
                    });
                    if num_txns_processed == num_txns_in_the_block {
                        coordinator_client_clone_2.lock().unwrap().reset_block_init();
                        break;
                    }
                    let command2 = coordinator_client_clone_2.lock().unwrap().receive_execute_command_stream();
                    let txnsAndStIdx = match command2 {
                        StreamedExecutorShardCommand::InitBatch(
                            _,
                            _,
                            _,
                            _,
                            _,
                            _,
                        ) => {
                            panic!("Init Batch must not be called before executing all txns in the block");
                        },
                        StreamedExecutorShardCommand::ExecuteBatch(
                            transactions,
                            batch_start_index,
                        ) => {
                            (transactions, batch_start_index)
                        },
                        StreamedExecutorShardCommand::Stop => {
                            break;
                        },
                    };
                    transactions = txnsAndStIdx.0;
                    batch_start_index = txnsAndStIdx.1;
                }
            });*/

            let (stream_results_tx, stream_results_rx) = unbounded();
            let coordinator_client_clone = self.coordinator_client.clone();
            let stream_results_thread = thread::spawn(move || {
                let batch_size = 200;
                let mut curr_batch = vec![];
                loop {
                    let txn_idx_output: TransactionIdxAndOutput = stream_results_rx.recv().unwrap();
                    if txn_idx_output.txn_idx == u32::MAX && !curr_batch.is_empty() {
                        coordinator_client_clone.lock().unwrap().stream_execution_result(curr_batch);
                        break;
                    }
                    curr_batch.push(txn_idx_output);
                    if curr_batch.len() == batch_size {
                        coordinator_client_clone.lock().unwrap().stream_execution_result(curr_batch);
                        curr_batch = vec![];
                    }
                }
            });

            trace!(
                    "Shard {} received ExecuteBlock command of block size {} ",
                    self.shard_id,
                    num_txns_in_the_block
                );
            let exe_timer = SHARDED_EXECUTOR_SERVICE_SECONDS
                .with_label_values(&[&self.shard_id.to_string(), "execute_block"])
                .start_timer();
            let ret = self.execute_block(
                blocking_transactions_provider,
                state_view.as_ref(),
                BlockExecutorConfig {
                    local: BlockExecutorLocalConfig {
                        concurrency_level: AptosVM::get_concurrency_level(),
                        allow_fallback: true,
                        discard_failed_blocks: false,
                    },
                    onchain: onchain_config,
                },
                shard_txns_start_index as TxnIndex,
                stream_results_tx.clone(),
            );
            drop(state_view);
            drop(exe_timer);

            self.coordinator_client.lock().unwrap().record_execution_complete_time_on_shard();

            stream_results_tx.send(TransactionIdxAndOutput {
                txn_idx: u32::MAX,
                txn_output: TransactionOutput::default(),
            }).unwrap();
            stream_results_thread.join().unwrap();
        }

        let exe_time = SHARDED_EXECUTOR_SERVICE_SECONDS
            .get_metric_with_label_values(&[&self.shard_id.to_string(), "execute_block"])
            .unwrap()
            .get_sample_sum();
        info!(
            "Shard {} is shutting down; On shard execution tps {} txns/s ({} txns / {} s)",
            self.shard_id,
            (cumulative_txns as f64 / exe_time),
            cumulative_txns,
            exe_time
        );
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CmdsAndMetaData {
    pub cmds: Vec<AnalyzedTransaction>,
    pub num_txns: usize,
    pub shard_txns_start_index: usize,
    pub onchain_config: BlockExecutorConfigFromOnchain,
    pub batch_start_index: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CmdsAndMetaDataRef<'a>  {
    pub cmds: &'a [&'a AnalyzedTransaction],
    pub num_txns: usize,
    pub shard_txns_start_index: usize,
    pub onchain_config: &'a BlockExecutorConfigFromOnchain,
    pub batch_start_index: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionIdxAndOutput {
    pub txn_idx: TxnIndex,
    pub txn_output: TransactionOutput,
}