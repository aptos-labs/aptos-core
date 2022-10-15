// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{components::apply_chunk_output::ApplyChunkOutput, metrics};
use anyhow::Result;
use aptos_logger::{trace, warn};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
};
use aptos_vm::{AptosVM, VMExecutor};
use executor_types::ExecutedChunk;
use fail::fail_point;
use storage_interface::{
    cached_state_view::{CachedStateView, StateCache},
    ExecutedTrees,
};

pub struct ChunkOutput {
    /// Input transactions.
    pub transactions: Vec<Transaction>,
    /// Raw VM output.
    pub transaction_outputs: Vec<TransactionOutput>,
    /// Carries the frozen base state view, so all in-mem nodes involved won't drop before the
    /// execution result is processed; as well as al the accounts touched during execution, together
    /// with their proofs.
    pub state_cache: StateCache,
}

impl ChunkOutput {
    pub fn by_transaction_execution<V: VMExecutor>(
        transactions: Vec<Transaction>,
        state_view: CachedStateView,
    ) -> Result<Self> {
        let transaction_outputs = V::execute_block(transactions.clone(), &state_view)?;

        // to print txn output for debugging, uncomment:
        // println!("{:?}", transaction_outputs.iter().map(|t| t.status() ).collect::<Vec<_>>());

        update_counters_for_processed_chunk(&transactions, &transaction_outputs, "executed");

        Ok(Self {
            transactions,
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
        })
    }

    pub fn by_transaction_output(
        transactions_and_outputs: Vec<(Transaction, TransactionOutput)>,
        state_view: CachedStateView,
    ) -> Result<Self> {
        let (transactions, transaction_outputs): (Vec<_>, Vec<_>) =
            transactions_and_outputs.into_iter().unzip();

        update_counters_for_processed_chunk(&transactions, &transaction_outputs, "output");

        // collect all accounts touched and dedup
        let write_set = transaction_outputs
            .iter()
            .map(|o| o.write_set())
            .collect::<Vec<_>>();

        // prime the state cache by fetching all touched accounts
        state_view.prime_cache_by_write_set(write_set)?;

        Ok(Self {
            transactions,
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
        })
    }

    pub fn apply_to_ledger(
        self,
        base_view: &ExecutedTrees,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        fail_point!("executor::vm_execute_chunk", |_| {
            Err(anyhow::anyhow!("Injected error in apply_to_ledger."))
        });
        ApplyChunkOutput::apply(self, base_view)
    }

    pub fn trace_log_transaction_status(&self) {
        let status: Vec<_> = self
            .transaction_outputs
            .iter()
            .map(TransactionOutput::status)
            .cloned()
            .collect();

        if !status.is_empty() {
            trace!("Execution status: {:?}", status);
        }
    }
}

pub fn update_counters_for_processed_chunk(
    transactions: &[Transaction],
    transaction_outputs: &[TransactionOutput],
    process_type: &str,
) {
    let detailed_counters = AptosVM::get_processed_transactions_detailed_counters();
    let detailed_counters_label = if detailed_counters { "true" } else { "false" };
    if transactions.len() != transaction_outputs.len() {
        warn!(
            "Chunk lenthgs don't match: txns: {} and outputs: {}",
            transactions.len(),
            transaction_outputs.len()
        );
    }

    for (txn, output) in transactions.iter().zip(transaction_outputs.iter()) {
        let (state, reason, error_code) = match output.status() {
            TransactionStatus::Keep(execution_status) => match execution_status {
                ExecutionStatus::Success => ("keep_success", "", "".to_string()),
                ExecutionStatus::OutOfGas => ("keep_rejected", "OutOfGas", "error".to_string()),
                ExecutionStatus::MoveAbort { info, .. } => (
                    "keep_rejected",
                    "MoveAbort",
                    if detailed_counters {
                        info.as_ref()
                            .map(|v| v.reason_name.to_lowercase())
                            .unwrap_or_else(|| "none".to_string())
                    } else {
                        "error".to_string()
                    },
                ),
                ExecutionStatus::ExecutionFailure { .. } => {
                    ("keep_rejected", "ExecutionFailure", "error".to_string())
                }
                ExecutionStatus::MiscellaneousError(e) => (
                    "keep_rejected",
                    "MiscellaneousError",
                    if detailed_counters {
                        e.map(|v| format!("{:?}", v).to_lowercase())
                            .unwrap_or_else(|| "none".to_string())
                    } else {
                        "error".to_string()
                    },
                ),
            },
            TransactionStatus::Discard(discard_status_code) => (
                "discard",
                "error_code",
                if detailed_counters {
                    format!("{:?}", discard_status_code).to_lowercase()
                } else {
                    "error".to_string()
                },
            ),
            TransactionStatus::Retry => ("retry", "", "".to_string()),
        };

        let kind = match txn {
            Transaction::UserTransaction(_) => "user_transaction",
            Transaction::GenesisTransaction(_) => "genesis",
            Transaction::BlockMetadata(_) => "block_metadata",
            Transaction::StateCheckpoint(_) => "state_checkpoint",
        };

        metrics::APTOS_PROCESSED_TXNS_COUNT
            .with_label_values(&[process_type, kind, state])
            .inc();

        if !error_code.is_empty() {
            metrics::APTOS_PROCESSED_FAILED_TXNS_REASON_COUNT
                .with_label_values(&[
                    detailed_counters_label,
                    process_type,
                    state,
                    reason,
                    &error_code,
                ])
                .inc();
        }

        if let Transaction::UserTransaction(user_txn) = txn {
            match user_txn.payload() {
                aptos_types::transaction::TransactionPayload::Script(_script) => {
                    metrics::APTOS_PROCESSED_USER_TRANSACTIONS_PAYLOAD_TYPE
                        .with_label_values(&[process_type, "script", state])
                        .inc();
                }
                aptos_types::transaction::TransactionPayload::ModuleBundle(_module) => {
                    metrics::APTOS_PROCESSED_USER_TRANSACTIONS_PAYLOAD_TYPE
                        .with_label_values(&[process_type, "module", state])
                        .inc();
                }
                aptos_types::transaction::TransactionPayload::EntryFunction(function) => {
                    metrics::APTOS_PROCESSED_USER_TRANSACTIONS_PAYLOAD_TYPE
                        .with_label_values(&[process_type, "function", state])
                        .inc();

                    let is_core = function.module().address() == &CORE_CODE_ADDRESS;
                    metrics::APTOS_PROCESSED_USER_TRANSACTIONS_ENTRY_FUNCTION_MODULE
                        .with_label_values(&[
                            detailed_counters_label,
                            process_type,
                            if is_core { "core" } else { "user" },
                            if detailed_counters {
                                function.module().name().as_str()
                            } else if is_core {
                                "core_module"
                            } else {
                                "user_module"
                            },
                            state,
                        ])
                        .inc();
                    if is_core && detailed_counters {
                        metrics::APTOS_PROCESSED_USER_TRANSACTIONS_ENTRY_FUNCTION_CORE_METHOD
                            .with_label_values(&[
                                process_type,
                                function.module().name().as_str(),
                                function.function().as_str(),
                                state,
                            ])
                            .inc();
                    }
                }
            }
        }

        for event in output.events() {
            let is_core = event.key().get_creator_address() == CORE_CODE_ADDRESS;
            let creation_number = if is_core && detailed_counters {
                event.key().get_creation_number().to_string()
            } else {
                "event".to_string()
            };
            metrics::APTOS_PROCESSED_USER_TRANSACTIONS_CORE_EVENTS
                .with_label_values(&[
                    detailed_counters_label,
                    process_type,
                    if is_core { "core" } else { "user" },
                    &creation_number,
                ])
                .inc();
        }
    }
}
