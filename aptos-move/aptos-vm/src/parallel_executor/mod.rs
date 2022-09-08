// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod storage_wrapper;
mod vm_wrapper;

use crate::{
    adapter_common::{preprocess_transaction, PreprocessedTransaction},
    aptos_vm::AptosVM,
    parallel_executor::vm_wrapper::AptosVMWrapper,
};
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::TransactionOutputExt};
use aptos_parallel_executor::{
    errors::Error,
    executor::ParallelTransactionExecutor,
    output_delta_resolver::ResolvedData,
    task::{Transaction as PTransaction, TransactionOutput as PTransactionOutput},
};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_deps::move_core_types::vm_status::{StatusCode, VMStatus};
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
    ) -> Result<(Vec<TransactionOutput>, Option<Error<VMStatus>>), VMStatus> {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verified_block: Vec<PreprocessedTransaction> = transactions
            .par_iter()
            .map(|txn| preprocess_transaction::<AptosVM>(txn.clone()))
            .collect();

        match ParallelTransactionExecutor::<PreprocessedTransaction, AptosVMWrapper<S>>::new(
            concurrency_level,
        )
        .execute_transactions_parallel(state_view, signature_verified_block)
        {
            Ok((results, delta_resolver)) => {
                // TODO: with more deltas, collect keys in parallel (in parallel executor).
                let mut aggregator_keys: HashMap<StateKey, anyhow::Result<ResolvedData>> =
                    HashMap::new();

                for res in results.iter() {
                    let output_ext = AptosTransactionOutput::as_ref(res);
                    for (key, _) in output_ext.delta_change_set().iter() {
                        if !aggregator_keys.contains_key(key) {
                            aggregator_keys.insert(key.clone(), state_view.get_state_value(key));
                        }
                    }
                }

                let materialized_deltas =
                    delta_resolver.resolve(aggregator_keys.into_iter().collect(), results.len());
                Ok((
                    results
                        .into_iter()
                        .zip(materialized_deltas.into_iter())
                        .map(|(res, delta_writes)| {
                            let output_ext = AptosTransactionOutput::into(res);
                            output_ext.output_with_delta_writes(WriteSetMut::new(delta_writes))
                        })
                        .collect(),
                    None,
                ))
            }
            Err(err @ Error::ModulePathReadWrite) => {
                let output = AptosVM::execute_block_and_keep_vm_status(transactions, state_view)?;
                Ok((
                    output
                        .into_iter()
                        .map(|(_vm_status, txn_output)| txn_output)
                        .collect(),
                    Some(err),
                ))
            }
            Err(Error::InvariantViolation) => Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )),
            Err(Error::UserError(err)) => Err(err),
        }
    }
}
