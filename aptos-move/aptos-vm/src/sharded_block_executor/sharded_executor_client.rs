// Copyright © Aptos Foundation

use crate::{
    block_executor::BlockAptosVM,
    sharded_block_executor::{
        block_executor_client::BlockExecutorClient,
        counters::SHARDED_BLOCK_EXECUTION_SECONDS,
        cross_shard_client::{CrossShardCommitReceiver, CrossShardCommitSender},
        cross_shard_state_view::CrossShardStateView,
        messages::CrossShardMsg,
    },
};
use aptos_block_partitioner::sharded_block_partitioner::MAX_ALLOWED_PARTITIONING_ROUNDS;
use aptos_logger::{info, trace};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{
        BlockExecutorTransactions, ShardId, SubBlock, SubBlocksForShard,
    },
    transaction::{Transaction, TransactionOutput},
};
use futures::{channel::oneshot, executor::block_on};
use move_core_types::vm_status::VMStatus;
use std::{
    collections::HashSet,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

pub struct ShardedExecutorClient {
    shard_id: ShardId,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    // The receivers of cross shard messages from other shards per round.
    message_rxs: Arc<Vec<Mutex<Receiver<CrossShardMsg>>>>,
    // The senders of cross-shard messages to other shards per round.
    message_txs: Arc<Vec<Vec<Mutex<Sender<CrossShardMsg>>>>>,
}

impl ShardedExecutorClient {
    pub fn new(
        shard_id: ShardId,
        num_threads: usize,
        message_rxs: Vec<Receiver<CrossShardMsg>>,
        message_txs: Vec<Vec<Sender<CrossShardMsg>>>,
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
            executor_thread_pool,
            message_rxs: Arc::new(message_rxs.into_iter().map(Mutex::new).collect()),
            message_txs: Arc::new(
                message_txs
                    .into_iter()
                    .map(|inner_vec| inner_vec.into_iter().map(Mutex::new).collect())
                    .collect(),
            ),
        }
    }

    pub fn create_sharded_executor_clients(
        num_shards: usize,
        num_threads: Option<usize>,
    ) -> Vec<Self> {
        let num_threads = num_threads
            .unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);
        let mut cross_shard_msg_txs = vec![];
        let mut cross_shard_msg_rxs = vec![];
        for _ in 0..num_shards {
            let mut current_shard_msg_txs = vec![];
            let mut current_shard_msg_rxs = vec![];
            for _ in 0..MAX_ALLOWED_PARTITIONING_ROUNDS {
                let (messages_tx, messages_rx) = std::sync::mpsc::channel();
                current_shard_msg_txs.push(messages_tx);
                current_shard_msg_rxs.push(messages_rx);
            }
            cross_shard_msg_txs.push(current_shard_msg_txs);
            cross_shard_msg_rxs.push(current_shard_msg_rxs);
        }
        cross_shard_msg_rxs
            .into_iter()
            .enumerate()
            .map(|(shard_id, rxs)| {
                Self::new(
                    shard_id as ShardId,
                    num_threads,
                    rxs,
                    cross_shard_msg_txs.clone(),
                )
            })
            .collect()
    }

    fn create_cross_shard_state_view<'a, S: StateView + Sync + Send>(
        &self,
        base_view: &'a S,
        sub_block: &SubBlock<Transaction>,
    ) -> CrossShardStateView<'a, S> {
        let mut cross_shard_state_key = HashSet::new();
        for txn in &sub_block.transactions {
            for (_, storage_locations) in txn.cross_shard_dependencies.required_edges_iter() {
                for storage_location in storage_locations {
                    cross_shard_state_key.insert(storage_location.clone().into_state_key());
                }
            }
        }
        CrossShardStateView::new(self.shard_id, cross_shard_state_key, base_view)
    }

    fn execute_sub_block<S: StateView + Sync + Send>(
        &self,
        sub_block: SubBlock<Transaction>,
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
            CrossShardCommitSender::new(self.shard_id, self.message_txs.clone(), &sub_block);

        let (callback, callback_receiver) = oneshot::channel();

        let message_rxs = self.message_rxs.clone();
        let self_message_tx = Arc::new(Mutex::new(
            self.message_txs[self.shard_id][round]
                .lock()
                .unwrap()
                .clone(),
        ));
        let cross_shard_state_view =
            Arc::new(self.create_cross_shard_state_view(state_view, &sub_block));
        let cross_shard_state_view_clone1 = cross_shard_state_view.clone();
        self.executor_thread_pool.scope(|s| {
            s.spawn(move |_| {
                if round != 0 {
                    // If this is not the first round, start the cross-shard commit receiver.
                    // this is a bit ugly, we can get rid of this when we have round number
                    // information in the cross shard dependencies.
                    CrossShardCommitReceiver::start(
                        cross_shard_state_view_clone1,
                        &message_rxs[round].lock().unwrap(),
                    );
                }
            });
            s.spawn(move |_| {
                let ret = BlockAptosVM::execute_block(
                    self.executor_thread_pool.clone(),
                    BlockExecutorTransactions::Unsharded(sub_block.into_txns()),
                    cross_shard_state_view.as_ref(),
                    concurrency_level,
                    maybe_block_gas_limit,
                    Some(cross_shard_commit_sender),
                );
                // Send a stop command to the cross-shard commit receiver.
                if round != 0 {
                    self_message_tx
                        .lock()
                        .unwrap()
                        .send(CrossShardMsg::StopMsg)
                        .unwrap();
                }
                callback.send(ret).unwrap();
            });
        });
        let ret = block_on(callback_receiver).unwrap();
        trace!(
            "finished executing sub block for shard {} and round {}",
            self.shard_id,
            round
        );
        ret
    }
}

impl BlockExecutorClient for ShardedExecutorClient {
    fn execute_block<S: StateView + Sync + Send>(
        &self,
        transactions: SubBlocksForShard<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        let mut result = vec![];
        for (round, sub_block) in transactions.into_sub_blocks().into_iter().enumerate() {
            let _timer = SHARDED_BLOCK_EXECUTION_SECONDS
                .with_label_values(&[&self.shard_id.to_string(), &round.to_string()])
                .start_timer();
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
        }
        Ok(result)
    }
}
