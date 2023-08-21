// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{RemoteExecutionRequest, RemoteExecutionResult};
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_state_view::in_memory_state_view::InMemoryStateView;
use aptos_types::{
    block_executor::partitioner::ShardId, transaction::TransactionOutput, vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::{
    coordinator_client::CoordinatorClient, ExecutorShardCommand,
};
use crossbeam_channel::{Receiver, Sender};
use std::{net::SocketAddr, sync::Arc};

pub struct RemoteCoordinatorClient {
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

        Self {
            command_rx,
            result_tx,
        }
    }
}

impl CoordinatorClient<InMemoryStateView> for RemoteCoordinatorClient {
    fn receive_execute_command(&self) -> ExecutorShardCommand<InMemoryStateView> {
        let message = self.command_rx.recv().unwrap();
        let request: RemoteExecutionRequest = bcs::from_bytes(&message.data).unwrap();
        match request {
            RemoteExecutionRequest::ExecuteBlock(command) => {
                let (sub_blocks, state_view, concurrency, gas_limit) = command.into();
                ExecutorShardCommand::ExecuteSubBlocks(
                    Arc::new(state_view),
                    sub_blocks,
                    concurrency,
                    gas_limit,
                )
            },
        }
    }

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>) {
        let remote_execution_result = RemoteExecutionResult::new(result);
        let output_message = bcs::to_bytes(&remote_execution_result).unwrap();
        self.result_tx.send(Message::new(output_message)).unwrap();
    }
}
