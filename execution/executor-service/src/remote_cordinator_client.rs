// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    metrics::REMOTE_EXECUTOR_TIMER, remote_state_view::RemoteStateViewClient, ExecuteBlockCommand,
    RemoteExecutionRequest, RemoteExecutionResult,
};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
use aptos_types::{
    block_executor::partitioner::ShardId, state_store::state_key::StateKey,
    transaction::TransactionOutput, vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::{
    coordinator_client::CoordinatorClient, ExecutorShardCommand,
};
use crossbeam_channel::{Receiver, Sender};
use rayon::prelude::*;
use std::{net::SocketAddr, sync::Arc};
use std::sync::atomic::AtomicU64;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::{get_delta_time, REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER};

pub struct RemoteCoordinatorClient {
    state_view_client: Arc<RemoteStateViewClient>,
    command_rx: Receiver<Message>,
    //result_tx: Sender<Message>,
    result_tx: OutboundRpcHelper,
    shard_id: ShardId,
    cmd_rx_msg_duration_since_epoch: AtomicU64,
}

impl RemoteCoordinatorClient {
    pub fn new(
        shard_id: ShardId,
        controller: &mut NetworkController,
        coordinator_address: SocketAddr,
    ) -> Self {
        let execute_command_type = format!("execute_command_{}", shard_id);
        let execute_result_type = format!("execute_result_{}", shard_id);
        let command_rx = controller.create_inbound_channel(execute_command_type);
        let result_tx = OutboundRpcHelper::new(controller.get_self_addr(), coordinator_address);
            //controller.create_outbound_channel(coordinator_address, execute_result_type);

        let state_view_client =
            RemoteStateViewClient::new(shard_id, controller, coordinator_address);

        Self {
            state_view_client: Arc::new(state_view_client),
            command_rx,
            result_tx,
            shard_id,
            cmd_rx_msg_duration_since_epoch: AtomicU64::new(0),
        }
    }

    // Extract all the state keys from the execute block command. It is possible that there are duplicate state keys.
    // We are not de-duplicating them here to avoid the overhead of deduplication. The state view server will deduplicate
    // the state keys.
    fn extract_state_keys(command: &ExecuteBlockCommand) -> Vec<StateKey> {
        command
            .sub_blocks
            .sub_block_iter()
            .flat_map(|sub_block| {
                sub_block
                    .transactions
                    .iter()
                    .map(|txn| {
                        let mut state_keys = vec![];
                        for storage_location in txn
                            .txn()
                            .read_hints()
                            .iter()
                            .chain(txn.txn().write_hints().iter())
                        {
                            state_keys.push(storage_location.state_key().clone());
                        }
                        state_keys
                    })
                    .flatten()
                    .collect::<Vec<StateKey>>()
            })
            .collect::<Vec<StateKey>>()
    }
}

impl CoordinatorClient<RemoteStateViewClient> for RemoteCoordinatorClient {
    fn receive_execute_command(&self) -> ExecutorShardCommand<RemoteStateViewClient> {
        match self.command_rx.recv() {
            Ok(message) => {
                let delta = get_delta_time(message.start_ms_since_epoch.unwrap());
                REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
                    .with_label_values(&["5_cmd_tx_msg_shard_recv"]).observe(delta as f64);
                self.cmd_rx_msg_duration_since_epoch.store(message.start_ms_since_epoch.unwrap(), std::sync::atomic::Ordering::Relaxed);
                let _rx_timer = REMOTE_EXECUTOR_TIMER
                    .with_label_values(&[&self.shard_id.to_string(), "cmd_rx"])
                    .start_timer();
                let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
                    .with_label_values(&[&self.shard_id.to_string(), "cmd_rx_bcs_deser"])
                    .start_timer();
                let request: RemoteExecutionRequest = bcs::from_bytes(&message.data).unwrap();
                drop(bcs_deser_timer);

                match request {
                    RemoteExecutionRequest::ExecuteBlock(command) => {
                        let init_prefetch_timer = REMOTE_EXECUTOR_TIMER
                            .with_label_values(&[&self.shard_id.to_string(), "init_prefetch"])
                            .start_timer();
                        let state_keys = Self::extract_state_keys(&command);
                        self.state_view_client.init_for_block(state_keys);
                        drop(init_prefetch_timer);

                        let (sub_blocks, concurrency, onchain_config) = command.into();
                        ExecutorShardCommand::ExecuteSubBlocks(
                            self.state_view_client.clone(),
                            sub_blocks,
                            concurrency,
                            onchain_config,
                        )
                    },
                }
            },
            Err(_) => ExecutorShardCommand::Stop,
        }
    }

    fn send_execution_result(&mut self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>) {
        let execute_result_type = format!("execute_result_{}", self.shard_id);
        let duration_since_epoch = self.cmd_rx_msg_duration_since_epoch.load(std::sync::atomic::Ordering::Relaxed);
        let delta = get_delta_time(duration_since_epoch);
        REMOTE_EXECUTOR_CMD_RESULTS_RND_TRP_JRNY_TIMER
            .with_label_values(&["6_results_tx_msg_shard_send"]).observe(delta as f64);
        let remote_execution_result = RemoteExecutionResult::new(result);
        let output_message = bcs::to_bytes(&remote_execution_result).unwrap();
        self.result_tx.send(Message::create_with_metadata(output_message, duration_since_epoch, 0, 0), &MessageType::new(execute_result_type));
    }
}
