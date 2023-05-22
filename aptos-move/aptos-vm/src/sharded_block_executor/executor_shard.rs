// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block_executor::BlockAptosVM, sharded_block_executor::ExecutorShardCommand};
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::transaction::TransactionOutput;
use aptos_vm_logging::disable_speculative_logging;
use move_core_types::vm_status::VMStatus;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

/// A remote block executor that receives transactions from a channel and executes them in parallel.
/// Currently it runs in the local machine and it will be further extended to run in a remote machine.
pub struct ExecutorShard<S: StateView + Sync + Send + 'static> {
    shard_id: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    command_rx: Receiver<ExecutorShardCommand<S>>,
    result_tx: Sender<Result<Vec<TransactionOutput>, VMStatus>>,
    maybe_gas_limit: Option<u64>,
}

impl<S: StateView + Sync + Send + 'static> ExecutorShard<S> {
    pub fn new(
        shard_id: usize,
        num_executor_threads: usize,
        command_rx: Receiver<ExecutorShardCommand<S>>,
        result_tx: Sender<Result<Vec<TransactionOutput>, VMStatus>>,
        maybe_gas_limit: Option<u64>,
    ) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_executor_threads)
                .build()
                .unwrap(),
        );
        disable_speculative_logging();

        Self {
            shard_id,
            executor_thread_pool,
            command_rx,
            result_tx,
            maybe_gas_limit,
        }
    }

    pub fn start(&self) {
        loop {
            let command = self.command_rx.recv().unwrap();
            match command {
                ExecutorShardCommand::ExecuteBlock(
                    state_view,
                    transactions,
                    concurrency_level_per_shard,
                ) => {
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        transactions.len()
                    );
                    let ret = BlockAptosVM::execute_block(
                        self.executor_thread_pool.clone(),
                        transactions,
                        state_view.as_ref(),
                        concurrency_level_per_shard,
                        self.maybe_gas_limit,
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
