// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod storage_wrapper;
pub(crate) mod vm_wrapper;

use crate::{
    adapter_common::{preprocess_transaction, PreprocessedTransaction},
    block_executor::vm_wrapper::AptosExecutorTask,
    AptosVM,
};
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::TransactionOutputExt};
use aptos_block_executor::{
    errors::Error,
    executor::{BlockExecutor, RAYON_EXEC_POOL},
    output_delta_resolver::{OutputDeltaResolver, ResolvedData},
    task::{
        Transaction as BlockExecutorTransaction,
        TransactionOutput as BlockExecutorTransactionOutput,
    },
};
use aptos_logger::debug;
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::vm_status::VMStatus;
use rayon::prelude::*;
use std::collections::HashMap;

impl BlockExecutorTransaction for PreprocessedTransaction {
    type Key = StateKey;
    type Value = WriteOp;
}

// Wrapper to avoid orphan rule
pub(crate) struct AptosTransactionOutput(TransactionOutputExt);

impl AptosTransactionOutput {
    pub fn new(output: TransactionOutputExt) -> Self {
        Self(output)
    }

    pub fn into(self) -> TransactionOutputExt {
        self.0
    }

    pub fn as_ref(&self) -> &TransactionOutputExt {
        &self.0
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type T = PreprocessedTransaction;

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
    fn process_parallel_block_output<S: StateView>(
        results: Vec<AptosTransactionOutput>,
        delta_resolver: OutputDeltaResolver<StateKey, WriteOp>,
        state_view: &S,
    ) -> Vec<TransactionOutput> {
        // TODO: MVHashmap, and then delta resolver should track aggregator base values.
        let mut aggregator_base_values: HashMap<StateKey, anyhow::Result<ResolvedData>> =
            HashMap::new();
        for res in results.iter() {
            for (key, _) in res.as_ref().delta_change_set().iter() {
                if !aggregator_base_values.contains_key(key) {
                    aggregator_base_values.insert(key.clone(), state_view.get_state_value(key));
                }
            }
        }

        let materialized_deltas =
            delta_resolver.resolve(aggregator_base_values.into_iter().collect(), results.len());

        results
            .into_iter()
            .zip(materialized_deltas.into_iter())
            .map(|(res, delta_writes)| {
                res.into()
                    .output_with_delta_writes(WriteSetMut::new(delta_writes))
            })
            .collect()
    }

    fn process_sequential_block_output(
        results: Vec<AptosTransactionOutput>,
    ) -> Vec<TransactionOutput> {
        results
            .into_iter()
            .map(|res| {
                let (deltas, output) = res.into().into();
                debug_assert!(deltas.is_empty(), "[Execution] Deltas must be materialized");
                output
            })
            .collect()
    }

    pub fn execute_block<S: StateView>(
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verified_block: Vec<PreprocessedTransaction> =
            RAYON_EXEC_POOL.install(|| {
                transactions
                    .into_par_iter()
                    .map(preprocess_transaction::<AptosVM>)
                    .collect()
            });

        let executor =
            BlockExecutor::<PreprocessedTransaction, AptosExecutorTask<S>>::new(concurrency_level);

        let mut ret = if concurrency_level > 1 {
            executor
                .execute_transactions_parallel(state_view, &signature_verified_block)
                .map(|(results, delta_resolver)| {
                    Self::process_parallel_block_output(results, delta_resolver, state_view)
                })
        } else {
            executor
                .execute_transactions_sequential(state_view, &signature_verified_block)
                .map(Self::process_sequential_block_output)
        };

        if ret == Err(Error::ModulePathReadWrite) {
            debug!("[Execution]: Module read & written, sequential fallback");

            ret = executor
                .execute_transactions_sequential(state_view, &signature_verified_block)
                .map(Self::process_sequential_block_output);
        }

        // Explicit async drop. Happens here because we can't currently move to
        // BlockExecutor due to the Module publishing fallback. TODO: fix after
        // module publishing fallback is removed.
        RAYON_EXEC_POOL.spawn(move || {
            // Explicit async drops.
            drop(signature_verified_block);
        });

        match ret {
            Ok(outputs) => Ok(outputs),
            Err(Error::ModulePathReadWrite) => {
                unreachable!("[Execution]: Must be handled by sequential fallback")
            }
            Err(Error::UserError(err)) => Err(err),
        }
    }
}
