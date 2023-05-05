// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{
    block_partitioner::{BlockPartitioner, UniformPartitioner},
    executor_shard::ExecutorShard,
};
use aptos_block_executor::errors::Error;
use aptos_state_view::StateView;
use aptos_types::transaction::{Transaction, TransactionOutput};
use move_core_types::vm_status::VMStatus;
use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

mod block_partitioner;
mod executor_shard;

/// A wrapper around sharded block executors that manages multiple shards and aggregates the results.
pub struct ShardedBlockExecutor<S: StateView + Sync + Send + 'static> {
    num_executor_shards: usize,
    partitioner: Arc<dyn BlockPartitioner>,
    transactions_txs: Vec<Sender<Vec<Transaction>>>,
    shard_threads: Vec<thread::JoinHandle<()>>,
    quit_signals: Vec<Arc<Mutex<bool>>>,
    result_rx: Receiver<(usize, Result<Vec<TransactionOutput>, Error<VMStatus>>)>,
    phantom: PhantomData<S>,
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
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let mut transactions_txs = vec![];
        let mut shard_join_handles = vec![];
        let mut quit_signals = vec![];
        for i in 0..num_executor_shards {
            let (transactions_tx, transactions_rx) = std::sync::mpsc::channel();
            let quit_signal = Arc::new(Mutex::new(false));
            transactions_txs.push(transactions_tx);
            shard_join_handles.push(spawn_executor_shard(
                i,
                num_threads_per_executor,
                quit_signal.clone(),
                state_view.clone(),
                transactions_rx,
                result_tx.clone(),
            ));
        }
        Self {
            num_executor_shards,
            partitioner: Arc::new(UniformPartitioner {}),
            transactions_txs,
            shard_threads: shard_join_handles,
            quit_signals,
            result_rx,
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
            self.transactions_txs[i].send(transactions).unwrap();
        }

        // wait for all remote executors to send the result back and append them in order by shard id
        // maintain a map of shard id to results and aggregate them later
        let mut results = HashMap::new();
        println!("Waiting for results");
        for _ in 0..self.num_executor_shards {
            let (shard_id, result) = self.result_rx.recv().unwrap();
            println!("Got result from shard {}", shard_id);
            if results.insert(shard_id, result?).is_some() {
                panic!("Duplicate shard id {} in results", shard_id)
            }
        }
        // aggregate the results
        let mut aggregated_results = vec![];
        for i in 0..self.num_executor_shards {
            aggregated_results.extend(results.get(&i).unwrap().clone());
        }
        Ok(aggregated_results)
    }
}

fn spawn_executor_shard<S: StateView + Sync + Send + 'static>(
    shard_id: usize,
    concurrency_level: usize,
    quit_signal: Arc<Mutex<bool>>,
    state_view: Arc<S>,
    transaction_rx: Receiver<Vec<Transaction>>,
    result_tx: Sender<(usize, Result<Vec<TransactionOutput>, Error<VMStatus>>)>,
) -> thread::JoinHandle<()> {
    // create and start a new executor shard in a separate thread
    thread::Builder::new()
        .name(format!("executor-shard-{}", shard_id))
        .spawn(move || {
            let executor_shard = ExecutorShard::new(
                shard_id,
                concurrency_level,
                quit_signal,
                state_view.clone(),
                transaction_rx,
                result_tx,
            );
            executor_shard.start();
        })
        .unwrap()
}
