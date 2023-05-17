// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod vm_wrapper;

use crate::{
    adapter_common::{preprocess_transaction, PreprocessedTransaction},
    block_executor::vm_wrapper::AptosExecutorTask,
    counters::{
        BLOCK_EXECUTOR_CONCURRENCY, BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS,
    },
    AptosVM,
};
use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_block_executor::{
    errors::Error,
    executor::BlockExecutor,
    task::{
        Transaction as BlockExecutorTransaction,
        TransactionOutput as BlockExecutorTransactionOutput,
    },
};
use aptos_infallible::Mutex;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
};
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use aptos_vm_types::{op::Op, vm_output::VMOutput, vm_view::AptosVMView};
use move_core_types::vm_status::VMStatus;
use once_cell::sync::OnceCell;
use rayon::{prelude::*, ThreadPool};
use std::sync::Arc;

impl BlockExecutorTransaction for PreprocessedTransaction {
    type Key = StateKey;
    type Value = Op<Vec<u8>>;
}

// Wrapper to avoid orphan rule
#[derive(Debug)]
pub(crate) struct AptosTransactionOutput {
    vm_output: Mutex<Option<VMOutput>>,
    committed_output: OnceCell<TransactionOutput>,
}

impl AptosTransactionOutput {
    pub(crate) fn new(output: VMOutput) -> Self {
        Self {
            vm_output: Mutex::new(Some(output)),
            committed_output: OnceCell::new(),
        }
    }

    fn take_output(mut self) -> TransactionOutput {
        match self.committed_output.take() {
            Some(output) => output,
            None => self
                .vm_output
                .lock()
                .take()
                .expect("Output must be set")
                .output_with_materialized_deltas(vec![]),
        }
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type Txn = PreprocessedTransaction;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Retry))
    }

    /// Should never be called after incorporate_materialized_deltas, as it
    /// will consume vm_output to prepare an output with deltas.
    fn get_resource_writes(&self) -> Vec<(StateKey, Op<Vec<u8>>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get writes")
            .resource_writes()
            .iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect()
    }

    fn get_module_writes(&self) -> Vec<(StateKey, Op<Vec<u8>>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get writes")
            .module_writes()
            .iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect()
    }

    fn get_aggregator_writes(&self) -> Vec<(StateKey, Op<Vec<u8>>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get writes")
            .aggregator_writes()
            .iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect()
    }

    /// Should never be called after incorporate_materialized_deltas, as it
    /// will consume vm_output to prepare an output with deltas.
    fn get_deltas(&self) -> Vec<(StateKey, DeltaOp)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get deltas")
            .deltas()
            .iter()
            .map(|(key, op)| (key.clone(), *op))
            .collect()
    }

    /// Can be called (at most) once after transaction is committed to internally
    /// include the materialized delta outputs with the transaction outputs.
    fn incorporate_materialized_deltas(&self, materialized_deltas: Vec<(StateKey, Op<Vec<u8>>)>) {
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .lock()
                        .take()
                        .expect("Output must be set to combine with deltas")
                        .output_with_materialized_deltas(materialized_deltas),
                )
                .is_ok(),
            "Could not combine VMOutput with materialized deltas"
        );
    }
}

pub struct BlockAptosVM();

impl BlockAptosVM {
    pub fn execute_block<S: AptosVMView + Sync>(
        executor_thread_pool: Arc<ThreadPool>,
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let _timer = BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verification_timer =
            BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS.start_timer();
        let signature_verified_block: Vec<PreprocessedTransaction> =
            executor_thread_pool.install(|| {
                transactions
                    .into_par_iter()
                    .with_min_len(25)
                    .map(preprocess_transaction::<AptosVM>)
                    .collect()
            });
        drop(signature_verification_timer);

        init_speculative_logs(signature_verified_block.len());

        BLOCK_EXECUTOR_CONCURRENCY.set(concurrency_level as i64);
        let executor = BlockExecutor::<PreprocessedTransaction, AptosExecutorTask<S>, S>::new(
            concurrency_level,
            executor_thread_pool,
        );

        let ret = executor.execute_block(state_view, signature_verified_block, state_view);

        flush_speculative_logs();

        match ret {
            Ok(outputs) => Ok(outputs
                .into_iter()
                .map(|output| output.take_output())
                .collect()),
            Err(Error::ModulePathReadWrite) => {
                unreachable!("[Execution]: Must be handled by sequential fallback")
            },
            Err(Error::UserError(err)) => Err(err),
        }
    }

    pub fn execute_block_benchmark<S: AptosVMView + Sync>(
        executor_thread_pool: Arc<ThreadPool>,
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
    ) -> Result<Vec<TransactionOutput>, Error<VMStatus>> {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verified_block: Vec<PreprocessedTransaction> =
            executor_thread_pool.install(|| {
                transactions
                    .clone()
                    .into_par_iter()
                    .with_min_len(25)
                    .map(preprocess_transaction::<AptosVM>)
                    .collect()
            });

        BLOCK_EXECUTOR_CONCURRENCY.set(concurrency_level as i64);
        let executor = BlockExecutor::<PreprocessedTransaction, AptosExecutorTask<S>, S>::new(
            concurrency_level,
            executor_thread_pool,
        );
        executor
            .execute_block(state_view, signature_verified_block, state_view)
            .map(|outputs| {
                outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect()
            })
    }
}
