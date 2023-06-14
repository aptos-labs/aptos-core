// Copyright © Aptos Foundation

use crate::block_executor::BlockAptosVM;
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::ExecutableTransactions,
    transaction::{Transaction, TransactionOutput},
};
use move_core_types::vm_status::VMStatus;
use std::sync::Arc;

pub trait BlockExecutorClient {
    fn execute_block<S: StateView + Sync>(
        &self,
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus>;
}

impl BlockExecutorClient for LocalExecutorClient {
    fn execute_block<S: StateView + Sync>(
        &self,
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        BlockAptosVM::execute_block(
            self.executor_thread_pool.clone(),
            // TODO: (skedia) Change this to sharded transactions
            ExecutableTransactions::Unsharded(transactions),
            state_view,
            concurrency_level,
            maybe_block_gas_limit,
        )
    }
}

pub struct LocalExecutorClient {
    executor_thread_pool: Arc<rayon::ThreadPool>,
}

impl LocalExecutorClient {
    pub fn new(num_threads: usize) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );

        Self {
            executor_thread_pool,
        }
    }

    pub fn create_local_clients(num_shards: usize, num_threads: Option<usize>) -> Vec<Self> {
        let num_threads = num_threads
            .unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);
        (0..num_shards)
            .map(|_| LocalExecutorClient::new(num_threads))
            .collect()
    }
}
