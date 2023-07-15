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
use aptos_types::block_executor::partitioner::SubBlocksForShard;
use aptos_types::transaction::Transaction;
use crate::sharded_block_executor::executor_shard::CoordinatorToExecutorShardClient;

/// A block executor that receives transactions from a channel and executes them in parallel.
/// It runs in the local machine.
pub struct LocalExecutorShard<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    executor_service: Arc<ShardedExecutorService<S>>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl<S: StateView + Sync + Send + 'static> LocalExecutorShard<S> {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        cross_shard_txs: Vec<Vec<Sender<CrossShardMsg>>>,
        cross_shard_rxs: Vec<Receiver<CrossShardMsg>>,
    ) -> Self {
        let coordinator_client = Arc::new(LocalCoordinatorClient1::new());
        let cross_shard_client =
            Arc::new(LocalCrossShardClient::new(shard_id, cross_shard_txs, cross_shard_rxs));

        let executor_service = Arc::new(ShardedExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            coordinator_client,
            cross_shard_client,
        ));
        Self {
            shard_id,
            executor_service,
            join_handle: None,
        }
    }

    pub fn create_local_executor_shards(
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
                let (messages_tx, messages_rx) = unbounded();
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
                    num_shards,
                    num_threads,
                    cross_shard_msg_txs.clone(),
                    rxs,
                )
            })
            .collect()
    }
}

impl<S: StateView + Sync + Send + 'static> ExecutorShard<S> for LocalExecutorShard<S> {
    fn start(&mut self) {
        let executor_service = Arc::clone(&self.executor_service);
        let join_handle = thread::Builder::new()
            .name(format!("executor-shard-{}", self.shard_id))
            .spawn(move || executor_service.start())
            .unwrap();
        self.join_handle = Some(join_handle);
    }

    fn stop(&mut self) {
        self.executor_service.stop();
        if let Some(executor_shard_thread) = self.join_handle.take() {
            executor_shard_thread.join().unwrap_or_else(|e| {
                error!("Failed to join executor shard thread: {:?}", e);
            });
        }
    }

    fn send_execute_command(&self, execute_command: ExecutorShardCommand<S>) {
        self.executor_service.send_execute_command(execute_command);
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        self.executor_service.get_execution_result()
    }
}

pub struct LocalCoordinatorClient<S> {
    // Channels to send execute block commands to the executor shards.
    command_tx: Vec<Sender<ExecutorShardCommand<S>>>,
    // Channels to receive execution results from the executor shards.
    result_rx: Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
}

impl<S> LocalCoordinatorClient<S> {
    pub fn new(
        command_tx: Vec<Sender<ExecutorShardCommand<S>>>,
        result_rx: Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
    ) -> Self {
        Self {
            command_tx,
            result_rx,
        }
    }
}

impl<S: StateView + Sync + Send + 'static> CoordinatorToExecutorShardClient<S> for LocalCoordinatorClient<S> {
    fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<SubBlocksForShard<Transaction>>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>) {
        for (i, sub_blocks_for_shard) in block.into_iter().enumerate() {
            self.command_tx[i].send_execute_command(ExecutorShardCommand::ExecuteSubBlocks(
                state_view.clone(),
                sub_blocks_for_shard,
                concurrency_level_per_shard,
                maybe_block_gas_limit,
            ))
        }
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus> {
        trace!("ShardedBlockExecutor Waiting for results");
        self.result_rx.iter().enumerate().map(|(shard, rx)| {
            trace!("ShardedBlockExecutor Waiting for result from shard {}", shard);
            rx.recv().unwrap()?
        }).collect()
    }
}



pub struct LocalCoordinatorClient1<S> {
    // Channel to receive execute block commands from the coordinator.
    command_tx: Sender<ExecutorShardCommand<S>>,
    command_rx: Receiver<ExecutorShardCommand<S>>,
    // Channel to send execution results to the coordinator.
    result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
    result_rx: Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
}

impl<S> LocalCoordinatorClient1<S> {
    pub fn new() -> Self {
        let (command_tx, command_rx) = unbounded();
        let (result_tx, result_rx) = unbounded();
        Self {
            command_tx,
            command_rx,
            result_tx,
            result_rx,
        }
    }
}

impl<S> Default for LocalCoordinatorClient1<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: StateView + Sync + Send + 'static> CoordinatorClient<S> for LocalCoordinatorClient1<S> {
    fn send_execute_command(
        &self,
        execute_command: ExecutorShardCommand<S>,
    ) -> Result<(), SendError<ExecutorShardCommand<S>>> {
        self.command_tx.send(execute_command)
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        self.result_rx.recv().unwrap()
    }

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
