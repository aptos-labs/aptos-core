// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{counters::NUM_EXECUTOR_SHARDS, executor_shard::ExecutorShard};
use aptos_block_partitioner::{BlockPartitioner, UniformPartitioner};
use aptos_logger::{error, info, trace};
use aptos_state_view::StateView;
use aptos_types::transaction::{Transaction, TransactionOutput};
use block_executor_client::BlockExecutorClient;
use move_core_types::vm_status::VMStatus;
use std::{
    marker::PhantomData,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

pub mod block_executor_client;
mod counters;
mod executor_shard;

/// A wrapper around sharded block executors that manages multiple shards and aggregates the results.
pub struct ShardedBlockExecutor<S: StateView + Sync + Send + 'static> {
    num_executor_shards: usize,
    partitioner: Arc<dyn BlockPartitioner>,
    command_txs: Vec<Sender<ExecutorShardCommand<S>>>,
    shard_threads: Vec<thread::JoinHandle<()>>,
    result_rxs: Vec<Receiver<Result<Vec<TransactionOutput>, VMStatus>>>,
    phantom: PhantomData<S>,
}

pub enum ExecutorShardCommand<S> {
    ExecuteBlock(Arc<S>, Vec<Transaction>, usize, Option<u64>),
    Stop,
}

impl<S: StateView + Sync + Send + 'static> ShardedBlockExecutor<S> {
    pub fn new<E: BlockExecutorClient + Sync + Send + 'static>(executor_clients: Vec<E>) -> Self {
        let mut command_txs = vec![];
        let mut result_rxs = vec![];
        let mut shard_join_handles = vec![];
        let num_executor_shards = executor_clients.len();
        for (i, executor_client) in executor_clients.into_iter().enumerate() {
            let (transactions_tx, transactions_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            command_txs.push(transactions_tx);
            result_rxs.push(result_rx);
            shard_join_handles.push(spawn_executor_shard(
                num_executor_shards,
                executor_client,
                i,
                transactions_rx,
                result_tx,
            ));
        }
        info!(
            "Creating a new ShardedBlockExecutor with {} shards",
            num_executor_shards
        );
        Self {
            num_executor_shards,
            partitioner: Arc::new(UniformPartitioner {}),
            command_txs,
            shard_threads: shard_join_handles,
            result_rxs,
            phantom: PhantomData,
        }
    }

    /// Execute a block of transactions in parallel by splitting the block into num_remote_executors partitions and
    /// dispatching each partition to a remote executor shard.
    pub fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<Transaction>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        NUM_EXECUTOR_SHARDS.set(self.num_executor_shards as i64);
        let block_partitions = self.partitioner.partition(block, self.num_executor_shards);
        // Number of partitions might be smaller than the number of executor shards in case of
        // block size is smaller than number of executor shards.
        let num_partitions = block_partitions.len();
        for (i, transactions) in block_partitions.into_iter().enumerate() {
            self.command_txs[i]
                .send(ExecutorShardCommand::ExecuteBlock(
                    state_view.clone(),
                    transactions,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                ))
                .unwrap();
        }
        // wait for all remote executors to send the result back and append them in order by shard id
        let mut aggregated_results = vec![];
        trace!("ShardedBlockExecutor Waiting for results");
        for i in 0..num_partitions {
            let result = self.result_rxs[i].recv().unwrap();
            aggregated_results.extend(result?);
        }
        Ok(aggregated_results)
    }
}

impl<S: StateView + Sync + Send + 'static> Drop for ShardedBlockExecutor<S> {
    /// Best effort stops all the executor shards and waits for the thread to finish.
    fn drop(&mut self) {
        // send stop command to all executor shards
        for command_tx in self.command_txs.iter() {
            if let Err(e) = command_tx.send(ExecutorShardCommand::Stop) {
                error!("Failed to send stop command to executor shard: {:?}", e);
            }
        }

        // wait for all executor shards to stop
        for shard_thread in self.shard_threads.drain(..) {
            shard_thread.join().unwrap_or_else(|e| {
                error!("Failed to join executor shard thread: {:?}", e);
            });
        }
    }
}

fn spawn_executor_shard<
    S: StateView + Sync + Send + 'static,
    E: BlockExecutorClient + Sync + Send + 'static,
>(
    num_executor_shards: usize,
    executor_client: E,
    shard_id: usize,
    command_rx: Receiver<ExecutorShardCommand<S>>,
    result_tx: Sender<Result<Vec<TransactionOutput>, VMStatus>>,
) -> thread::JoinHandle<()> {
    // create and start a new executor shard in a separate thread
    thread::Builder::new()
        .name(format!("executor-shard-{}", shard_id))
        .spawn(move || {
            let executor_shard = ExecutorShard::new(
                num_executor_shards,
                executor_client,
                shard_id,
                command_rx,
                result_tx,
            );
            executor_shard.start();
        })
        .unwrap()
}
