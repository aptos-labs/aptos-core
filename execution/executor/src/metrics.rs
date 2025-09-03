// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::{prelude::*, sample, warn};
use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge_vec, Histogram, HistogramVec, IntCounter,
    IntCounterVec, IntGaugeVec, TimerHelper,
};
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{
        authenticator::AccountAuthenticator, signature_verified_transaction::TransactionProvider,
        ExecutionStatus, Transaction, TransactionExecutableRef, TransactionOutput,
        TransactionStatus,
    },
};
use aptos_vm::AptosVM;
use move_core_types::{language_storage::CORE_CODE_ADDRESS, vm_status::StatusCode};
use once_cell::sync::Lazy;
use std::time::Duration;

pub static EXECUTE_CHUNK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_execute_chunk_seconds",
        // metric description
        "The time spent in seconds of chunk execution in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APPLY_CHUNK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_apply_chunk_seconds",
        // metric description
        "The time spent in seconds of applying txn output chunk in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static COMMIT_CHUNK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_commit_chunk_seconds",
        // metric description
        "The time spent in seconds of committing chunk in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_get_block_execution_output_by_executing_seconds",
        // metric description
        "The total time spent in seconds in executing execute_and_state_checkpoint in the BlockExecutorInner.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static OTHER_TIMERS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_executor_other_timers_seconds",
        // metric description
        "The time spent in seconds of others in Aptos executor",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static EXECUTOR_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_executor_error_total", "Cumulative number of errors").unwrap()
});

pub static BLOCK_EXECUTION_WORKFLOW_WHOLE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_block_execution_workflow_whole_seconds",
        // metric description
        "The total time spent in seconds in executing execute_and_state_checkpoint in the BlockExecutorInner.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static UPDATE_LEDGER: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_ledger_update_seconds",
        // metric description
        "The total time spent in ledger update in the block executor.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static CHUNK_OTHER_TIMERS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_chunk_executor_other_seconds",
        // metric description
        "The time spent in seconds of others in chunk executor.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static VM_EXECUTE_CHUNK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_vm_execute_chunk_seconds",
        // metric description
        "The total time spent in seconds of chunk execution in the chunk executor.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static COMMIT_BLOCKS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_commit_blocks_seconds",
        // metric description
        "The total time spent in seconds of commiting blocks in Aptos executor ",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static SAVE_TRANSACTIONS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_save_transactions_seconds",
        // metric description
        "The time spent in seconds of calling save_transactions to storage in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static TRANSACTIONS_SAVED: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_transactions_saved",
        // metric description
        "The number of transactions saved to storage in Aptos executor"
    )
    .unwrap()
});

//////////////////////////////////////
// EXECUTED TRANSACTION STATS COUNTERS
//////////////////////////////////////

/// Count of the executed transactions since last restart.
pub static PROCESSED_TXNS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_txns_count",
        "Count of the transactions since last restart. state is success, failed or retry",
        &["process", "kind", "state"]
    )
    .unwrap()
});

/// Count of the executed transactions since last restart.
pub static PROCESSED_FAILED_TXNS_REASON_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_failed_txns_reason_count",
        "Count of the transactions since last restart. state is success, failed or retry",
        &["is_detailed", "process", "state", "reason", "error_code"]
    )
    .unwrap()
});

/// Counter of executed user transactions by payload type
pub static PROCESSED_USER_TXNS_BY_PAYLOAD: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_user_transactions_by_payload",
        "Counter of processed user transactions by payload type",
        &["process", "payload_type", "state"]
    )
    .unwrap()
});

/// Counter of executed EntryFunction user transactions by module
pub static PROCESSED_USER_TXNS_ENTRY_FUNCTION_BY_MODULE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_user_transactions_entry_function_by_module",
        "Counter of processed EntryFunction user transactions by module",
        &["is_detailed", "process", "account", "name", "state"]
    )
    .unwrap()
});

/// Counter of executed EntryFunction user transaction for core address by method
pub static PROCESSED_USER_TXNS_ENTRY_FUNCTION_BY_CORE_METHOD: Lazy<IntCounterVec> =
    Lazy::new(|| {
        register_int_counter_vec!(
            "aptos_processed_user_transactions_entry_function_by_core_method",
            "Counter of processed EntryFunction user transaction for core address by method",
            &["process", "module", "method", "state"]
        )
        .unwrap()
    });

/// Counter of executed EntryFunction user transaction for core address by method
pub static PROCESSED_USER_TXNS_CORE_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_user_transactions_core_events",
        "Counter of processed EntryFunction user transaction for core address by method",
        &["is_detailed", "process", "account", "creation_number"]
    )
    .unwrap()
});

pub static PROCESSED_TXNS_OUTPUT_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_processed_txns_output_size",
        "Histogram of transaction output sizes",
        &["process"],
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 25).unwrap()
    )
    .unwrap()
});

pub static PROCESSED_TXNS_NUM_AUTHENTICATORS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_processed_txns_num_authenticators",
        "Histogram of number of authenticators in a transaction",
        &["process"],
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 6).unwrap()
    )
    .unwrap()
});

pub static PROCESSED_TXNS_AUTHENTICATOR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_txns_authenticator",
        "Counter of authenticators by type, for processed transactions",
        &["process", "auth_type"]
    )
    .unwrap()
});

pub static CONCURRENCY_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_executor_call_concurrency",
        "Call concurrency by API.",
        &["executor", "call"]
    )
    .unwrap()
});

pub fn update_counters_for_processed_chunk<T>(
    transactions: &[T],
    transaction_outputs: &[TransactionOutput],
    process_type: &str,
) where
    T: TransactionProvider,
{
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
        if detailed_counters {
            if let Ok(size) = bcs::serialized_size(output) {
                PROCESSED_TXNS_OUTPUT_SIZE.observe_with(&[process_type], size as f64);
            }
        }

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
                (
                    // Specialize duplicate txns for alerts
                    if *discard_status_code == StatusCode::SEQUENCE_NUMBER_TOO_OLD {
                        "discard_sequence_number_too_old"
                    } else if *discard_status_code == StatusCode::SEQUENCE_NUMBER_TOO_NEW {
                        "discard_sequence_number_too_new"
                    } else if *discard_status_code == StatusCode::TRANSACTION_EXPIRED {
                        "discard_transaction_expired"
                    } else if *discard_status_code == StatusCode::NONCE_ALREADY_USED {
                        "discard_nonce_already_used"
                    } else {
                        // Only log if it is an interesting discard
                        sample!(
                            SampleRate::Duration(Duration::from_secs(15)),
                            warn!(
                                "[sampled] Txn being discarded is {:?} with status code {:?}",
                                txn, discard_status_code
                            );
                        );
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

        let kind = match txn.get_transaction() {
            Some(Transaction::UserTransaction(_)) => "user_transaction",
            Some(Transaction::GenesisTransaction(_)) => "genesis",
            Some(Transaction::BlockMetadata(_)) => "block_metadata",
            Some(Transaction::BlockMetadataExt(_)) => "block_metadata_ext",
            Some(Transaction::StateCheckpoint(_)) => "state_checkpoint",
            Some(Transaction::BlockEpilogue(_)) => "block_epilogue",
            Some(Transaction::ValidatorTransaction(_)) => "validator_transaction",
            None => "unknown",
        };

        PROCESSED_TXNS_COUNT
            .with_label_values(&[process_type, kind, state])
            .inc();

        if !error_code.is_empty() {
            PROCESSED_FAILED_TXNS_REASON_COUNT
                .with_label_values(&[
                    detailed_counters_label,
                    process_type,
                    state,
                    reason,
                    &error_code,
                ])
                .inc();
        }

        if let Some(Transaction::UserTransaction(user_txn)) = txn.get_transaction() {
            if detailed_counters {
                let mut signature_count = 0;
                let account_authenticators = user_txn.authenticator_ref().all_signers();
                for account_authenticator in account_authenticators {
                    match account_authenticator {
                        AccountAuthenticator::Ed25519 { .. } => {
                            signature_count += 1;
                            PROCESSED_TXNS_AUTHENTICATOR
                                .with_label_values(&[process_type, "Ed25519"])
                                .inc();
                        },
                        AccountAuthenticator::MultiEd25519 { signature, .. } => {
                            let count = signature.signatures().len();
                            signature_count += count;
                            PROCESSED_TXNS_AUTHENTICATOR
                                .with_label_values(&[process_type, "Ed25519_in_MultiEd25519"])
                                .inc_by(count as u64);
                        },
                        AccountAuthenticator::SingleKey { authenticator } => {
                            signature_count += 1;
                            PROCESSED_TXNS_AUTHENTICATOR
                                .with_label_values(&[
                                    process_type,
                                    &format!("{}_in_SingleKey", authenticator.signature().name()),
                                ])
                                .inc();
                        },
                        AccountAuthenticator::MultiKey { authenticator } => {
                            for (_, signature) in authenticator.signatures() {
                                signature_count += 1;
                                PROCESSED_TXNS_AUTHENTICATOR
                                    .with_label_values(&[
                                        process_type,
                                        &format!("{}_in_MultiKey", signature.name()),
                                    ])
                                    .inc();
                            }
                        },
                        AccountAuthenticator::NoAccountAuthenticator => {
                            PROCESSED_TXNS_AUTHENTICATOR
                                .with_label_values(&[process_type, "NoAccountAuthenticator"])
                                .inc();
                        },
                        AccountAuthenticator::Abstraction { .. } => {
                            PROCESSED_TXNS_AUTHENTICATOR
                                .with_label_values(&[process_type, "AbstractionAuthenticator"])
                                .inc();
                        },
                    };
                }

                PROCESSED_TXNS_NUM_AUTHENTICATORS
                    .observe_with(&[process_type], signature_count as f64);
            }

            let payload_type = if user_txn.payload().is_multisig() {
                "multisig"
            } else {
                match user_txn.payload().executable_ref() {
                    Ok(TransactionExecutableRef::Script(_)) => "script",
                    Ok(TransactionExecutableRef::EntryFunction(_)) => "function",
                    Ok(TransactionExecutableRef::Empty) => "empty",
                    Err(_) => "deprecated_payload",
                }
            };
            if user_txn.payload().replay_protection_nonce().is_some() {
                PROCESSED_USER_TXNS_BY_PAYLOAD
                    .with_label_values(&[
                        process_type,
                        &(payload_type.to_string() + "_orderless"),
                        state,
                    ])
                    .inc();
            } else {
                PROCESSED_USER_TXNS_BY_PAYLOAD
                    .with_label_values(&[process_type, payload_type, state])
                    .inc();
            }

            if let Ok(TransactionExecutableRef::EntryFunction(function)) =
                user_txn.payload().executable_ref()
            {
                let is_core = function.module().address() == &CORE_CODE_ADDRESS;
                PROCESSED_USER_TXNS_ENTRY_FUNCTION_BY_MODULE
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
                    PROCESSED_USER_TXNS_ENTRY_FUNCTION_BY_CORE_METHOD
                        .with_label_values(&[
                            process_type,
                            function.module().name().as_str(),
                            function.function().as_str(),
                            state,
                        ])
                        .inc();
                }
            };
        }

        for event in output.events() {
            let (is_core, creation_number) = match event {
                ContractEvent::V1(v1) => (
                    v1.key().get_creator_address() == CORE_CODE_ADDRESS,
                    if detailed_counters {
                        v1.key().get_creation_number().to_string()
                    } else {
                        "event".to_string()
                    },
                ),
                ContractEvent::V2(_v2) => (false, "event".to_string()),
            };
            PROCESSED_USER_TXNS_CORE_EVENTS
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
