// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    histogram_opts, register_histogram, register_int_counter, register_int_counter_vec,
    register_int_gauge, Histogram, IntCounter, IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

const BLOCK_EXECUTION_TIME_BUCKETS: [f64; 25] = [
    0.05, 0.075, 0.10, 0.125, 0.15, 0.175, 0.20, 0.225, 0.25, 0.275, 0.30, 0.325, 0.35, 0.375,
    0.40, 0.425, 0.45, 0.475, 0.50, 0.55, 0.60, 0.70, 0.80, 0.90, 1.0,
];

pub static BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        // metric name
        "block_executor_execute_block_seconds",
        // metric description
        "The time spent in seconds for executing a block in block executor",
        BLOCK_EXECUTION_TIME_BUCKETS.to_vec()
    );
    register_histogram!(histogram_opts).unwrap()
});

pub static BLOCK_EXECUTOR_CONCURRENCY: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "block_executor_concurrency",
        "Concurrency level for the block executor"
    )
    .unwrap()
});

const SIGNATURE_VERIFICATION_TIME_BUCKETS: [f64; 25] = [
    0.000025, 0.00005, 0.000075, 0.0001, 0.000125, 0.00015, 0.0002, 0.00025, 0.0003, 0.00035,
    0.0004, 0.00045, 0.0005, 0.00055, 0.0006, 0.00065, 0.0007, 0.00075, 0.0008, 0.00085, 0.0009,
    0.001, 0.0015, 0.002, 0.003,
];

pub static BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        // metric name
        "block_executor_signature_verification_seconds",
        // metric description
        "The time spent in seconds for signature verification of a block in executor",
        SIGNATURE_VERIFICATION_TIME_BUCKETS.to_vec()
    );
    register_histogram!(histogram_opts).unwrap()
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

const NUM_BLOCK_TRANSACTIONS_BUCKETS: [f64; 34] = [
    2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0,
    90.0, 100.0, 125.0, 150.0, 175.0, 200.0, 250.0, 300.0, 350.0, 400.0, 450.0, 500.0, 600.0,
    700.0, 800.0, 900.0, 1000.0,
];

pub static BLOCK_TRANSACTION_COUNT: Lazy<Histogram> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_vm_num_txns_per_block",
        "Number of transactions per block",
        NUM_BLOCK_TRANSACTIONS_BUCKETS.to_vec()
    );
    register_histogram!(histogram_opts).unwrap()
});

const TRANSACTION_EXECUTION_TIME_BUCKETS: [f64; 32] = [
    0.001, 0.002, 0.003, 0.004, 0.005, 0.006, 0.007, 0.008, 0.009, 0.01, 0.011, 0.012, 0.013,
    0.014, 0.015, 0.016, 0.017, 0.018, 0.019, 0.020, 0.025, 0.03, 0.035, 0.04, 0.05, 0.06, 0.07,
    0.08, 0.09, 0.10, 0.11, 0.12,
];

pub static TXN_TOTAL_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_vm_txn_total_seconds",
        "Execution time per user transaction",
        TRANSACTION_EXECUTION_TIME_BUCKETS.to_vec()
    );
    register_histogram!(histogram_opts).unwrap()
});

const TRANSACTION_VALIDATION_TIME_BUCKETS: [f64; 20] = [
    0.00025, 0.0005, 0.00075, 0.001, 0.00125, 0.0015, 0.00175, 0.002, 0.00225, 0.0025, 0.00275,
    0.003, 0.00325, 0.0035, 0.00375, 0.004, 0.00425, 0.0045, 0.00475, 0.005,
];

pub static TXN_VALIDATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_vm_txn_validation_seconds",
        "Validation time per user transaction",
        TRANSACTION_VALIDATION_TIME_BUCKETS.to_vec()
    );
    register_histogram!(histogram_opts).unwrap()
});

const TXN_GAS_USAGE_BUCKETS: [f64; 18] = [
    2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0, 11.0,
];

pub static TXN_GAS_USAGE: Lazy<Histogram> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_vm_txn_gas_usage",
        "Gas used per transaction",
        TXN_GAS_USAGE_BUCKETS.to_vec()
    );
    register_histogram!(histogram_opts).unwrap()
});
