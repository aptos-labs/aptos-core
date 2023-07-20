use std::collections::HashMap;
// Copyright © Aptos Foundation
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    thread_executor_service::ThreadExecutorService, ExecuteBlockCommand, RemoteExecutionRequest,
    RemoteExecutionResult,
};
use aptos_config::utils;
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_state_view::{StateView, TStateView};
use aptos_types::{
    transaction::TransactionOutput, vm_status::VMStatus,
};
use crossbeam_channel::{Receiver, Sender};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use aptos_logger::trace;
use aptos_state_view::in_memory_state_view::InMemoryStateView;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::block_executor::partitioner::SubBlocksForShard;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_vm::sharded_block_executor::executor_shard::ExecutorClient;

pub struct RemoteExecutorClient<S: StateView + Sync + Send + 'static> {
    // Channels to send execute block commands to the executor shards.
    command_txs: Arc<Vec<Mutex<Sender<Message>>>>,
    // Channels to receive execution results from the executor shards.
    result_rxs: Vec<Receiver<Message>>,
    // Thread pool used to pre-fetch the state values for the block in parallel and create an in-memory state view.
    thread_pool: Arc<rayon::ThreadPool>,
    phantom: std::marker::PhantomData<S>,
}

impl<S: StateView + Sync + Send + 'static> RemoteExecutorClient<S> {
    pub fn new(
        remote_shard_addresses: Vec<SocketAddr>,
        controller: &mut NetworkController,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = num_threads.unwrap_or_else(|| num_cpus::get());
        let thread_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap());
        let (command_txs, result_rxs) = remote_shard_addresses
            .iter().enumerate()
            .map(|(shard_id, address)| {
                let execute_command_type = format!("execute_command_{}", shard_id);
                let execute_result_type = format!("execute_result_{}", shard_id);
                let command_tx = Mutex::new(controller
                    .create_outbound_channel(*address, execute_command_type.to_string()));
                let result_rx = controller.create_inbound_channel(execute_result_type);
                (command_tx, result_rx)
            })
            .unzip();
        Self {
            command_txs: Arc::new(command_txs),
            result_rxs,
            thread_pool,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn create_thread_remote_executor_shards(
        num_shards: usize,
        num_threads: Option<usize>,
    ) -> (
        NetworkController,
        RemoteExecutorClient<S>,
        Vec<ThreadExecutorService>,
    ) {
        // First create the coordinator.
        let listen_port = utils::get_available_port();
        let coordinator_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port);
        let mut controller =
            NetworkController::new("remote-executor-coordinator".to_string(), coordinator_address, 5000);
        let remote_shard_addresses = (0..num_shards)
            .map(|_| {
                let listen_port = utils::get_available_port();
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port)
            })
            .collect::<Vec<_>>();

        let num_threads = num_threads
            .unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);

        let remote_executor_services = (0..num_shards)
            .map(|shard_id| {
                ThreadExecutorService::new(
                    shard_id,
                    num_shards,
                    num_threads,
                    coordinator_address,
                    remote_shard_addresses.clone(),
                )
            })
            .collect::<Vec<_>>();


        let remote_executor_client = RemoteExecutorClient::new(
            remote_shard_addresses,
            &mut controller,
            None,
        );
        (
            controller,
            remote_executor_client,
            remote_executor_services,
        )
    }

}

impl<S: StateView + Sync + Send + 'static> ExecutorClient<S> for RemoteExecutorClient<S> {
    fn num_shards(&self) -> usize {
        self.command_txs.len()
    }

    fn execute_block(&self, state_view: Arc<S>, block: Vec<SubBlocksForShard<AnalyzedTransaction>>, concurrency_level_per_shard: usize, maybe_block_gas_limit: Option<u64>) {
        self.thread_pool.scope(|s| {
            for (shard_id, sub_blocks) in block.into_iter().enumerate() {
                let state_view = state_view.clone();
                let senders = self.command_txs.clone();
                let _ = s.spawn(move |_| {
                    let mut in_memory_state_date = HashMap::new();
                    for txn in sub_blocks.iter() {
                        for storage_location in txn.txn.read_hints().iter().chain(txn.txn.write_hints().iter()) {
                            let state_key = storage_location.state_key();
                            if !in_memory_state_date.contains_key(state_key) {
                                let state_value = state_view.get_state_value(state_key).unwrap().unwrap();
                                in_memory_state_date.insert(state_key.clone(), state_value);
                            }
                        }
                    }
                    //let in_memory_state_view = InMemoryStateView::new(in_memory_state_date);
                    let execution_request = RemoteExecutionRequest::ExecuteBlock(ExecuteBlockCommand {
                        sub_blocks,
                        state_view: state_view.as_in_memory_state_view(),
                        concurrency_level: concurrency_level_per_shard,
                        maybe_block_gas_limit,
                    });

                    senders[shard_id].lock().unwrap().send(Message::new(bcs::to_bytes(&execution_request).unwrap())).unwrap()

                });
            }
        });
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus> {
        trace!("RemoteExecutorClient Waiting for results");
        let mut results = vec![];
        for rx in self.result_rxs.iter() {
            let received_bytes = rx.recv().unwrap().to_bytes();
            let result: RemoteExecutionResult = bcs::from_bytes(&received_bytes).unwrap();
            results.push(result.inner?);
        }
        Ok(results)
    }
}
