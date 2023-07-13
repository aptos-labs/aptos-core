// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::RemoteExecutionRequest;
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_state_view::in_memory_state_view::InMemoryStateView;
use aptos_types::{transaction::TransactionOutput, vm_status::VMStatus};
use aptos_vm::sharded_block_executor::{executor_shard::CoordinatorClient, ExecutorShardCommand};
use crossbeam_channel::{Receiver, SendError, Sender};
use std::{net::SocketAddr, sync::Arc};

pub struct RemoteCoordinatorClient {
    command_rx: Receiver<Message>,
    result_tx: Sender<Message>,
}

impl RemoteCoordinatorClient {
    pub fn new(controller: &mut NetworkController, coordinator_address: SocketAddr) -> Self {
        let command_rx = controller.create_inbound_channel("execute_command".to_string());
        let result_tx =
            controller.create_outbound_channel(coordinator_address, "execute_result".to_string());

        Self {
            command_rx,
            result_tx,
        }
    }
}

impl CoordinatorClient<InMemoryStateView> for RemoteCoordinatorClient {
    fn send_execute_command(
        &self,
        _execute_command: ExecutorShardCommand<InMemoryStateView>,
    ) -> Result<(), SendError<ExecutorShardCommand<InMemoryStateView>>> {
        unreachable!("RemoteCoordinatorClient should not send execute command")
    }

    fn get_execution_result(&self) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        unreachable!("RemoteCoordinatorClient should not get execution result")
    }

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
        let output_message = bcs::to_bytes(&result).unwrap();
        self.result_tx.send(Message::new(output_message)).unwrap();
    }
}
