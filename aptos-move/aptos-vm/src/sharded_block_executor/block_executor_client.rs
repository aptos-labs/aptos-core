// Copyright © Aptos Foundation

use crate::block_executor::{AptosTransactionOutput, BlockAptosVM};
use aptos_block_executor::txn_commit_hook::NoOpTransactionCommitHook;
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{BlockExecutorTransactions, SubBlocksForShard},
    transaction::{Transaction, TransactionOutput},
};
use move_core_types::vm_status::VMStatus;
use std::sync::Arc;

pub trait BlockExecutorClient {
    fn execute_block<S: StateView + Sync + Send>(
        &self,
        transactions: SubBlocksForShard<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<Vec<TransactionOutput>>, VMStatus>;
}

impl BlockExecutorClient for VMExecutorClient {
    fn execute_block<S: StateView + Sync + Send>(
        &self,
        sub_blocks: SubBlocksForShard<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<Vec<TransactionOutput>>, VMStatus> {
        Ok(vec![BlockAptosVM::execute_block::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
        >(
            self.executor_thread_pool.clone(),
            BlockExecutorTransactions::Sharded(sub_blocks),
            state_view,
            concurrency_level,
            maybe_block_gas_limit,
            None,
        )?])
    }
}

pub struct VMExecutorClient {
    executor_thread_pool: Arc<rayon::ThreadPool>,
}

impl VMExecutorClient {
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

    pub fn create_vm_clients(num_shards: usize, num_threads: Option<usize>) -> Vec<Self> {
        let num_threads = num_threads
            .unwrap_or_else(|| (num_cpus::get() as f64 / num_shards as f64).ceil() as usize);
        (0..num_shards)
            .map(|_| VMExecutorClient::new(num_threads))
            .collect()
    }
}
