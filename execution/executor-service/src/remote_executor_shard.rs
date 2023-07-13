// Copyright © Aptos Foundation

use crate::{ExecuteBlockCommand, RemoteExecutionRequest, RemoteExecutionResult};
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::ShardId, transaction::TransactionOutput, vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::{executor_shard::ExecutorShard, ExecutorShardCommand};
use crossbeam_channel::{Receiver, Sender};
use std::net::SocketAddr;

/// A block executor that receives transactions from a channel and executes them in parallel.
/// It runs in the local machine.
#[allow(dead_code)]
pub struct RemoteExecutorShard<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    command_tx: Sender<Message>,
    result_rx: Receiver<Message>,
    phantom: std::marker::PhantomData<S>,
}

#[allow(dead_code)]
impl<S: StateView + Sync + Send + 'static> RemoteExecutorShard<S> {
    pub fn _new(
        shard_id: ShardId,
        remote_shard_addr: SocketAddr,
        controller: &mut NetworkController,
    ) -> Self {
        let command_tx = controller
            .create_outbound_channel(remote_shard_addr, "executor-shard-command".to_string());
        let result_rx = controller.create_inbound_channel("executor-shard-result".to_string());
        Self {
            shard_id,
            command_tx,
            result_rx,
            phantom: std::marker::PhantomData,
        }
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
        let received_bytes = self.result_rx.recv().unwrap().to_bytes();
        let result: RemoteExecutionResult = bcs::from_bytes(&received_bytes).unwrap();
        result.inner
    }
}
