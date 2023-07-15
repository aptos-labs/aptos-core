// Copyright © Aptos Foundation

use crate::sharded_block_executor::{
    executor_shard::{CoordinatorClient, CrossShardClient, ExecutorShard},
    messages::CrossShardMsg,
    sharded_executor_service::ShardedExecutorService,
    ExecutorShardCommand,
};
use aptos_block_partitioner::sharded_block_partitioner::MAX_ALLOWED_PARTITIONING_ROUNDS;
use aptos_logger::{error, trace};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId},
    transaction::TransactionOutput,
};
use crossbeam_channel::{unbounded, Receiver, SendError, Sender};
use move_core_types::vm_status::VMStatus;
use std::{
    sync::{Arc, Mutex},
    thread,
};
use futures::StreamExt;
use aptos_types::block_executor::partitioner::SubBlocksForShard;
use aptos_types::transaction::Transaction;
use crate::sharded_block_executor::executor_shard::{CoordinatorToExecutorClient, ExecutorToCoordinatorClient};

/// A block executor that receives transactions from a channel and executes them in parallel.
/// It runs in the local machine.
pub struct LocalExecutorShard<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    executor_service: Arc<ShardedExecutorService<S>>,
    join_handle: thread::JoinHandle<()>,
}

impl<S: StateView + Sync + Send + 'static> LocalExecutorShard<S> {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        command_rx: Receiver<ExecutorShardCommand<S>>,
        result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
        cross_shard_txs: Vec<Vec<Sender<CrossShardMsg>>>,
        cross_shard_rxs: Vec<Receiver<CrossShardMsg>>,
    ) -> Self {
        let cross_shard_client =
            Arc::new(LocalCrossShardClient::new(shard_id, cross_shard_txs, cross_shard_rxs));
        let coordinator_client = Arc::new(LocalCoordinatorClient::new(command_rx, result_tx));
        let executor_service = Arc::new(ShardedExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            coordinator_client,
            cross_shard_client,
        ));
        let executor_service_clone = Arc::clone(&executor_service);
        let join_handle = thread::Builder::new()
            .name(format!("executor-shard-{}", shard_id))
            .spawn(move || executor_service_clone.start())
            .unwrap();
        Self {
            shard_id,
            executor_service,
            join_handle,
        }
    }

    pub fn setup_local_executor_shards(
        num_shards: usize,
        num_threads: Option<usize>,
    ) -> (LocalExecutorClient<S>, Vec<Self>) {
        let num_threads = num_threads
            .unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);
        let mut cross_shard_msg_txs = vec![];
        let mut cross_shard_msg_rxs = vec![];
        let mut command_txs = vec![];
        let mut command_rxs = vec![];
        let mut result_txs = vec![];
        let mut result_rxs = vec![];
        for _ in 0..num_shards {
            let (command_tx, command_rx) = unbounded();
            let (result_tx, result_rx) = unbounded();
            command_rxs.push(command_rx);
            result_txs.push(result_tx);
            command_txs.push(command_tx);
            result_rxs.push(result_rx);
            let mut current_shard_msg_txs = vec![];
            let mut current_shard_msg_rxs = vec![];
            for _ in 0..MAX_ALLOWED_PARTITIONING_ROUNDS {
                let (messages_tx, messages_rx) = unbounded();
                current_shard_msg_txs.push(messages_tx);
                current_shard_msg_rxs.push(messages_rx);
            }
            cross_shard_msg_txs.push(current_shard_msg_txs);
            cross_shard_msg_rxs.push(current_shard_msg_rxs);
        }
        let executor_shards = command_rxs.into_iter().zip(result_txs.into_iter()).zip(cross_shard_msg_rxs.into_iter())
            .enumerate()
            .map(|(shard_id, ((command_rx, result_tx), cross_shard_rxs))| {
                Self::new(
                    shard_id as ShardId,
                    num_shards,
                    num_threads,
                    command_rx,
                    result_tx,
                    cross_shard_msg_txs.clone(),
                    cross_shard_rxs,
                )
            })
            .collect();
        let coordinator_client = LocalExecutorClient::new(command_txs, result_rxs);
        (coordinator_client, executor_shards)
    }
}

pub struct LocalExecutorClient<S: StateView + Sync + Send + 'static> {
    // Channels to send execute block commands to the executor shards.
    command_txs: Vec<Sender<ExecutorShardCommand<S>>>,
    // Channels to receive execution results from the executor shards.
    result_rxs: Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
}

impl<S: StateView + Sync + Send + 'static> LocalExecutorClient<S> {
    pub fn new(
        command_tx: Vec<Sender<ExecutorShardCommand<S>>>,
        result_rx: Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
    ) -> Self {
        Self {
            command_txs: command_tx,
            result_rxs: result_rx,
        }
    }
}

impl<S: StateView + Sync + Send + 'static> CoordinatorToExecutorClient<S> for LocalExecutorClient<S> {
    fn num_shards(&self) -> usize {
        self.command_txs.len()
    }

    fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<SubBlocksForShard<Transaction>>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>) {
        for (i, sub_blocks_for_shard) in block.into_iter().enumerate() {
            self.command_txs[i].send(ExecutorShardCommand::ExecuteSubBlocks(
                state_view.clone(),
                sub_blocks_for_shard,
                concurrency_level_per_shard,
                maybe_block_gas_limit,
            )).unwrap();
        }
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus> {
        trace!("ShardedBlockExecutor Waiting for results");
        let mut results = vec![];
        for rx in self.result_rxs.iter() {
            results.push(rx.recv().unwrap()?);
        }
        Ok(results)
    }
}

impl<S: StateView + Sync + Send + 'static> Drop for LocalExecutorClient<S> {
    fn drop(&mut self) {
        for command_tx in self.command_txs.iter() {
            let _ =  command_tx.send(ExecutorShardCommand::Stop);
        }
    }
}


pub struct LocalCoordinatorClient<S> {
    command_rx: Receiver<ExecutorShardCommand<S>>,
    // Channel to send execution results to the coordinator.
    result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
}

impl<S> LocalCoordinatorClient<S> {
    pub fn new(
        command_rx: Receiver<ExecutorShardCommand<S>>,
        result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
    ) -> Self {
        Self {
            command_rx,
            result_tx,
        }
    }
}

impl<S: StateView + Sync + Send + 'static> ExecutorToCoordinatorClient<S> for LocalCoordinatorClient<S> {
    fn receive_execute_command(&self) -> ExecutorShardCommand<S> {
        self.command_rx.recv().unwrap()
    }

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>) {
        self.result_tx.send(result).unwrap()
    }
}

pub struct LocalCrossShardClient {
    shard_id: ShardId,
    // The senders of cross-shard messages to other shards per round.
    message_txs: Arc<Vec<Vec<Mutex<Sender<CrossShardMsg>>>>>,
    // The receivers of cross shard messages from other shards per round.
    message_rxs: Arc<Vec<Mutex<Receiver<CrossShardMsg>>>>,
}

impl LocalCrossShardClient {
    pub fn new(
        shard_id: ShardId,
        cross_shard_txs: Vec<Vec<Sender<CrossShardMsg>>>,
        cross_shard_rxs: Vec<Receiver<CrossShardMsg>>,
    ) -> Self {
        Self {
            shard_id,
            message_rxs: Arc::new(cross_shard_rxs.into_iter().map(Mutex::new).collect()),
            message_txs: Arc::new(
                cross_shard_txs
                    .into_iter()
                    .map(|inner_vec| inner_vec.into_iter().map(Mutex::new).collect())
                    .collect(),
            ),
        }
    }
}

impl CrossShardClient for LocalCrossShardClient {
    fn send_cross_shard_msg(&self, shard_id: ShardId, round: RoundId, msg: CrossShardMsg) {
        self.message_txs[shard_id][round]
            .lock()
            .unwrap()
            .send(msg)
            .unwrap()
    }

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg {
        self.message_rxs[current_round]
            .lock()
            .unwrap()
            .recv()
            .unwrap()
    }
}
