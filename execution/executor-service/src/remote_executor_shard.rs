// Copyright © Aptos Foundation
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    thread_executor_service::ThreadExecutorService, ExecuteBlockCommand, RemoteExecutionRequest,
    RemoteExecutionResult,
};
use aptos_config::utils;
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_state_view::StateView;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    block_executor::partitioner::ShardId, transaction::TransactionOutput, vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::{executor_shard::ExecutorShard, ExecutorShardCommand};
use crossbeam_channel::{Receiver, Sender};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// A block executor that receives transactions from a channel and executes them in parallel.
/// It runs in the local machine.
pub struct RemoteExecutorShard<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    command_tx: Sender<Message>,
    result_rx: Receiver<Message>,
    phantom: std::marker::PhantomData<S>,
}

impl<S: StateView + Sync + Send + 'static> RemoteExecutorShard<S> {
    pub fn new(
        shard_id: ShardId,
        remote_shard_addr: SocketAddr,
        controller: &mut NetworkController,
    ) -> Self {
        let execute_command_type = format!("execute_command_{}", shard_id);
        let execute_result_type = format!("execute_result_{}", shard_id);

        let command_tx = controller
            .create_outbound_channel(remote_shard_addr, execute_command_type.to_string());
        let result_rx = controller.create_inbound_channel(execute_result_type.to_string());
        Self {
            shard_id,
            command_tx,
            result_rx,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn create_thread_remote_executor_shards(
        num_shards: usize,
        num_threads: Option<usize>,
    ) -> (
        NetworkController,
        Vec<Self>,
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


        // Now create the remote shards.
        let remote_shards = remote_shard_addresses
            .iter()
            .enumerate()
            .map(|(shard_id, address)| {
                RemoteExecutorShard::new(shard_id as ShardId, *address, &mut controller)
            })
            .collect::<Vec<_>>();
        (
            controller,
            remote_shards,
            remote_executor_services,
        )
    }
}

impl<S: StateView + Sync + Send + 'static> ExecutorShard<S> for RemoteExecutorShard<S> {
    fn start(&mut self) {
        // do nothing, assumption is that the remote process is already started at this point
    }

    fn stop(&mut self) {
        // No-op
    }

    fn send_execute_command(&self, execute_command: ExecutorShardCommand<S>) {
        match execute_command {
            ExecutorShardCommand::ExecuteSubBlocks(
                state_view,
                sub_blocks,
                concurrency,
                gas_limit,
            ) => {
                let execution_request = RemoteExecutionRequest::ExecuteBlock(ExecuteBlockCommand {
                    sub_blocks,
                    // TODO: Avoid serializing this for each shard and serialize it once in the coordinator
                    state_view: S::as_in_memory_state_view(state_view.as_ref()),
                    concurrency_level: concurrency,
                    maybe_block_gas_limit: gas_limit,
                });
                self.command_tx
                    .send(Message::new(bcs::to_bytes(&execution_request).unwrap()))
                    .unwrap();
            },
            ExecutorShardCommand::Stop => {
                // Do nothing
            },
        }
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        println!("Waiting for result for shard {}", self.shard_id);
        let received_bytes = self.result_rx.recv().unwrap().to_bytes();
        println!("Received result for shard {}", self.shard_id);
        let result: RemoteExecutionResult = bcs::from_bytes(&received_bytes).unwrap();
        result.inner
    }
}
