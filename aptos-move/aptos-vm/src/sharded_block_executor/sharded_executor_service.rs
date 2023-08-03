// Copyright © Aptos Foundation

use crate::{
    block_executor::BlockAptosVM,
    sharded_block_executor::{
        coordinator_client::CoordinatorClient,
        counters::{SHARDED_BLOCK_EXECUTION_SECONDS, SHARDED_BLOCK_EXECUTOR_TXN_COUNT},
        cross_shard_client::{CrossShardClient, CrossShardCommitReceiver, CrossShardCommitSender},
        cross_shard_state_view::CrossShardStateView,
        messages::CrossShardMsg,
        ExecutorShardCommand,
    },
};
use aptos_logger::{info, trace};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{ShardId, SubBlock, SubBlocksForShard},
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
};
use futures::{channel::oneshot, executor::block_on};
use move_core_types::vm_status::VMStatus;
use std::{collections::HashSet, sync::Arc};

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

    fn create_cross_shard_state_view<'a>(
        &self,
        base_view: &'a S,
        sub_block: &SubBlock<AnalyzedTransaction>,
        round_id: usize,
    ) -> CrossShardStateView<'a, S> {
        let mut cross_shard_state_key = HashSet::new();
        for txn in &sub_block.transactions {
            for (_, storage_locations) in txn.cross_shard_dependencies.required_edges_iter() {
                for storage_location in storage_locations {
                    cross_shard_state_key.insert(storage_location.clone().into_state_key());
                }
            }
        }
        CrossShardStateView::new(self.shard_id, round_id, cross_shard_state_key, base_view)
    }

    fn execute_sub_block(
        &self,
        sub_block: SubBlock<AnalyzedTransaction>,
        round: usize,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        trace!(
            "executing sub block for shard {} and round {}",
            self.shard_id,
            round
        );
        let cross_shard_commit_sender =
            CrossShardCommitSender::new(self.shard_id, self.cross_shard_client.clone(), &sub_block);

        let (callback, callback_receiver) = oneshot::channel();

        let cross_shard_state_view =
            Arc::new(self.create_cross_shard_state_view(state_view, &sub_block, round));
        let cross_shard_state_view_clone = cross_shard_state_view.clone();
        let cross_shard_client = self.cross_shard_client.clone();
        let cross_shard_client_clone = cross_shard_client.clone();
        self.executor_thread_pool.scope(|s| {
            s.spawn(move |_| {
                CrossShardCommitReceiver::start(
                    cross_shard_state_view_clone,
                    cross_shard_client,
                    round,
                );
            });
            s.spawn(move |_| {
                let ret = BlockAptosVM::execute_block(
                    self.executor_thread_pool.clone(),
                    sub_block
                        .into_txns()
                        .into_iter()
                        .map(|txn| txn.into_txn())
                        .collect(),
                    cross_shard_state_view.as_ref(),
                    concurrency_level,
                    maybe_block_gas_limit,
                    Some(cross_shard_commit_sender),
                );
                trace!(
                    "executed sub block for shard {} and round {}",
                    self.shard_id,
                    round
                );
                // Send a self message to stop the cross-shard commit receiver.
                cross_shard_client_clone.send_cross_shard_msg(
                    self.shard_id,
                    round,
                    CrossShardMsg::StopMsg,
                );
                callback.send(ret).unwrap();
            });
        });
        block_on(callback_receiver).unwrap()
    }

    fn execute_block(
        &self,
        transactions: SubBlocksForShard<AnalyzedTransaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        let mut result = vec![];
        for (round, sub_block) in transactions.into_sub_blocks().into_iter().enumerate() {
            let _timer = SHARDED_BLOCK_EXECUTION_SECONDS
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
            result.push(self.execute_sub_block(
                sub_block,
                round,
                state_view,
                concurrency_level,
                maybe_block_gas_limit,
            )?);
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
        loop {
            let command = self.coordinator_client.receive_execute_command();
            match command {
                ExecutorShardCommand::ExecuteSubBlocks(
                    state_view,
                    transactions,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                ) => {
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        transactions.num_txns()
                    );
                    let ret = self.execute_block(
                        transactions,
                        state_view.as_ref(),
                        concurrency_level_per_shard,
                        maybe_block_gas_limit,
                    );
                    drop(state_view);
                    self.coordinator_client.send_execution_result(ret);
                },
                ExecutorShardCommand::Stop => {
                    break;
                },
            }
        }
        trace!("Shard {} is shutting down", self.shard_id);
    }
}
