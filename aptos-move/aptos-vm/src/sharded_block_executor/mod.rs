// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::executor_shard::ExecutorShard;
use aptos_block_partitioner::{BlockPartitioner, UniformPartitioner};
use aptos_logger::{error, info, trace};
use aptos_state_view::StateView;
use aptos_types::transaction::{
    analyzed_transaction::AnalyzedTransaction, Transaction, TransactionOutput,
};
use move_core_types::vm_status::VMStatus;
use std::{
    marker::PhantomData,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

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

pub struct SubBlockOutput {
    sub_block_id: usize,
}

pub enum ExecutorShardCommand<S: StateView + Sync + Send + 'static> {
    ExecuteBlock(Arc<S>, Vec<Transaction>, usize),
    ExeDone(Result<Vec<TransactionOutput>, VMStatus>),
    SubBlockFinished(SubBlockOutput),
    Stop,
}

impl<S: StateView + Sync + Send + 'static> ShardedBlockExecutor<S> {
    pub fn new(
        num_executor_shards: usize,
        executor_threads_per_shard: Option<usize>,
        maybe_gas_limit: Option<u64>,
    ) -> Self {
        assert!(num_executor_shards > 0, "num_executor_shards must be > 0");
        let executor_threads_per_shard = executor_threads_per_shard.unwrap_or_else(|| {
            (num_cpus::get() as f64 / num_executor_shards as f64).ceil() as usize
        });
        let mut to_shard_txs = vec![];
        let mut to_shard_rxs = vec![];
        let mut to_master_txs = vec![];
        let mut to_master_rxs = vec![];

        for _i in 0..num_executor_shards {
            let (to_shard_tx, to_shard_rx) = std::sync::mpsc::channel();
            let (from_shard_tx, from_shard_rx) = std::sync::mpsc::channel();
            to_shard_txs.push(to_shard_tx);
            to_shard_rxs.push(to_shard_rx);
            to_master_txs.push(from_shard_tx);
            to_master_rxs.push(from_shard_rx);
        }

        let mut shard_join_handles = vec![];
        for (shard_id, ((to_shard_rx, to_master_tx), to_shard_tx)) in to_shard_rxs.into_iter().zip(to_master_txs.into_iter()).zip(to_shard_txs.iter()).enumerate() {
            shard_join_handles.push(spawn_executor_shard(
                num_executor_shards,
                shard_id,
                executor_threads_per_shard,
                to_shard_tx.clone(),
                to_shard_rx,
                to_master_tx,
                to_shard_txs.clone(),
                maybe_gas_limit,
            ));
        }

        info!(
            "Creating a new ShardedBlockExecutor with {} shards and concurrency per shard {}",
            num_executor_shards, executor_threads_per_shard
        );
        Self {
            num_executor_shards,
            partitioner: Arc::new(UniformPartitioner {}),
            command_txs: to_shard_txs,
            shard_threads: shard_join_handles,
            result_rxs: to_master_rxs,
            phantom: PhantomData,
        }
    }

    /// Execute a block of transactions in parallel by splitting the block into num_remote_executors partitions and
    /// dispatching each partition to a remote executor shard.
    pub fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<AnalyzedTransaction>,
        concurrency_level_per_shard: usize,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let block_partitions = self.partitioner.partition(block, self.num_executor_shards);
        // Number of partitions might be smaller than the number of executor shards in case of
        // block size is smaller than number of executor shards.
        let num_partitions = block_partitions.len();
        for (i, transactions) in block_partitions.into_iter().enumerate() {
            self.command_txs[i]
                .send(ExecutorShardCommand::ExecuteBlock(
                    state_view.clone(),
                    transactions
                        .into_iter()
                        .map(|t| t.into())
                        .collect::<Vec<Transaction>>(),
                    concurrency_level_per_shard,
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

fn spawn_executor_shard<S: StateView + Sync + Send + 'static>(
    num_executor_shards: usize,
    shard_id: usize,
    concurrency_level: usize,
    command_tx: Sender<ExecutorShardCommand<S>>,
    command_rx: Receiver<ExecutorShardCommand<S>>,
    result_tx: Sender<Result<Vec<TransactionOutput>, VMStatus>>,
    peer_tx_vec: Vec<Sender<ExecutorShardCommand<S>>>,
    maybe_gas_limit: Option<u64>,
) -> thread::JoinHandle<()> {
    // create and start a new executor shard in a separate thread
    thread::Builder::new()
        .name(format!("executor-shard-{}", shard_id))
        .spawn(move || {
            let mut executor_shard = ExecutorShard::new(
                num_executor_shards,
                shard_id,
                concurrency_level,
                command_tx,
                command_rx,
                result_tx,
                peer_tx_vec,
                maybe_gas_limit,
            );
            executor_shard.start();
        })
        .unwrap()
}
