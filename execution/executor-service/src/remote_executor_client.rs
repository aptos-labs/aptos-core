// Copyright © Aptos Foundation
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ExecuteBlockCommand, RemoteExecutionRequest, RemoteExecutionResult};
use aptos_logger::trace;
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::PartitionedTransactions, transaction::TransactionOutput,
    vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::executor_client::{ExecutorClient, ShardedExecutionOutput};
use crossbeam_channel::{Receiver, Sender};
use std::{
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, Mutex},
};

#[allow(dead_code)]
pub struct RemoteExecutorClient<S: StateView + Sync + Send + 'static> {
    // Channels to send execute block commands to the executor shards.
    command_txs: Arc<Vec<Mutex<Sender<Message>>>>,
    // Channels to receive execution results from the executor shards.
    result_rxs: Vec<Receiver<Message>>,
    // Thread pool used to pre-fetch the state values for the block in parallel and create an in-memory state view.
    thread_pool: Arc<rayon::ThreadPool>,

    phantom: std::marker::PhantomData<S>,
}

#[allow(dead_code)]
impl<S: StateView + Sync + Send + 'static> RemoteExecutorClient<S> {
    pub fn new(
        remote_shard_addresses: Vec<SocketAddr>,
        controller: &mut NetworkController,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = num_threads.unwrap_or_else(num_cpus::get);
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        let (command_txs, result_rxs) = remote_shard_addresses
            .iter()
            .enumerate()
            .map(|(shard_id, address)| {
                let execute_command_type = format!("execute_command_{}", shard_id);
                let execute_result_type = format!("execute_result_{}", shard_id);
                let command_tx =
                    Mutex::new(controller.create_outbound_channel(*address, execute_command_type));
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

    fn get_output_from_shards(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus> {
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

impl<S: StateView + Sync + Send + 'static> ExecutorClient<S> for RemoteExecutorClient<S> {
    fn num_shards(&self) -> usize {
        self.command_txs.len()
    }

    fn execute_block(
        &self,
        state_view: Arc<S>,
        transactions: PartitionedTransactions,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<ShardedExecutionOutput, VMStatus> {
        self.thread_pool.scope(|s| {
            let (block, global_txns) = transactions.into();
            assert!(
                global_txns.is_empty(),
                "Global transactions are not supported yet in remote execution mode."
            );
            for (shard_id, sub_blocks) in block.into_iter().enumerate() {
                let state_view = state_view.clone();
                let senders = self.command_txs.clone();
                s.spawn(move |_| {
                    let execution_request =
                        RemoteExecutionRequest::ExecuteBlock(ExecuteBlockCommand {
                            sub_blocks,
                            // TODO(skedia): Instead of serializing the entire state view, we should
                            // serialize only the state values needed for the shard.
                            state_view: S::as_in_memory_state_view(state_view.deref()),
                            concurrency_level: concurrency_level_per_shard,
                            maybe_block_gas_limit,
                        });

                    senders[shard_id]
                        .lock()
                        .unwrap()
                        .send(Message::new(bcs::to_bytes(&execution_request).unwrap()))
                        .unwrap()
                });
            }
        });

        let execution_results = self.get_output_from_shards()?;

        Ok(ShardedExecutionOutput::new(execution_results, vec![]))
    }
}
