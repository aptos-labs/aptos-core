// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_int_counter, register_int_counter_vec,
    Histogram, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;

pub static APTOS_EXECUTOR_EXECUTE_CHUNK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_execute_chunk_seconds",
        // metric description
        "The time spent in seconds of chunk execution in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_APPLY_CHUNK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_apply_chunk_seconds",
        // metric description
        "The time spent in seconds of applying txn output chunk in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_COMMIT_CHUNK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_commit_chunk_seconds",
        // metric description
        "The time spent in seconds of committing chunk in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_vm_execute_block_seconds",
        // metric description
        "The time spent in seconds of vm block execution in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_executor_error_total", "Cumulative number of errors").unwrap()
});

pub static APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_execute_block_seconds",
        // metric description
        "The total time spent in seconds of block execution in the block executor.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_VM_EXECUTE_CHUNK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_vm_execute_chunk_seconds",
        // metric description
        "The total time spent in seconds of chunk execution in the chunk executor.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_commit_blocks_seconds",
        // metric description
        "The total time spent in seconds of commiting blocks in Aptos executor ",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_save_transactions_seconds",
        // metric description
        "The time spent in seconds of calling save_transactions to storage in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_EXECUTOR_TRANSACTIONS_SAVED: Lazy<Histogram> = Lazy::new(|| {
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
pub static APTOS_PROCESSED_TXNS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_txns_count",
        "Count of the transactions since last restart. state is success, failed or retry",
        &["process", "kind", "state"]
    )
    .unwrap()
});

/// Count of the executed transactions since last restart.
pub static APTOS_PROCESSED_FAILED_TXNS_REASON_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_failed_txns_reason_count",
        "Count of the transactions since last restart. state is success, failed or retry",
        &["is_detailed", "process", "state", "reason", "error_code"]
    )
    .unwrap()
});

/// Counter of executed user transactions by payload type
pub static APTOS_PROCESSED_USER_TRANSACTIONS_PAYLOAD_TYPE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_user_transactions_by_payload",
        "Counter of processed user transactions by payload type",
        &["process", "payload_type", "state"]
    )
    .unwrap()
});

/// Counter of executed EntryFunction user transactions by module
pub static APTOS_PROCESSED_USER_TRANSACTIONS_ENTRY_FUNCTION_MODULE: Lazy<IntCounterVec> =
    Lazy::new(|| {
        register_int_counter_vec!(
            "aptos_processed_user_transactions_entry_function_by_module",
            "Counter of processed EntryFunction user transactions by module",
            &["is_detailed", "process", "account", "name", "state"]
        )
        .unwrap()
    });

/// Counter of executed EntryFunction user transaction for core address by method
pub static APTOS_PROCESSED_USER_TRANSACTIONS_ENTRY_FUNCTION_CORE_METHOD: Lazy<IntCounterVec> =
    Lazy::new(|| {
        register_int_counter_vec!(
            "aptos_processed_user_transactions_entry_function_by_core_method",
            "Counter of processed EntryFunction user transaction for core address by method",
            &["process", "module", "method", "state"]
        )
        .unwrap()
    });

/// Counter of executed EntryFunction user transaction for core address by method
pub static APTOS_PROCESSED_USER_TRANSACTIONS_CORE_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_processed_user_transactions_core_events",
        "Counter of processed EntryFunction user transaction for core address by method",
        &["is_detailed", "process", "account", "creation_number"]
    )
    .unwrap()
});
