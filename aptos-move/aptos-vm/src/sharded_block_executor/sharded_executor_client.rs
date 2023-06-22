// Copyright © Aptos Foundation

use crate::{
    block_executor::BlockAptosExecutor,
    sharded_block_executor::{
        block_executor_client::BlockExecutorClient, cross_shard_client::CrossShardCommitReceiver,
        cross_shard_commit_sender::CrossShardCommitSender,
        cross_shard_state_view::CrossShardStateView, messages::CrossShardMsg,
    },
};
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
    message_rx: Arc<Mutex<Receiver<CrossShardMsg>>>,
    // The senders of cross-shard messages to other shards.
    message_txs: Arc<Vec<Mutex<Sender<CrossShardMsg>>>>,
}

impl ShardedExecutorClient {
    pub fn new(
        shard_id: ShardId,
        num_threads: usize,
        message_txs: Vec<Sender<CrossShardMsg>>,
        message_rx: Receiver<CrossShardMsg>,
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
            message_rx: Arc::new(Mutex::new(message_rx)),
            message_txs: Arc::new(message_txs.into_iter().map(Mutex::new).collect()),
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
            let (messages_tx, messages_rx) = std::sync::mpsc::channel();
            cross_shard_msg_txs.push(messages_tx);
            cross_shard_msg_rxs.push(messages_rx);
        }
        cross_shard_msg_rxs
            .into_iter()
            .enumerate()
            .map(|(shard_id, rx)| {
                Self::new(
                    shard_id as ShardId,
                    num_threads,
                    cross_shard_msg_txs.clone(),
                    rx,
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
        CrossShardStateView::new(cross_shard_state_key, base_view)
    }

    fn execute_sub_block<S: StateView + Sync + Send>(
        &self,
        sub_block: SubBlock<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let executor = Arc::new(BlockAptosExecutor::new(
            concurrency_level,
            sub_block.num_txns(),
            self.executor_thread_pool.clone(),
            maybe_block_gas_limit,
            CrossShardCommitSender::new(
                self.message_txs
                    .iter()
                    .map(|t| t.lock().unwrap().clone())
                    .collect(),
                &sub_block,
            ),
        ));

        let (callback, callback_receiver) = oneshot::channel();

        let message_rxs = self.message_rx.clone();
        let self_message_tx = Arc::new(Mutex::new(
            self.message_txs[self.shard_id].lock().unwrap().clone(),
        ));
        let cross_shard_state_view =
            Arc::new(self.create_cross_shard_state_view(state_view, &sub_block));
        let cross_shard_state_view_clone1 = cross_shard_state_view.clone();
        let cross_shard_state_view_clone2 = cross_shard_state_view.clone();
        self.executor_thread_pool.scope(|s| {
            s.spawn(move |_| {
                CrossShardCommitReceiver::start(
                    cross_shard_state_view_clone1,
                    &message_rxs.lock().unwrap(),
                );
            });
            s.spawn(move |_| {
                let ret = executor.execute_block(
                    BlockExecutorTransactions::Unsharded(sub_block.into_txns()),
                    cross_shard_state_view.as_ref(),
                );
                // Send a stop command to the cross-shard commit receiver.
                self_message_tx
                    .lock()
                    .unwrap()
                    .send(CrossShardMsg::StopMsg)
                    .unwrap();
                callback.send(ret).unwrap();
            });
        });
        block_on(callback_receiver).unwrap()
    }
}

impl BlockExecutorClient for ShardedExecutorClient {
    fn execute_block<S: StateView + Sync + Send>(
        &self,
        transactions: SubBlocksForShard<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let mut result = vec![];
        for sub_block in transactions.into_sub_blocks() {
            result.extend(self.execute_sub_block(
                sub_block,
                state_view,
                concurrency_level,
                maybe_block_gas_limit,
            )?);
        }
        Ok(result)
    }
}
