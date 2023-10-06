// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    remote_state_view::RemoteStateViewClient, ExecuteBlockCommand, RemoteExecutionRequest,
    RemoteExecutionResult,
};
use aptos_secure_net::network_controller::{Message, NetworkController};
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
use std::time::Instant;
use aptos_logger::info;
use crate::metrics::{APTOS_REMOTE_EXECUTOR_CMD_RX_BCS_DESERIALIZE_SECONDS, APTOS_REMOTE_EXECUTOR_CMD_RX_SECONDS, APTOS_REMOTE_EXECUTOR_INIT_PREFETCH_SECONDS};

//static mut init_prefetch_time: f64 = 0.0;
//static mut cmd_rx_bcs_deser_time: f64 = 0.0;

pub struct RemoteCoordinatorClient {
    state_view_client: Arc<RemoteStateViewClient>,
    command_rx: Receiver<Message>,
    result_tx: Sender<Message>,
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
        let result_tx =
            controller.create_outbound_channel(coordinator_address, execute_result_type);

        let state_view_client =
            RemoteStateViewClient::new(shard_id, controller, coordinator_address);

        unsafe {
            //cmd_rx_bcs_deser_time = 0.0;
            //init_prefetch_time = 0.0;
        }
        Self {
            state_view_client: Arc::new(state_view_client),
            command_rx,
            result_tx,
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
                    .par_iter()
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
                let _rx_timer = APTOS_REMOTE_EXECUTOR_CMD_RX_SECONDS.start_timer();
                let bcs_deser_timer = APTOS_REMOTE_EXECUTOR_CMD_RX_BCS_DESERIALIZE_SECONDS.start_timer();
                let request: RemoteExecutionRequest = bcs::from_bytes(&message.data).unwrap();
                bcs_deser_timer.stop_and_record();

                match request {
                    RemoteExecutionRequest::ExecuteBlock(command) => {
                        info!("&&&&&&&&&&&& Cmd rx ");
                        let init_prefetch_timer = APTOS_REMOTE_EXECUTOR_INIT_PREFETCH_SECONDS.start_timer();
                        let state_keys = Self::extract_state_keys(&command);
                        self.state_view_client.init_for_block(state_keys);
                        init_prefetch_timer.stop_and_record();
                        let (sub_blocks, concurrency, gas_limit) = command.into();
                        ExecutorShardCommand::ExecuteSubBlocks(
                            self.state_view_client.clone(),
                            sub_blocks,
                            concurrency,
                            gas_limit,
                        )
                    },
                }
            },
            Err(_) => {
                /*unsafe {
                    info!("&&&&&&&&&&&& Total cmd rx bcs deser time is {} ", cmd_rx_bcs_deser_time);
                    info!("&&&&&&&&&&&& Total init prefetch time is {} ", init_prefetch_time);
                    init_prefetch_time = 0.0;
                    cmd_rx_bcs_deser_time = 0.0;
                }*/
                info!("&&&&&&&&&&&& cmd rx time is {} s", APTOS_REMOTE_EXECUTOR_CMD_RX_SECONDS.get_sample_sum());
                info!("&&&&&&&&&&&& cmd rx bcs deser time is {} s", APTOS_REMOTE_EXECUTOR_CMD_RX_BCS_DESERIALIZE_SECONDS.get_sample_sum());
                info!("&&&&&&&&&&&& init prefetch time is {} s", APTOS_REMOTE_EXECUTOR_INIT_PREFETCH_SECONDS.get_sample_sum());
                self.state_view_client.print_info();
                ExecutorShardCommand::Stop
            },
        }
    }

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>) {
        let remote_execution_result = RemoteExecutionResult::new(result);
        let output_message = bcs::to_bytes(&remote_execution_result).unwrap();
        self.result_tx.send(Message::new(output_message)).unwrap();
    }
}
