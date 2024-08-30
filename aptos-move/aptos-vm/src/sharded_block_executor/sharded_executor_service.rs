// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use crate::{
    block_executor::BlockAptosVM,
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
        ExecuteV3PartitionCommand, ExecutorShardCommand,
    },
    AptosVM,
};
use aptos_block_executor::txn_provider::{
    default::DefaultTxnProvider,
    sharded::{CrossShardClientForV3, ShardedTxnProvider},
};

use aptos_logger::{info, trace};
use aptos_types::{
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorLocalConfig},
        partitioner::{PartitionV3, ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies},
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
use rayon::{
    iter::ParallelIterator,
    prelude::{IndexedParallelIterator, IntoParallelIterator},
};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::SystemTime;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use aptos_block_executor::txn_commit_hook::OutputStreamHook;
use aptos_block_executor::txn_provider::sharded::ShardedTransaction;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use aptos_types::state_store::state_key::StateKey;
use crate::block_executor::AptosTransactionOutput;
use crate::sharded_block_executor::ExecuteV3PartitionStreamedInitCommand;

pub struct ShardedExecutorService<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    num_shards: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    coordinator_client: Arc<Mutex<dyn CoordinatorClient<S>>>,
    cross_shard_client: Arc<dyn CrossShardClient>,
    v3_client: Arc<dyn CrossShardClientForV3<SignatureVerifiedTransaction, VMStatus>>,
}

impl<S: StateView + Sync + Send + 'static> ShardedExecutorService<S> {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        coordinator_client: Arc<Mutex<dyn CoordinatorClient<S>>>,
        cross_shard_client: Arc<dyn CrossShardClient>,
        v3_client: Arc<dyn CrossShardClientForV3<SignatureVerifiedTransaction, VMStatus>>,
    ) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                // We need two extra threads for the cross-shard commit receiver and the thread
                // that is blocked on waiting for execute block to finish.
                .thread_name(move |i| format!("sharded-executor-shard-{}-{}", shard_id, i))
                .num_threads(num_threads + 2 + 1)
                .build()
                .unwrap(),
        );
        Self {
            shard_id,
            num_shards,
            executor_thread_pool,
            coordinator_client,
            cross_shard_client,
            v3_client,
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
        info!(
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
                let txn_provider = Arc::new(DefaultTxnProvider::new(signature_verified_transactions));
                let ret = BlockAptosVM::execute_block(
                    executor_thread_pool,
                    txn_provider.clone(),
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

    pub fn start(&mut self) {
        trace!(
            "Shard starting, shard_id={}, num_shards={}.",
            self.shard_id,
            self.num_shards
        );

        let mut cumulative_txns = 0;
        let mut i = 0;
        loop {
            // info!("Looping back to recv cmd after execution of a block********************");
            let mut command = self.coordinator_client.lock().unwrap().receive_execute_command();
            match command {
                ExecutorShardCommand::ExecuteSubBlocks(
                    state_view,
                    transactions,
                    concurrency_level_per_shard,
                    onchain_config,
                ) => {
                    cumulative_txns += transactions.num_txns();
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        cumulative_txns
                    );
                    let exe_timer = SHARDED_EXECUTOR_SERVICE_SECONDS
                        .with_label_values(&[&self.shard_id.to_string(), "execute_block"])
                        .start_timer();
                    let ret = self.execute_block(
                        transactions,
                        state_view.as_ref(),
                        BlockExecutorConfig {
                            local: BlockExecutorLocalConfig {
                                concurrency_level: concurrency_level_per_shard,
                                allow_fallback: true,
                                discard_failed_blocks: false,
                            },
                            onchain: onchain_config,
                        },
                    );
                    drop(state_view);
                    drop(exe_timer);

                    let _result_tx_timer = SHARDED_EXECUTOR_SERVICE_SECONDS
                        .with_label_values(&[&self.shard_id.to_string(), "result_tx"])
                        .start_timer();
                    self.coordinator_client.lock().unwrap().send_execution_result(ret);
                },
                ExecutorShardCommand::ExecuteV3Partition(cmd) => {
                    let ExecuteV3PartitionCommand {
                        state_view,
                        partition,
                        concurrency_level_per_shard,
                        onchain_config,
                    } = cmd;
                    let PartitionV3 {
                        block_id,
                        txns,
                        global_idxs,
                        local_idx_by_global,
                        key_sets_by_dep,
                        follower_shard_sets,
                    } = partition;
                    let processed_txns: Vec<ShardedTransaction<SignatureVerifiedTransaction>> = self.executor_thread_pool.install(|| {
                        txns.into_par_iter()
                            .with_min_len(25)
                            .map(|analyzed_txn| {
                                ShardedTransaction::Txn(Arc::new(analyzed_txn.into_txn()))
                            })
                            .collect()
                    });
                    cumulative_txns += processed_txns.len();
                    let txn_provider = ShardedTxnProvider::new(
                        block_id,
                        self.num_shards,
                        self.shard_id,
                        self.v3_client.clone(),
                        Arc::new(processed_txns),
                        global_idxs,
                        local_idx_by_global,
                        key_sets_by_dep,
                        follower_shard_sets,
                        None::<OutputStreamHookImpl>,
                    );

                    disable_speculative_logging();
                    let result = BlockAptosVM::execute_block(
                        self.executor_thread_pool.clone(),
                        Arc::new(txn_provider),
                        state_view.as_ref(),
                        BlockExecutorConfig {
                            local: BlockExecutorLocalConfig {
                                concurrency_level: concurrency_level_per_shard,
                                allow_fallback: true,
                                discard_failed_blocks: false,
                            },
                            onchain: onchain_config,
                        },
                        None::<CrossShardCommitSender>,
                    ).map(BlockOutput::into_transaction_outputs_forced);

                    // Wrap the 1D result as a 2D result so we can reuse the existing `result_rxs`.
                    let wrapped_2d_result = result.map(|output_vec| vec![output_vec]);
                    self.coordinator_client.lock().unwrap()
                        .send_execution_result(wrapped_2d_result)
                },
                ExecutorShardCommand::ExecuteV3PartitionStreamedInit(cmd) => {
                    let ExecuteV3PartitionStreamedInitCommand {
                        state_view,
                        blocking_transactions_provider,
                        stream_results_receiver,
                        num_txns,
                        onchain_config,
                    } = cmd;
                    cumulative_txns += num_txns;

                    //let execution_done = Arc::new(AtomicBool::new(false));
                   // let execution_done_clone = execution_done.clone();

                    let coordinator_client_clone = self.coordinator_client.clone();
                    let stream_results_thread = thread::spawn(move || {
                        let batch_size = 200;
                        let mut num_outputs_received: usize = 0;
                        let mut seq_num: u64 = 0;
                        let mut rng = StdRng::from_entropy();
                        let random_number = rng.gen_range(0, u64::MAX);
                        let mut curr_batch = vec![];
                        loop {
                            let ret = stream_results_receiver.recv().unwrap();
                            let txn_idx_output = TransactionIdxAndOutput {
                                txn_idx: ret.0,
                                txn_output: ret.1,
                            };
                            num_outputs_received += 1;
                            //info!("num_outputs_received: {}; total txns: {}", num_outputs_received, num_txns);
                            curr_batch.push(txn_idx_output);
                            if num_outputs_received == num_txns { //todo: check if this works
                            //if execution_done_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                if !curr_batch.is_empty() {
                                    coordinator_client_clone.lock().unwrap().stream_execution_result(curr_batch, random_number as usize, seq_num);
                                }
                                break;
                            }
                            if curr_batch.len() == batch_size {
                                coordinator_client_clone.lock().unwrap().stream_execution_result(curr_batch, random_number as usize, seq_num);
                                curr_batch = vec![];
                                seq_num += 1;
                            }
                        }
                    });

                    disable_speculative_logging();
                    let exe_timer = SHARDED_EXECUTOR_SERVICE_SECONDS
                        .with_label_values(&[&self.shard_id.to_string(), "execute_block"])
                        .start_timer();
                    let result = BlockAptosVM::execute_block(
                        self.executor_thread_pool.clone(),
                        Arc::new(blocking_transactions_provider),
                        state_view.as_ref(),
                        BlockExecutorConfig {
                            local: BlockExecutorLocalConfig {
                                concurrency_level: AptosVM::get_concurrency_level(),
                                allow_fallback: true,
                                discard_failed_blocks: false,
                            },
                            onchain: onchain_config,
                        },
                        None::<CrossShardCommitSender>,
                    ).map(BlockOutput::into_transaction_outputs_forced);
                    drop(state_view);
                    drop(exe_timer);

                    self.coordinator_client.lock().unwrap().record_execution_complete_time_on_shard();

                    /*stream_results_tx.send(TransactionIdxAndOutput {
                        txn_idx: u32::MAX,
                        txn_output: TransactionOutput::default(),
                    }).unwrap(); // todo: this logic probably has a race condition*/
                   // execution_done.store(true, std::sync::atomic::Ordering::Relaxed); // todo: this logic probably has a race condition
                    stream_results_thread.join().unwrap();
                    self.coordinator_client.lock().unwrap().reset_state_view();
                    let exe_time = SHARDED_EXECUTOR_SERVICE_SECONDS
                        .get_metric_with_label_values(&[&self.shard_id.to_string(), "execute_block"])
                        .unwrap()
                        .get_sample_sum();
                    info!(
                        "On shard execution tps {} txns/s ({} txns / {} s)",
                        cumulative_txns as f64 / exe_time,
                        cumulative_txns,
                        exe_time
                    );
                    //info!("Shard {} finished streaming results", self.shard_id);
                },
                ExecutorShardCommand::Stop => {
                    break;
                },
            };
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
pub enum V3CmdsOrMetaData {
    Cmds(V3Cmds),
    MetaData(V3MetaData),
}

#[derive(Clone, Debug, Serialize)]
pub enum V3CmdsOrMetaDataRef<'a> {
    Cmds(V3CmdsRef<'a>),
    MetaData(V3MetaDataRef<'a>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct V3MetaData {
    pub num_txns: usize,
    pub global_idxs: Vec<u32>,
    pub local_idx_by_global: HashMap<u32, usize>,
    pub key_sets_by_dep: HashMap<u32, HashSet<StateKey>>,
    pub follower_shard_sets: Vec<HashSet<usize>>,
    pub onchain_config: BlockExecutorConfigFromOnchain,
}

#[derive(Clone, Debug, Serialize)]
pub struct V3MetaDataRef<'a>  {
    pub num_txns: usize,
    pub global_idxs: &'a [u32],
    pub local_idx_by_global: &'a HashMap<u32, usize>,
    pub key_sets_by_dep: &'a HashMap<u32, HashSet<StateKey>>,
    pub follower_shard_sets: &'a [HashSet<usize>],
    pub onchain_config: &'a BlockExecutorConfigFromOnchain,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct V3Cmds {
    pub cmds: Vec<AnalyzedTransaction>,
    pub num_txns_total: usize,
    pub batch_start_index: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct V3CmdsRef<'a>  {
    pub cmds: &'a [&'a AnalyzedTransaction],
    pub num_txns_total: usize,
    pub batch_start_index: usize,
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

pub struct OutputStreamHookImpl {
    pub stream_results_tx: Sender<(TxnIndex, TransactionOutput)>,
}

impl OutputStreamHook for OutputStreamHookImpl {
    type Output = AptosTransactionOutput;

    fn stream_output(&self, txn_idx: TxnIndex, output: &Self::Output) {
        let txn_output = output.committed_output();
        self.stream_results_tx.send((txn_idx, txn_output.clone())).unwrap();
    }
}
