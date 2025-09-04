// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{
    coordinator_client::CoordinatorClient,
    counters::WAIT_FOR_SHARDED_OUTPUT_SECONDS,
    cross_shard_client::CrossShardClient,
    executor_client::{ExecutorClient, ShardedExecutionOutput},
    global_executor::GlobalExecutor,
    messages::CrossShardMsg,
    sharded_aggregator_service,
    sharded_executor_service::ShardedExecutorService,
    ExecutorShardCommand, ShardedBlockExecutor,
};
use velor_logger::trace;
use velor_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{
            PartitionedTransactions, RoundId, ShardId, GLOBAL_ROUND_ID,
            MAX_ALLOWED_PARTITIONING_ROUNDS,
        },
    },
    state_store::StateView,
    transaction::TransactionOutput,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use move_core_types::vm_status::VMStatus;
use std::{sync::Arc, thread};

/// Executor service that runs on local machine and waits for commands from the coordinator and executes
/// them in parallel.
pub struct LocalExecutorService<S: StateView + Sync + Send + 'static> {
    join_handle: Option<thread::JoinHandle<()>>,
    phantom: std::marker::PhantomData<S>,
}

impl<S: StateView + Sync + Send + 'static> LocalExecutorService<S> {
    fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        command_rx: Receiver<ExecutorShardCommand<S>>,
        result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
        cross_shard_client: LocalCrossShardClient,
    ) -> Self {
        let coordinator_client = Arc::new(LocalCoordinatorClient::new(command_rx, result_tx));
        let executor_service = Arc::new(ShardedExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            coordinator_client,
            Arc::new(cross_shard_client),
        ));
        let join_handle = thread::Builder::new()
            .name(format!("executor-shard-{}", shard_id))
            .spawn(move || executor_service.start())
            .unwrap();
        Self {
            join_handle: Some(join_handle),
            phantom: std::marker::PhantomData,
        }
    }

    fn setup_global_executor() -> (GlobalExecutor<S>, Sender<CrossShardMsg>) {
        let (cross_shard_tx, cross_shard_rx) = unbounded();
        let cross_shard_client = Arc::new(GlobalCrossShardClient::new(
            cross_shard_tx.clone(),
            cross_shard_rx,
        ));
        // Limit the number of global executor threads to 32 as parallel execution doesn't scale well beyond that.
        let executor_threads = num_cpus::get().min(32);
        let global_executor = GlobalExecutor::new(cross_shard_client, executor_threads);
        (global_executor, cross_shard_tx)
    }

    pub fn setup_local_executor_shards(
        num_shards: usize,
        num_threads: Option<usize>,
    ) -> LocalExecutorClient<S> {
        let (global_executor, global_cross_shard_tx) = Self::setup_global_executor();
        let num_threads = num_threads
            .unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);
        let (command_txs, command_rxs): (
            Vec<Sender<ExecutorShardCommand<S>>>,
            Vec<Receiver<ExecutorShardCommand<S>>>,
        ) = (0..num_shards).map(|_| unbounded()).unzip();
        let (result_txs, result_rxs): (
            Vec<Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
            Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
        ) = (0..num_shards).map(|_| unbounded()).unzip();
        // We need to create channels for each shard and each round. This is needed because individual
        // shards might send cross shard messages to other shards that will be consumed in different rounds.
        // Having a single channel per shard will cause a shard to receiver messages that is not intended in the current round.
        let (cross_shard_msg_txs, cross_shard_msg_rxs): (
            Vec<Vec<Sender<CrossShardMsg>>>,
            Vec<Vec<Receiver<CrossShardMsg>>>,
        ) = (0..num_shards)
            .map(|_| {
                (0..MAX_ALLOWED_PARTITIONING_ROUNDS)
                    .map(|_| unbounded())
                    .unzip()
            })
            .unzip();
        let executor_shards = command_rxs
            .into_iter()
            .zip(result_txs)
            .zip(cross_shard_msg_rxs)
            .enumerate()
            .map(|(shard_id, ((command_rx, result_tx), cross_shard_rxs))| {
                let cross_shard_client = LocalCrossShardClient::new(
                    global_cross_shard_tx.clone(),
                    cross_shard_msg_txs.clone(),
                    cross_shard_rxs,
                );
                Self::new(
                    shard_id as ShardId,
                    num_shards,
                    num_threads,
                    command_rx,
                    result_tx,
                    cross_shard_client,
                )
            })
            .collect();
        LocalExecutorClient::new(command_txs, result_rxs, executor_shards, global_executor)
    }
}

pub struct LocalExecutorClient<S: StateView + Sync + Send + 'static> {
    // Channels to send execute block commands to the executor shards.
    command_txs: Vec<Sender<ExecutorShardCommand<S>>>,
    // Channels to receive execution results from the executor shards.
    result_rxs: Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
    executor_services: Vec<LocalExecutorService<S>>,
    global_executor: GlobalExecutor<S>,
}

impl<S: StateView + Sync + Send + 'static> LocalExecutorClient<S> {
    pub fn new(
        command_tx: Vec<Sender<ExecutorShardCommand<S>>>,
        result_rx: Vec<Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>>,
        executor_shards: Vec<LocalExecutorService<S>>,
        global_executor: GlobalExecutor<S>,
    ) -> Self {
        Self {
            command_txs: command_tx,
            result_rxs: result_rx,
            executor_services: executor_shards,
            global_executor,
        }
    }

    pub fn create_local_sharded_block_executor(
        num_shards: usize,
        num_threads: Option<usize>,
    ) -> ShardedBlockExecutor<S, LocalExecutorClient<S>> {
        ShardedBlockExecutor::new(LocalExecutorService::setup_local_executor_shards(
            num_shards,
            num_threads,
        ))
    }

    fn get_output_from_shards(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus> {
        let _timer = WAIT_FOR_SHARDED_OUTPUT_SECONDS.start_timer();
        trace!("LocalExecutorClient Waiting for results");
        let mut results = vec![];
        for (i, rx) in self.result_rxs.iter().enumerate() {
            results.push(
                rx.recv()
                    .unwrap_or_else(|_| panic!("Did not receive output from shard {}", i))?,
            );
        }
        Ok(results)
    }
}

impl<S: StateView + Sync + Send + 'static> ExecutorClient<S> for LocalExecutorClient<S> {
    fn num_shards(&self) -> usize {
        self.command_txs.len()
    }

    fn execute_block(
        &self,
        state_view: Arc<S>,
        transactions: PartitionedTransactions,
        concurrency_level_per_shard: usize,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ShardedExecutionOutput, VMStatus> {
        assert_eq!(transactions.num_shards(), self.num_shards());
        let (sub_blocks, global_txns) = transactions.into();
        for (i, sub_blocks_for_shard) in sub_blocks.into_iter().enumerate() {
            self.command_txs[i]
                .send(ExecutorShardCommand::ExecuteSubBlocks(
                    state_view.clone(),
                    sub_blocks_for_shard,
                    concurrency_level_per_shard,
                    onchain_config.clone(),
                ))
                .unwrap();
        }

        // This means that we are executing the global transactions concurrently with the individual shards but the
        // global transactions will be blocked for cross shard transaction results. This hopefully will help with
        // finishing the global transactions faster but we need to evaluate if this causes thread contention. If it
        // does, then we can simply move this call to the end of the function.
        let mut global_output = self.global_executor.execute_global_txns(
            global_txns,
            state_view.as_ref(),
            onchain_config,
        )?;

        let mut sharded_output = self.get_output_from_shards()?;

        sharded_aggregator_service::aggregate_and_update_total_supply(
            &mut sharded_output,
            &mut global_output,
            state_view.as_ref(),
            self.global_executor.get_executor_thread_pool(),
        );

        Ok(ShardedExecutionOutput::new(sharded_output, global_output))
    }

    fn shutdown(&mut self) {}
}

impl<S: StateView + Sync + Send + 'static> Drop for LocalExecutorClient<S> {
    fn drop(&mut self) {
        for command_tx in self.command_txs.iter() {
            let _ = command_tx.send(ExecutorShardCommand::Stop);
        }

        // wait for join handles to finish
        for executor_service in self.executor_services.iter_mut() {
            let _ = executor_service.join_handle.take().unwrap().join();
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

impl<S: StateView + Sync + Send + 'static> CoordinatorClient<S> for LocalCoordinatorClient<S> {
    fn receive_execute_command(&self) -> ExecutorShardCommand<S> {
        self.command_rx.recv().unwrap()
    }

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>) {
        self.result_tx.send(result).unwrap()
    }
}

/// A cross shard client used by the global shard to receive cross-shard messages from other shards.
pub struct GlobalCrossShardClient {
    // Sender of global cross-shard message, used for sending Stop message to self.
    global_message_tx: Sender<CrossShardMsg>,
    // The receiver of cross-shard messages from other shards
    global_message_rx: Receiver<CrossShardMsg>,
}

impl GlobalCrossShardClient {
    pub fn new(message_tx: Sender<CrossShardMsg>, message_rx: Receiver<CrossShardMsg>) -> Self {
        Self {
            global_message_tx: message_tx,
            global_message_rx: message_rx,
        }
    }
}

impl CrossShardClient for GlobalCrossShardClient {
    fn send_global_msg(&self, msg: CrossShardMsg) {
        self.global_message_tx.send(msg).unwrap()
    }

    fn send_cross_shard_msg(&self, _shard_id: ShardId, _round: RoundId, _msg: CrossShardMsg) {
        unreachable!("Global shard client should not send cross-shard messages")
    }

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg {
        assert_eq!(
            current_round, GLOBAL_ROUND_ID,
            "Global shard client should only receive cross-shard messages in global round"
        );
        self.global_message_rx.recv().unwrap()
    }
}

pub struct LocalCrossShardClient {
    global_message_tx: Sender<CrossShardMsg>,
    // The senders of cross-shard messages to other shards per round.
    message_txs: Vec<Vec<Sender<CrossShardMsg>>>,
    // The receivers of cross shard messages from other shards per round.
    message_rxs: Vec<Receiver<CrossShardMsg>>,
}

impl LocalCrossShardClient {
    pub fn new(
        global_message_tx: Sender<CrossShardMsg>,
        cross_shard_txs: Vec<Vec<Sender<CrossShardMsg>>>,
        cross_shard_rxs: Vec<Receiver<CrossShardMsg>>,
    ) -> Self {
        Self {
            global_message_tx,
            message_txs: cross_shard_txs,
            message_rxs: cross_shard_rxs,
        }
    }
}

impl CrossShardClient for LocalCrossShardClient {
    fn send_global_msg(&self, msg: CrossShardMsg) {
        self.global_message_tx.send(msg).unwrap()
    }

    fn send_cross_shard_msg(&self, shard_id: ShardId, round: RoundId, msg: CrossShardMsg) {
        self.message_txs[shard_id][round].send(msg).unwrap()
    }

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg {
        self.message_rxs[current_round].recv().unwrap()
    }
}
