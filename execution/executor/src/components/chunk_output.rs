// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{components::apply_chunk_output::ApplyChunkOutput, metrics};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutedBlock, ExecutedChunk};
use aptos_infallible::Mutex;
use aptos_logger::{sample, sample::SampleRate, trace, warn};
use aptos_storage_interface::{
    cached_state_view::{CachedStateView, StateCache},
    ExecutedTrees,
};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    block_executor::partitioner::{ExecutableTransactions, PartitionedTransactions},
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
};
use aptos_vm::{
    sharded_block_executor::{
        local_executor_shard::{LocalExecutorClient, LocalExecutorService},
        ShardedBlockExecutor,
    },
    AptosVM, VMExecutor,
};
use fail::fail_point;
use move_core_types::vm_status::StatusCode;
use once_cell::sync::Lazy;
use std::{ops::Deref, sync::Arc, time::Duration};

pub static SHARDED_BLOCK_EXECUTOR: Lazy<
    Arc<Mutex<ShardedBlockExecutor<CachedStateView, LocalExecutorClient<CachedStateView>>>>,
> = Lazy::new(|| {
    let client = LocalExecutorService::setup_local_executor_shards(AptosVM::get_num_shards(), None);
    Arc::new(Mutex::new(ShardedBlockExecutor::new(client)))
});

pub struct ChunkOutput {
    /// Input transactions.
    pub transactions: Vec<Transaction>,
    /// Raw VM output.
    pub transaction_outputs: Vec<TransactionOutput>,
    /// Carries the frozen base state view, so all in-mem nodes involved won't drop before the
    /// execution result is processed; as well as all the accounts touched during execution, together
    /// with their proofs.
    pub state_cache: StateCache,
}

impl ChunkOutput {
    pub fn by_transaction_execution<V: VMExecutor>(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Self> {
        match transactions {
            ExecutableTransactions::Unsharded(txns) => {
                Self::by_transaction_execution_unsharded::<V>(
                    txns,
                    state_view,
                    maybe_block_gas_limit,
                )
            },
            ExecutableTransactions::Sharded(txns) => {
                Self::by_transaction_execution_sharded::<V>(txns, state_view, maybe_block_gas_limit)
            },
        }
    }

    fn by_transaction_execution_unsharded<V: VMExecutor>(
        transactions: Vec<Transaction>,
        state_view: CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Self> {
        let transaction_outputs =
            Self::execute_block::<V>(transactions.clone(), &state_view, maybe_block_gas_limit)?;

        // to print txn output for debugging, uncomment:
        // println!("{:?}", transaction_outputs.iter().map(|t| t.status() ).collect::<Vec<_>>());

        update_counters_for_processed_chunk(&transactions, &transaction_outputs, "executed");

        Ok(Self {
            transactions,
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
        })
    }

    pub fn by_transaction_execution_sharded<V: VMExecutor>(
        transactions: PartitionedTransactions,
        state_view: CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Self> {
        let state_view_arc = Arc::new(state_view);
        let transaction_outputs = Self::execute_block_sharded::<V>(
            transactions.clone(),
            state_view_arc.clone(),
            maybe_block_gas_limit,
        )?;

        // TODO(skedia) add logic to emit counters per shard instead of doing it globally.

        // Unwrapping here is safe because the execution has finished and it is guaranteed that
        // the state view is not used anymore.
        let state_view = Arc::try_unwrap(state_view_arc).unwrap();

        Ok(Self {
            transactions: PartitionedTransactions::flatten(transactions)
                .into_iter()
                .map(|t| t.into_txn())
                .collect(),
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
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        fail_point!("executor::apply_to_ledger", |_| {
            Err(anyhow::anyhow!("Injected error in apply_to_ledger."))
        });
        ApplyChunkOutput::apply_chunk(self, base_view, append_state_checkpoint_to_block)
    }

    pub fn apply_to_ledger_for_block(
        self,
        base_view: &ExecutedTrees,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<(ExecutedBlock, Vec<Transaction>, Vec<Transaction>)> {
        fail_point!("executor::apply_to_ledger_for_block", |_| {
            Err(anyhow::anyhow!(
                "Injected error in apply_to_ledger_for_block."
            ))
        });
        ApplyChunkOutput::apply_block(self, base_view, append_state_checkpoint_to_block)
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

    fn execute_block_sharded<V: VMExecutor>(
        partitioned_txns: PartitionedTransactions,
        state_view: Arc<CachedStateView>,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>> {
        Ok(V::execute_block_sharded(
            SHARDED_BLOCK_EXECUTOR.lock().deref(),
            partitioned_txns,
            state_view,
            maybe_block_gas_limit,
        )?)
    }

    /// Executes the block of [Transaction]s using the [VMExecutor] and returns
    /// a vector of [TransactionOutput]s.
    #[cfg(not(feature = "consensus-only-perf-test"))]
    fn execute_block<V: VMExecutor>(
        transactions: Vec<Transaction>,
        state_view: &CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>> {
        Ok(V::execute_block(
            transactions,
            state_view,
            maybe_block_gas_limit,
        )?)
    }

    /// In consensus-only mode, executes the block of [Transaction]s using the
    /// [VMExecutor] only if its a genesis block. In all other cases, this
    /// method returns an [TransactionOutput] with an empty [WriteSet], constant
    /// gas and a [ExecutionStatus::Success] for each of the [Transaction]s.
    #[cfg(feature = "consensus-only-perf-test")]
    fn execute_block<V: VMExecutor>(
        transactions: Vec<Transaction>,
        state_view: &CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>> {
        use aptos_state_view::{StateViewId, TStateView};
        use aptos_types::write_set::WriteSet;

        let transaction_outputs = match state_view.id() {
            // this state view ID implies a genesis block in non-test cases.
            StateViewId::Miscellaneous => {
                V::execute_block(transactions, state_view, maybe_block_gas_limit)?
            },
            _ => transactions
                .iter()
                .map(|_| {
                    TransactionOutput::new(
                        WriteSet::default(),
                        Vec::new(),
                        100,
                        TransactionStatus::Keep(ExecutionStatus::Success),
                    )
                })
                .collect::<Vec<_>>(),
        };
        Ok(transaction_outputs)
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
                },
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
            TransactionStatus::Discard(discard_status_code) => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(15)),
                    warn!(
                        "Txn being discarded is {:?} with status code {:?}",
                        txn, discard_status_code
                    )
                );
                (
                    // Specialize duplicate txns for alerts
                    if *discard_status_code == StatusCode::SEQUENCE_NUMBER_TOO_OLD {
                        "discard_sequence_number_too_old"
                    } else {
                        "discard"
                    },
                    "error_code",
                    if detailed_counters {
                        format!("{:?}", discard_status_code).to_lowercase()
                    } else {
                        "error".to_string()
                    },
                )
            },
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
                },
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
                },
                aptos_types::transaction::TransactionPayload::Multisig(_) => {
                    metrics::APTOS_PROCESSED_USER_TRANSACTIONS_PAYLOAD_TYPE
                        .with_label_values(&[process_type, "multisig", state])
                        .inc();
                },

                // Deprecated. Will be removed in the future.
                aptos_types::transaction::TransactionPayload::ModuleBundle(_module) => {
                    metrics::APTOS_PROCESSED_USER_TRANSACTIONS_PAYLOAD_TYPE
                        .with_label_values(&[process_type, "module", state])
                        .inc();
                },
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
