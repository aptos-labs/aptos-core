// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::{block_executor::BlockAptosVM, sharded_block_executor::ExecutorShardCommand};
use aptos_block_executor::errors::Error;
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::transaction::TransactionOutput;
use move_core_types::vm_status::VMStatus;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

/// A remote block executor that receives transactions from a channel and executes them in parallel.
/// Currently it runs in the local machine and it will be further extended to run in a remote machine.
pub struct ExecutorShard<S: StateView + Sync> {
    shard_id: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    num_executor_threads: usize,
    command_rx: Receiver<ExecutorShardCommand>,
    result_tx: Sender<Result<Vec<TransactionOutput>, Error<VMStatus>>>,
    state_view: Arc<S>,
}

impl<S: StateView + Sync> ExecutorShard<S> {
    pub fn new(
        shard_id: usize,
        concurrency_level: usize,
        state_view: Arc<S>,
        command_rx: Receiver<ExecutorShardCommand>,
        result_tx: Sender<Result<Vec<TransactionOutput>, Error<VMStatus>>>,
    ) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(concurrency_level)
                .build()
                .unwrap(),
        );
        Self {
            shard_id,
            executor_thread_pool,
            num_executor_threads: concurrency_level,
            command_rx,
            result_tx,
            state_view,
        }
    }

    pub fn start(&self) {
        loop {
            let command = self.command_rx.recv().unwrap();
            match command {
                ExecutorShardCommand::ExecuteBlock(transactions) => {
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        transactions.len()
                    );
                    let ret = BlockAptosVM::execute_block_benchmark(
                        self.executor_thread_pool.clone(),
                        transactions,
                        self.state_view.as_ref(),
                        self.num_executor_threads,
                    );
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
