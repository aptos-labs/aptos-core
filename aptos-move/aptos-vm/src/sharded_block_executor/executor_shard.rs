// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block_executor::BlockAptosVM, sharded_block_executor::ExecutorShardCommand};
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::transaction::TransactionOutput;
use aptos_vm_logging::disable_speculative_logging;
use move_core_types::vm_status::VMStatus;
use std::sync::{mpsc::{Receiver, Sender}, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

/// A remote block executor that receives transactions from a channel and executes them in parallel.
/// Currently it runs in the local machine and it will be further extended to run in a remote machine.
pub struct ExecutorShard<S: StateView + Sync + Send + 'static> {
    shard_id: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    tx: Sender<ExecutorShardCommand<S>>,
    rx: Receiver<ExecutorShardCommand<S>>,
    master_tx: Sender<Result<Vec<TransactionOutput>, VMStatus>>,
    peer_txs: Vec<Sender<ExecutorShardCommand<S>>>,
    maybe_gas_limit: Option<u64>,
    maybe_state_view: Option<Arc<S>>,
    maybe_work_thread: Option<JoinHandle<()>>,
}

impl<S: StateView + Sync + Send + 'static> ExecutorShard<S> {
    pub fn new(
        num_executor_shards: usize,
        shard_id: usize,
        num_executor_threads: usize,
        command_tx: Sender<ExecutorShardCommand<S>>,
        command_rx: Receiver<ExecutorShardCommand<S>>,
        master_tx: Sender<Result<Vec<TransactionOutput>, VMStatus>>,
        peer_txs: Vec<Sender<ExecutorShardCommand<S>>>,
        maybe_gas_limit: Option<u64>,
    ) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_executor_threads)
                .build()
                .unwrap(),
        );

        if num_executor_shards > 1 {
            // todo: speculative logging is not yet compatible with sharded block executor.
            disable_speculative_logging();
        }

        Self {
            shard_id,
            executor_thread_pool,
            tx: command_tx,
            rx: command_rx,
            master_tx,
            peer_txs,
            maybe_gas_limit,
            maybe_state_view: None,
            maybe_work_thread: None,
        }
    }

    pub fn start(&mut self) {
        loop {
            let command = self.rx.recv().unwrap();
            match command {
                ExecutorShardCommand::ExecuteBlock(
                    state_view,
                    transactions,
                    concurrency_level_per_shard,
                ) => {
                    let shard_id = self.shard_id;
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        shard_id,
                        transactions.len()
                    );
                    self.maybe_state_view = Some(state_view.clone());
                    let tx_clone = self.tx.clone();
                    let pool = self.executor_thread_pool.clone();
                    let maybe_gas_limit = self.maybe_gas_limit;

                    let join_handle = thread::Builder::new().name(format!("executor-shard-{}-blockstm", shard_id)).spawn(move||{
                        let ret = BlockAptosVM::execute_block(
                            pool,
                            transactions,
                            state_view.as_ref(),
                            concurrency_level_per_shard,
                            maybe_gas_limit,
                        );
                        tx_clone.send(ExecutorShardCommand::ExeDone(ret)).unwrap();
                    }).unwrap();
                    self.maybe_work_thread = Some(join_handle);
                },
                ExecutorShardCommand::ExeDone(txo_or_vmst) => {
                    drop(self.maybe_state_view.take().unwrap());
                    self.master_tx.send(txo_or_vmst).unwrap();
                    self.maybe_work_thread.take().unwrap().join().unwrap();
                },
                ExecutorShardCommand::Stop => {
                    break;
                },
                ExecutorShardCommand::SubBlockFinished(sub_block_output) => {
                    println!("yes");
                }
            }
        }
        trace!("Shard {} is shutting down", self.shard_id);
    }
}
