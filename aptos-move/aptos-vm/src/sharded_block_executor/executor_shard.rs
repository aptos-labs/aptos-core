// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{
    block_executor_client::BlockExecutorClient, ExecutorShardCommand,
};
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::transaction::TransactionOutput;
use move_core_types::vm_status::VMStatus;
use std::sync::mpsc::{Receiver, Sender};

/// A remote block executor that receives transactions from a channel and executes them in parallel.
/// Currently it runs in the local machine and it will be further extended to run in a remote machine.
pub struct ExecutorShard<S, E> {
    num_shards: usize,
    shard_id: usize,
    executor_client: E,
    command_rx: Receiver<ExecutorShardCommand<S>>,
    result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
}

impl<S: StateView + Sync + Send + 'static, E: BlockExecutorClient> ExecutorShard<S, E> {
    pub fn new(
        num_shards: usize,
        executor_client: E,
        shard_id: usize,
        command_rx: Receiver<ExecutorShardCommand<S>>,
        result_tx: Sender<Result<Vec<Vec<TransactionOutput>>, VMStatus>>,
    ) -> Self {
        Self {
            num_shards,
            shard_id,
            executor_client,
            command_rx,
            result_tx,
        }
    }

    pub fn start(&self) {
        trace!(
            "Shard starting, shard_id={}, num_shards={}.",
            self.shard_id,
            self.num_shards
        );
        loop {
            let command = self.command_rx.recv().unwrap();
            match command {
                ExecutorShardCommand::ExecuteSubBlocks(
                    state_view,
                    transactions,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                ) => {
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        transactions.num_txns()
                    );
                    let ret = self.executor_client.execute_block(
                        transactions,
                        state_view.as_ref(),
                        concurrency_level_per_shard,
                        maybe_block_gas_limit,
                    );
                    drop(state_view);
                    self.result_tx.send(ret).unwrap();
                },
                ExecutorShardCommand::Stop => {
                    break;
                },
            }
        }
        trace!("Shard {} is shutting down", self.shard_id);
    }
}
