// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::block_executor::BlockAptosVM;
use aptos_block_executor::errors::Error;
use aptos_state_view::StateView;
use aptos_types::transaction::{Transaction, TransactionOutput};
use move_core_types::vm_status::VMStatus;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

/// A remote block executor that receives transactions from a channel and executes them in parallel.
/// Currently it runs in the local machine and it will be further extended to run in a remote machine.
pub struct ExecutorShard<S: StateView + Sync> {
    shard_id: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    quit_signal: Arc<Mutex<bool>>,
    num_executor_threads: usize,
    transactions_rx: Receiver<Vec<Transaction>>,
    result_tx: Sender<(usize, Result<Vec<TransactionOutput>, Error<VMStatus>>)>,
    state_view: Arc<S>,
}

impl<S: StateView + Sync> ExecutorShard<S> {
    pub fn new(
        shard_id: usize,
        concurrency_level: usize,
        quit_signal: Arc<Mutex<bool>>,
        state_view: Arc<S>,
        transactions_rx: Receiver<Vec<Transaction>>,
        result_tx: Sender<(usize, Result<Vec<TransactionOutput>, Error<VMStatus>>)>,
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
            quit_signal,
            num_executor_threads: concurrency_level,
            transactions_rx,
            result_tx,
            state_view,
        }
    }

    pub fn start(&self) {
        while !*self.quit_signal.lock().unwrap() {
            let transactions = self.transactions_rx.recv().unwrap();
            let ret = BlockAptosVM::execute_block_benchmark(
                self.executor_thread_pool.clone(),
                transactions.clone(),
                self.state_view.as_ref(),
                self.num_executor_threads,
            );
            self.result_tx.send((self.shard_id, ret)).unwrap();
        }
    }
}
