// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{
    block_partitioner::{BlockPartitioner, UniformPartitioner},
    executor_shard::ExecutorShard,
};
use aptos_block_executor::errors::Error;
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::transaction::{Transaction, TransactionOutput};
use move_core_types::vm_status::VMStatus;
use std::{
    marker::PhantomData,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

mod block_partitioner;
mod executor_shard;

/// A wrapper around sharded block executors that manages multiple shards and aggregates the results.
pub struct ShardedBlockExecutor<S: StateView + Sync + Send + 'static> {
    num_executor_shards: usize,
    partitioner: Arc<dyn BlockPartitioner>,
    command_txs: Vec<Sender<ExecutorShardCommand>>,
    shard_threads: Vec<thread::JoinHandle<()>>,
    result_rxs: Vec<Receiver<Result<Vec<TransactionOutput>, Error<VMStatus>>>>,
    phantom: PhantomData<S>,
}

pub enum ExecutorShardCommand {
    ExecuteBlock(Vec<Transaction>),
    Stop,
}

impl<S: StateView + Sync + Send + 'static> ShardedBlockExecutor<S> {
    pub fn new(
        num_executor_shards: usize,
        num_threads_per_executor: Option<usize>,
        state_view: Arc<S>,
    ) -> Self {
        assert!(num_executor_shards > 0, "num_executor_shards must be > 0");
        let num_threads_per_executor = num_threads_per_executor.unwrap_or_else(|| {
            (num_cpus::get() as f64 / num_executor_shards as f64).ceil() as usize
        });
        let mut command_txs = vec![];
        let mut result_rxs = vec![];
        let mut shard_join_handles = vec![];
        for i in 0..num_executor_shards {
            let (transactions_tx, transactions_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            command_txs.push(transactions_tx);
            result_rxs.push(result_rx);
            shard_join_handles.push(spawn_executor_shard(
                i,
                num_threads_per_executor,
                state_view.clone(),
                transactions_rx,
                result_tx,
            ));
        }
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
        block: Vec<Transaction>,
    ) -> Result<Vec<TransactionOutput>, Error<VMStatus>> {
        let block_partitions = self.partitioner.partition(block, self.num_executor_shards);
        for (i, transactions) in block_partitions.into_iter().enumerate() {
            self.command_txs[i]
                .send(ExecutorShardCommand::ExecuteBlock(transactions))
                .unwrap();
        }
        // wait for all remote executors to send the result back and append them in order by shard id
        let mut aggregated_results = vec![];
        trace!("ShardedBlockExecutor Waiting for results");
        for i in 0..self.num_executor_shards {
            let result = self.result_rxs[i].recv().unwrap();
            aggregated_results.extend(result?);
        }
        Ok(aggregated_results)
    }
}

impl<S: StateView + Sync + Send + 'static> Drop for ShardedBlockExecutor<S> {
    fn drop(&mut self) {
        // send stop command to all executor shards
        for command_tx in self.command_txs.iter() {
            command_tx.send(ExecutorShardCommand::Stop).unwrap();
        }

        // wait for all executor shards to stop
        for shard_thread in self.shard_threads.drain(..) {
            shard_thread.join().unwrap();
        }
    }
}

fn spawn_executor_shard<S: StateView + Sync + Send + 'static>(
    shard_id: usize,
    concurrency_level: usize,
    state_view: Arc<S>,
    command_rx: Receiver<ExecutorShardCommand>,
    result_tx: Sender<Result<Vec<TransactionOutput>, Error<VMStatus>>>,
) -> thread::JoinHandle<()> {
    // create and start a new executor shard in a separate thread
    thread::Builder::new()
        .name(format!("executor-shard-{}", shard_id))
        .spawn(move || {
            let executor_shard = ExecutorShard::new(
                shard_id,
                concurrency_level,
                state_view.clone(),
                command_rx,
                result_tx,
            );
            executor_shard.start();
        })
        .unwrap()
}
