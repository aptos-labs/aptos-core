// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod storage_wrapper;
mod vm_wrapper;

use crate::{
    adapter_common::{preprocess_transaction, PreprocessedTransaction},
    aptos_vm::AptosVM,
    logging::AdapterLogSchema,
    parallel_executor::vm_wrapper::AptosVMWrapper,
};
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::TransactionOutputExt};
use aptos_logger::{debug, info};
use aptos_parallel_executor::{
    errors::Error,
    executor::{ParallelTransactionExecutor, RAYON_EXEC_POOL},
    output_delta_resolver::ResolvedData,
    task::{Transaction as PTransaction, TransactionOutput as PTransactionOutput},
};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::vm_status::{StatusCode, VMStatus};
use rayon::prelude::*;
use std::collections::HashMap;

impl PTransaction for PreprocessedTransaction {
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

impl PTransactionOutput for AptosTransactionOutput {
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

pub struct ParallelAptosVM();

impl ParallelAptosVM {
    pub fn execute_block<S: StateView>(
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        // TODO: use the same threadpool as for parallel execution.
        let signature_verified_block: Vec<PreprocessedTransaction> =
            RAYON_EXEC_POOL.install(|| {
                transactions
                    .par_iter()
                    .map(|txn| preprocess_transaction::<AptosVM>(txn.clone()))
                    .collect()
            });

        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        info!(
            log_context,
            "Executing block, transaction count: {}",
            transactions.len()
        );

        let executor =
            ParallelTransactionExecutor::<PreprocessedTransaction, AptosVMWrapper<S>>::new(
                concurrency_level,
            );

        let mut ret = None;
        if concurrency_level > 1 {
            ret = Some(
                executor
                    .execute_transactions_parallel(state_view, &signature_verified_block)
                    .map(|(results, delta_resolver)| {
                        // TODO: with more deltas, collect keys in parallel (in parallel executor).
                        let mut aggregator_keys: HashMap<StateKey, anyhow::Result<ResolvedData>> =
                            HashMap::new();

                        for res in results.iter() {
                            let output_ext = AptosTransactionOutput::as_ref(res);
                            for (key, _) in output_ext.delta_change_set().iter() {
                                if !aggregator_keys.contains_key(key) {
                                    aggregator_keys
                                        .insert(key.clone(), state_view.get_state_value(key));
                                }
                            }
                        }

                        let materialized_deltas = delta_resolver
                            .resolve(aggregator_keys.into_iter().collect(), results.len());

                        results
                            .into_iter()
                            .zip(materialized_deltas.into_iter())
                            .map(|(res, delta_writes)| {
                                let output_ext = AptosTransactionOutput::into(res);
                                output_ext.output_with_delta_writes(WriteSetMut::new(delta_writes))
                            })
                            .collect()
                    }),
            );
        }

        if ret.is_none() || ret == Some(Err(Error::ModulePathReadWrite)) {
            debug!("[Execution]: Module read & written, sequential fallback");

            ret = Some(
                executor
                    .execute_transactions_sequential(state_view, &signature_verified_block)
                    .map(|results| {
                        results
                            .into_iter()
                            .map(|res| {
                                let output_ext = AptosTransactionOutput::into(res);
                                let (deltas, output) = output_ext.into();
                                debug_assert!(
                                    deltas.is_empty(),
                                    "[Execution] Deltas must be materialized"
                                );
                                output
                            })
                            .collect()
                    }),
            );
        }

        RAYON_EXEC_POOL.spawn(move || {
            // Explicit async drop.
            drop(signature_verified_block);
        });

        match ret.unwrap() {
            Ok(outputs) => Ok(outputs),
            Err(Error::ModulePathReadWrite) => {
                unreachable!("[Execution]: Must be handled by sequential fallback")
            }
            Err(Error::InvariantViolation) => Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )),
            Err(Error::UserError(err)) => Err(err),
        }
    }
}
