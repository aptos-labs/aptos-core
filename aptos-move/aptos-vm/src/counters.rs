// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_int_counter, register_int_counter_vec,
    register_int_gauge, Histogram, IntCounter, IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

pub static BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "block_executor_execute_block_seconds",
        // metric description
        "The time spent in seconds for executing a block in block executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static BLOCK_EXECUTOR_CONCURRENCY: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "block_executor_concurrency",
        "Concurrency level for the block executor"
    )
    .unwrap()
});

pub static BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "block_executor_signature_verification_seconds",
        // metric description
        "The time spent in seconds for signature verification of a block in executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Count the number of transactions that brake invariants of VM.
pub static TRANSACTIONS_INVARIANT_VIOLATION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_vm_transactions_invariant_violation",
        "Number of transactions that broke VM invariant",
    )
    .unwrap()
});

/// Count the number of transactions validated, with a "status" label to
/// distinguish success or failure results.
pub static TRANSACTIONS_VALIDATED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_vm_transactions_validated",
        "Number of transactions validated",
        &["status"]
    )
    .unwrap()
});

/// Count the number of user transactions executed, with a "status" label to
/// distinguish completed vs. discarded transactions.
pub static USER_TRANSACTIONS_EXECUTED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_vm_user_transactions_executed",
        "Number of user transactions executed",
        &["status"]
    )
    .unwrap()
});

/// Count the number of system transactions executed.
pub static SYSTEM_TRANSACTIONS_EXECUTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_vm_system_transactions_executed",
        "Number of system transactions executed"
    )
    .unwrap()
});

pub static BLOCK_TRANSACTION_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_num_txns_per_block",
        "Number of transactions per block"
    )
    .unwrap()
});

pub static TXN_TOTAL_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_txn_total_seconds",
        "Execution time per user transaction"
    )
    .unwrap()
});

pub static TXN_VALIDATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_txn_validation_seconds",
        "Validation time per user transaction"
    )
    .unwrap()
});

pub static TXN_GAS_USAGE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!("aptos_vm_txn_gas_usage", "Gas used per transaction").unwrap()
});
