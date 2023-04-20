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
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use move_core_types::vm_status::VMStatus;
use rayon::prelude::*;
use std::time::Instant;

impl BlockExecutorTransaction for PreprocessedTransaction {
    type Key = StateKey;
    type Value = WriteOp;
}

// Wrapper to avoid orphan rule
#[derive(PartialEq, Debug)]
pub(crate) struct AptosTransactionOutput(TransactionOutputExt);

impl AptosTransactionOutput {
    pub fn new(output: TransactionOutputExt) -> Self {
        Self(output)
    }

    pub fn into(self) -> TransactionOutputExt {
        self.0
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type Txn = PreprocessedTransaction;

    fn get_writes(&self) -> Vec<(StateKey, WriteOp)> {
        self.0
            .txn_output()
            .write_set()
            .iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect()
    }

    fn get_deltas(&self) -> Vec<(StateKey, DeltaOp)> {
        self.0
            .delta_change_set()
            .iter()
            .map(|(key, op)| (key.clone(), *op))
            .collect()
    }

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self {
        Self(TransactionOutputExt::from(TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Retry,
        )))
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

        let ret = executor
            .execute_block(state_view, signature_verified_block, state_view)
            .map(|results| {
                // Process the outputs in parallel, combining delta writes with other writes.
                // TODO: merge with rolling commit_hook (via trait) inside parallel executor.
                RAYON_EXEC_POOL.install(|| {
                    results
                        .into_par_iter()
                        .map(|(output, delta_writes)| {
                            output      // AptosTransactionOutput
                            .into()     // TransactionOutputExt
                            .output_with_delta_writes(WriteSetMut::new(delta_writes))
                        })
                        .collect()
                })
            });

        flush_speculative_logs();

        match ret {
            Ok(outputs) => Ok(outputs),
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
        let ret = executor.execute_block(state_view, signature_verified_block, state_view);
        let exec_t = timer.elapsed();
        println!(
            "Parallel execution finishes, TPS = {}",
            block_size * 1000 / exec_t.as_millis() as usize
        );

        flush_speculative_logs();

        // Merge the delta outputs for parallel execution in order to compare results.
        // TODO: remove after this becomes part of the rolling commit_hook.
        let par_ret = ret.map(|results| {
            results
                .into_iter()
                .map(|(output, delta_writes)| {
                    output // AptosTransactionOutput
                    .into() // TransactionOutputExt
                    .output_with_delta_writes(WriteSetMut::new(delta_writes))
                })
                .collect()
        });

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
        let seq_ret = seq_executor.execute_block(state_view, signature_verified_block, state_view);
        let seq_exec_t = seq_timer.elapsed();
        println!(
            "Sequential execution finishes, TPS = {}",
            block_size * 1000 / seq_exec_t.as_millis() as usize
        );

        // Sequential execution does not have deltas, assert and convert to the same type.
        let seq_ret: Result<Vec<TransactionOutput>, Error<VMStatus>> = seq_ret.map(|results| {
            results
                .into_iter()
                .map(|(output, deltas)| {
                    assert_eq!(deltas.len(), 0);
                    output
                        .into()
                        .output_with_delta_writes(WriteSetMut::new(vec![]))
                })
                .collect()
        });

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
