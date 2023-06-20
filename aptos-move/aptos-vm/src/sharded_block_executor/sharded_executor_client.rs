// Copyright © Aptos Foundation

use crate::{
    block_executor::BlockAptosExecutor,
    sharded_block_executor::{
        block_executor_client::BlockExecutorClient, cross_shard_client::CrossShardCommitReceiver,
        cross_shard_commit_listener::CrossShardCommitListener, messages::CrossShardMsg,
    },
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{BlockExecutorTransactions, ShardId, SubBlocksForShard},
    transaction::{analyzed_transaction::StorageLocation, Transaction, TransactionOutput},
};
use futures::{channel::oneshot, executor::block_on};
use move_core_types::vm_status::VMStatus;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

pub struct ShardedExecutorClient {
    shard_id: ShardId,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    message_rx: Arc<Mutex<Receiver<CrossShardMsg>>>,
    // The senders of cross-shard messages to other shards.
    message_txs: Arc<Vec<Sender<CrossShardMsg>>>,
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
            message_txs: Arc::new(message_txs),
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
}

impl BlockExecutorClient for ShardedExecutorClient {
    fn execute_block<S: StateView + Sync + Send>(
        &self,
        transactions: SubBlocksForShard<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let executor = Arc::new(BlockAptosExecutor::new(
            concurrency_level,
            transactions.num_txns(),
            self.executor_thread_pool.clone(),
            maybe_block_gas_limit,
            CrossShardCommitListener::new(self.message_txs.as_ref(), &transactions),
        ));

        for (txn_index, txn) in transactions.txn_with_index_iter() {
            for (_, storage_locations) in txn.cross_shard_dependencies.required_edges_iter() {
                for storage_location in storage_locations.iter() {
                    match storage_location {
                        StorageLocation::Specific(state_key) => {
                            executor.mark_estimate(state_key, txn_index as TxnIndex);
                        },
                        _ => {
                            panic!("Unsupported storage location")
                        },
                    }
                }
            }
        }

        let executor_clone = executor.clone();
        let (callback, callback_receiver) = oneshot::channel();

        let message_rxs = self.message_rx.clone();
        let self_message_tx = Arc::new(Mutex::new(self.message_txs[self.shard_id].clone()));
        self.executor_thread_pool.scope(|s| {
            s.spawn(move |_| {
                CrossShardCommitReceiver::start(executor_clone, &message_rxs.lock().unwrap());
            });
            s.spawn(move |_| {
                let ret = executor
                    .execute_block(BlockExecutorTransactions::Sharded(transactions), state_view);
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
