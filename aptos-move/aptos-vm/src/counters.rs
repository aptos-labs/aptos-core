// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, make_local_histogram_vec, make_local_int_counter,
    make_local_int_counter_vec, register_histogram, register_int_gauge, Histogram, HistogramVec,
    IntGauge,
};
use once_cell::sync::Lazy;

const BLOCK_EXECUTION_TIME_BUCKETS: [f64; 16] = [
    0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90, 1.0, 1.25, 1.5, 1.75, 2.0, 3.0, 4.0, 5.0,
];

pub static BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "block_executor_execute_block_seconds",
        // metric description
        "The time spent in seconds for executing a block in block executor",
        BLOCK_EXECUTION_TIME_BUCKETS.to_vec()
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

// Count the number of transactions that brake invariants of VM.
make_local_int_counter!(
    pub,
    TRANSACTIONS_INVARIANT_VIOLATION,
    "aptos_vm_transactions_invariant_violation",
    "Number of transactions that broke VM invariant",
);

// Count the number of transactions validated, with a "status" label to
// distinguish success or failure results.
make_local_int_counter_vec!(
    pub,
    TRANSACTIONS_VALIDATED,
    "aptos_vm_transactions_validated",
    "Number of transactions validated",
    &["status"]
);

// Count the number of user transactions executed, with a "status" label to
// distinguish completed vs. discarded transactions.
make_local_int_counter_vec!(
    pub,
    USER_TRANSACTIONS_EXECUTED,
    "aptos_vm_user_transactions_executed",
    "Number of user transactions executed",
    &["status"]
);

// Count the number of system transactions executed.
make_local_int_counter!(
    pub,
    SYSTEM_TRANSACTIONS_EXECUTED,
    "aptos_vm_system_transactions_executed",
    "Number of system transactions executed"
);

const NUM_BLOCK_TRANSACTIONS_BUCKETS: [f64; 24] = [
    5.0, 10.0, 20.0, 40.0, 75.0, 100.0, 200.0, 400.0, 800.0, 1200.0, 1800.0, 2500.0, 3300.0,
    4000.0, 5000.0, 6500.0, 8000.0, 10000.0, 12500.0, 15000.0, 18000.0, 21000.0, 25000.0, 30000.0,
];

pub static BLOCK_TRANSACTION_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_num_txns_per_block",
        "Number of transactions per block",
        NUM_BLOCK_TRANSACTIONS_BUCKETS.to_vec()
    )
    .unwrap()
});

const TRANSACTION_EXECUTION_TIME_BUCKETS: [f64; 20] = [
    0.002, 0.004, 0.008, 0.015, 0.02, 0.025, 0.03, 0.035, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09, 0.10,
    0.125, 0.15, 0.20, 0.40, 0.80,
];

pub static TXN_TOTAL_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_txn_total_seconds",
        "Execution time per user transaction",
        TRANSACTION_EXECUTION_TIME_BUCKETS.to_vec()
    )
    .unwrap()
});

const TRANSACTION_VALIDATION_TIME_BUCKETS: [f64; 14] = [
    0.0005, 0.001, 0.0015, 0.002, 0.0025, 0.003, 0.0035, 0.004, 0.0045, 0.005, 0.006, 0.008, 0.01,
    0.015,
];

pub static TXN_VALIDATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_txn_validation_seconds",
        "Validation time per user transaction",
        TRANSACTION_VALIDATION_TIME_BUCKETS.to_vec()
    )
    .unwrap()
});

const TXN_GAS_USAGE_BUCKETS: [f64; 22] = [
    150.0, 300.0, 450.0, 600.0, 750.0, 900.0, 1050.0, 1200.0, 1350.0, 1600.0, 1900.0, 2200.0,
    2800.0, 3600.0, 4600.0, 5800.0, 7100.0, 8800.0, 10700.0, 13000.0, 16500.0, 20000.0,
];

pub static TXN_GAS_USAGE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_vm_txn_gas_usage",
        "Gas used per transaction",
        TXN_GAS_USAGE_BUCKETS.to_vec()
    )
    .unwrap()
});

make_local_histogram_vec!(
    pub,
    TIMER,
    "aptos_vm_timer_seconds",
    "Various timers for performance analysis.",
    &["name"],
    exponential_buckets(/*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32).unwrap(),
);
