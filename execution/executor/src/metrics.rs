// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge_vec, Histogram, HistogramVec, IntCounter,
    IntCounterVec, IntGaugeVec,
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

pub static APTOS_EXECUTOR_OTHER_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
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

pub static APTOS_EXECUTOR_LEDGER_UPDATE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_ledger_update_seconds",
        // metric description
        "The total time spent in ledger update in the block executor.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_CHUNK_EXECUTOR_OTHER_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
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

pub static APTOS_PROCESSED_TXNS_OUTPUT_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_processed_txns_output_size",
        "Histogram of transaction output sizes",
        &["process"],
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 25).unwrap()
    )
    .unwrap()
});

pub static APTOS_PROCESSED_TXNS_NUM_AUTHENTICATORS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_processed_txns_num_authenticators",
        "Histogram of number of authenticators in a transaction",
        &["process"],
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 6).unwrap()
    )
    .unwrap()
});

pub static APTOS_PROCESSED_TXNS_AUTHENTICATOR: Lazy<IntCounterVec> = Lazy::new(|| {
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
