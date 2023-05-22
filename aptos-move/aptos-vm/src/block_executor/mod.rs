// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod vm_wrapper;

use crate::{
    adapter_common::{preprocess_transaction, PreprocessedTransaction},
    block_executor::vm_wrapper::AptosExecutorTask,
    counters::{
        BLOCK_EXECUTOR_CONCURRENCY, BLOCK_EXECUTOR_DUPLICATES_FILTERED,
        BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS, BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS,
    },
    AptosVM,
};
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::TransactionOutputExt};
use aptos_block_executor::{
    errors::Error,
    executor::BlockExecutor,
    task::{
        Transaction as BlockExecutorTransaction,
        TransactionOutput as BlockExecutorTransactionOutput,
    },
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::Mutex;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use moka::sync::SegmentedCache;
use move_core_types::vm_status::VMStatus;
use once_cell::sync::OnceCell;
use rayon::{prelude::*, ThreadPool};
use std::{collections::HashSet, sync::Arc};

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

    /// Return the amount of gas consumed by the transaction.
    fn gas_used(&self) -> u64 {
        self.committed_output
            .get()
            .map_or(0, |output| output.gas_used())
    }
}

pub struct BlockAptosVM();

/// Insert a transaction hash into `TXN_CACHE` if not present.
/// Return whether the insertion happened (i.e., cache miss).
fn insert_into_txn_cache(cache: &SegmentedCache<HashValue, u64>, txn: &Transaction) -> bool {
    let key = txn.hash();
    let nonce_in = rand::random::<u64>();
    let nonce_out = cache.get_with(key, || nonce_in);
    nonce_in == nonce_out
}

impl BlockAptosVM {
    pub fn execute_block<S: StateView + Sync>(
        executor_thread_pool: Arc<ThreadPool>,
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let _timer = BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();

        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verification_timer =
            BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS.start_timer();

        // Optimization to only check hashes for txns with matching (sender, sequence_number)
        let mut possible_duplicate_map = HashSet::new();
        {
            let mut seen = HashSet::new();
            for txn in &transactions {
                if let Transaction::UserTransaction(ref inner) = txn {
                    if !seen.insert((inner.sender(), inner.sequence_number())) {
                        possible_duplicate_map.insert((inner.sender(), inner.sequence_number()));
                    }
                }
            }
        }

        let duplicate_map = SegmentedCache::new(100000, 256);
        let signature_verified_block: Vec<PreprocessedTransaction> =
            executor_thread_pool.install(|| {
                transactions
                    .into_par_iter()
                    .with_min_len(25)
                    .map(|txn| match txn {
                        Transaction::UserTransaction(ref inner) => {
                            if possible_duplicate_map
                                .contains(&(inner.sender(), inner.sequence_number()))
                                && !insert_into_txn_cache(&duplicate_map, &txn)
                            {
                                return PreprocessedTransaction::Duplicate;
                            }
                            preprocess_transaction::<AptosVM>(txn)
                        },
                        _ => preprocess_transaction::<AptosVM>(txn),
                    })
                    .collect()
            });

        // Get the number for metrics
        let num_filtered = signature_verified_block
            .iter()
            .filter(|txn| matches!(txn, PreprocessedTransaction::Duplicate))
            .count();
        BLOCK_EXECUTOR_DUPLICATES_FILTERED.observe(num_filtered as f64);
        drop(signature_verification_timer);

        let num_txns = signature_verified_block.len();
        init_speculative_logs(num_txns);

        BLOCK_EXECUTOR_CONCURRENCY.set(concurrency_level as i64);
        let executor = BlockExecutor::<PreprocessedTransaction, AptosExecutorTask<S>, S>::new(
            concurrency_level,
            executor_thread_pool,
            maybe_gas_limit,
        );

        let ret = executor.execute_block(state_view, signature_verified_block, state_view);

        match ret {
            Ok(outputs) => {
                let output_vec: Vec<TransactionOutput> = outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect();

                // Flush the speculative logs of the committed transactions.
                let pos = output_vec.partition_point(|o| !o.status().is_retry());

                flush_speculative_logs(pos);

                Ok(output_vec)
            },
            Err(Error::ModulePathReadWrite) => {
                unreachable!("[Execution]: Must be handled by sequential fallback")
            },
            Err(Error::UserError(err)) => Err(err),
        }
    }
}
