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
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::TransactionOutputExt};
use aptos_block_executor::{
    errors::Error,
    executor::{BlockExecutor, RAYON_EXEC_POOL},
    task::{
        Transaction as BlockExecutorTransaction,
        TransactionOutput as BlockExecutorTransactionOutput,
    },
};
use aptos_infallible::Mutex;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use move_core_types::vm_status::VMStatus;
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use std::time::Instant;

impl BlockExecutorTransaction for PreprocessedTransaction {
    type Key = StateKey;
    type Value = WriteOp;
}

// Wrapper to avoid orphan rule
#[derive(Debug)]
pub(crate) struct AptosTransactionOutput {
    output_ext: Mutex<Option<TransactionOutputExt>>,
    committed_output: OnceCell<TransactionOutput>,
}

impl AptosTransactionOutput {
    pub(crate) fn new(output: TransactionOutputExt) -> Self {
        Self {
            output_ext: Mutex::new(Some(output)),
            committed_output: OnceCell::new(),
        }
    }

    fn take_output(mut self) -> TransactionOutput {
        match self.committed_output.take() {
            Some(output) => output,
            None => self
                .output_ext
                .lock()
                .take()
                .expect("Output must be set")
                .output_with_delta_writes(vec![]),
        }
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type Txn = PreprocessedTransaction;

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self {
        Self::new(TransactionOutputExt::from(TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Retry,
        )))
    }

    /// Should never be called after incorporate_delta_writes, as it will consume
    /// output_ext to prepare an output with deltas.
    fn get_writes(&self) -> Vec<(StateKey, WriteOp)> {
        self.output_ext
            .lock()
            .as_ref()
            .expect("Output to be set to get writes")
            .txn_output()
            .write_set()
            .iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect()
    }

    /// Should never be called after incorporate_delta_writes, as it will consume
    /// output_ext to prepare an output with deltas.
    fn get_deltas(&self) -> Vec<(StateKey, DeltaOp)> {
        self.output_ext
            .lock()
            .as_ref()
            .expect("Output to be set to get deltas")
            .delta_change_set()
            .iter()
            .map(|(key, op)| (key.clone(), *op))
            .collect()
    }

    /// Can be called (at most) once after transaction is committed to internally
    /// include the delta outputs with the transaction outputs.
    fn incorporate_delta_writes(&self, delta_writes: Vec<(StateKey, WriteOp)>) {
        assert!(
            self.committed_output
                .set(
                    self.output_ext
                        .lock()
                        .take()
                        .expect("Output must be set to combine with deltas")
                        .output_with_delta_writes(delta_writes),
                )
                .is_ok(),
            "Could not combine TransactionOutputExt with deltas"
        );
    }
}

pub struct BlockAptosVM();

impl BlockAptosVM {
    pub fn execute_block<S: StateView + Sync>(
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
            RAYON_EXEC_POOL.install(|| {
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

    fn execute_block_benchmark_parallel<S: StateView + Sync>(
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
    ) -> (
        usize,
        Option<Result<Vec<TransactionOutput>, Error<VMStatus>>>,
    ) {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verified_block: Vec<PreprocessedTransaction> =
            RAYON_EXEC_POOL.install(|| {
                transactions
                    .clone()
                    .into_par_iter()
                    .with_min_len(25)
                    .map(preprocess_transaction::<AptosVM>)
                    .collect()
            });
        let block_size = signature_verified_block.len();

        init_speculative_logs(signature_verified_block.len());

        BLOCK_EXECUTOR_CONCURRENCY.set(concurrency_level as i64);
        let executor = BlockExecutor::<PreprocessedTransaction, AptosExecutorTask<S>, S>::new(
            concurrency_level,
        );
        println!("Parallel execution starts...");
        let timer = Instant::now();
        let par_ret = executor
            .execute_block(state_view, signature_verified_block, state_view)
            .map(|outputs| {
                outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect()
            });

        let exec_t = timer.elapsed();
        println!(
            "Parallel execution finishes, TPS = {}",
            block_size * 1000 / exec_t.as_millis() as usize
        );

        flush_speculative_logs();

        (
            block_size * 1000 / exec_t.as_millis() as usize,
            Some(par_ret),
        )
    }

    fn execute_block_benchmark_sequential<S: StateView + Sync>(
        transactions: Vec<Transaction>,
        state_view: &S,
    ) -> (
        usize,
        Option<Result<Vec<TransactionOutput>, Error<VMStatus>>>,
    ) {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verified_block: Vec<PreprocessedTransaction> =
            RAYON_EXEC_POOL.install(|| {
                transactions
                    .clone()
                    .into_par_iter()
                    .with_min_len(25)
                    .map(preprocess_transaction::<AptosVM>)
                    .collect()
            });
        let block_size = signature_verified_block.len();

        // sequentially execute the block and check if the results match
        let seq_executor =
            BlockExecutor::<PreprocessedTransaction, AptosExecutorTask<S>, S>::new(1);
        println!("Sequential execution starts...");
        let seq_timer = Instant::now();
        let seq_ret = seq_executor
            .execute_block(state_view, signature_verified_block, state_view)
            .map(|outputs| {
                outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect()
            });
        let seq_exec_t = seq_timer.elapsed();
        println!(
            "Sequential execution finishes, TPS = {}",
            block_size * 1000 / seq_exec_t.as_millis() as usize
        );

        (
            block_size * 1000 / seq_exec_t.as_millis() as usize,
            Some(seq_ret),
        )
    }

    pub fn execute_block_benchmark<S: StateView + Sync>(
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        run_par: bool,
        run_seq: bool,
    ) -> (usize, usize) {
        let (par_tps, par_ret) = if run_par {
            BlockAptosVM::execute_block_benchmark_parallel(
                transactions.clone(),
                state_view,
                concurrency_level,
            )
        } else {
            (0, None)
        };
        let (seq_tps, seq_ret) = if run_seq {
            BlockAptosVM::execute_block_benchmark_sequential(transactions, state_view)
        } else {
            (0, None)
        };

        if let (Some(par), Some(seq)) = (par_ret.as_ref(), seq_ret.as_ref()) {
            assert_eq!(par, seq);
        }

        drop(par_ret);
        drop(seq_ret);

        (par_tps, seq_tps)
    }
}
