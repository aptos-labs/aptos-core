// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_histogram_vec, register_int_counter,
    Histogram, HistogramVec, IntCounter,
};
use once_cell::sync::Lazy;

pub struct GasType;

impl GasType {
    pub const EXECUTION_GAS: &'static str = "execution_gas";
    pub const IO_GAS: &'static str = "io_gas";
    pub const NON_STORAGE_GAS: &'static str = "non_storage_gas";
    pub const STORAGE_FEE: &'static str = "storage_fee";
    pub const STORAGE_GAS: &'static str = "storage_gas";
    pub const TOTAL_GAS: &'static str = "total_gas";
}

/// Record the block gas during parallel execution.
pub fn observe_parallel_execution_block_gas(cost: u64, gas_type: &'static str) {
    PARALLEL_BLOCK_GAS
        .with_label_values(&[gas_type])
        .observe(cost as f64);
}

/// Record the txn gas during parallel execution.
pub fn observe_parallel_execution_txn_gas(cost: u64, gas_type: &'static str) {
    PARALLEL_TXN_GAS
        .with_label_values(&[gas_type])
        .observe(cost as f64);
}

/// Record the block gas during sequential execution.
pub fn observe_sequential_execution_block_gas(cost: u64, gas_type: &'static str) {
    SEQUENTIAL_BLOCK_GAS
        .with_label_values(&[gas_type])
        .observe(cost as f64);
}

/// Record the txn gas during sequential execution.
pub fn observe_sequential_execution_txn_gas(cost: u64, gas_type: &'static str) {
    SEQUENTIAL_TXN_GAS
        .with_label_values(&[gas_type])
        .observe(cost as f64);
}

/// Count of times the module publishing fallback was triggered in parallel execution.
pub static MODULE_PUBLISHING_FALLBACK_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_execution_module_publishing_fallback_count",
        "Count times module was read and written in parallel execution (sequential fallback)"
    )
    .unwrap()
});

/// Count of speculative transaction re-executions due to a failed validation.
pub static SPECULATIVE_ABORT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_execution_speculative_abort_count",
        "Number of speculative aborts in parallel execution (leading to re-execution)"
    )
    .unwrap()
});

/// Count of times the BlockSTM is early halted due to exceeding the per-block gas limit.
pub static PARALLEL_EXCEED_PER_BLOCK_GAS_LIMIT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_execution_par_gas_limit_count",
        "Count of times the BlockSTM is early halted due to exceeding the per-block gas limit"
    )
    .unwrap()
});

/// Count of times the sequential execution is early halted due to exceeding the per-block gas limit.
pub static SEQUENTIAL_EXCEED_PER_BLOCK_GAS_LIMIT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_execution_seq_gas_limit_count",
        "Count of times the sequential execution is early halted due to exceeding the per-block gas limit"
    )
    .unwrap()
});

pub static PARALLEL_EXECUTION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_parallel_execution_seconds",
        // metric description
        "The time spent in seconds in parallel execution",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static RAYON_EXECUTION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_rayon_execution_seconds",
        // metric description
        "The time spent in seconds in rayon thread pool in parallel execution",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static VM_INIT_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_vm_init_seconds",
        // metric description
        "The time spent in seconds in initializing the VM in the block executor",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static TASK_VALIDATE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_task_validate_seconds",
        // metric description
        "The time spent in task validation in Block STM",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static WORK_WITH_TASK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_work_with_task_seconds",
        // metric description
        "The time spent in work task with scope call in Block STM",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static TASK_EXECUTE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_task_execute_seconds",
        // metric description
        "The time spent in seconds for task execution in Block STM",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static GET_NEXT_TASK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_get_next_task_seconds",
        // metric description
        "The time spent in seconds for getting next task from the scheduler",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DEPENDENCY_WAIT_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_execution_dependency_wait",
        "The time spent in waiting for dependency in Block STM",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static PARALLEL_BLOCK_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_parallel_block_gas",
        "Histogram for different block gas costs (execution, io, storage, storage fee, non-storage) during parallel execution",
        &["stage"]
    )
    .unwrap()
});

pub static PARALLEL_TXN_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_parallel_txn_gas",
        "Histogram for different average txn gas costs (execution, io, storage, storage fee, non-storage) during parallel execution",
        &["stage"]
    )
    .unwrap()
});

pub static SEQUENTIAL_BLOCK_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_sequential_block_gas",
        "Histogram for different block gas costs (execution, io, storage, storage fee, non-storage) during sequential execution",
        &["stage"]
    )
    .unwrap()
});

pub static SEQUENTIAL_TXN_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_sequential_txn_gas",
        "Histogram for different average txn gas costs (execution, io, storage, storage fee, non-storage) during sequential execution",
        &["stage"]
    )
    .unwrap()
});

pub static PARALLEL_BLOCK_COMMITTED_TXNS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_execution_par_block_committed_txns",
        "The per-block committed txns in parallel execution (Block STM)",
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static SEQUENTIAL_BLOCK_COMMITTED_TXNS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_execution_seq_block_committed_txns",
        "The per-block committed txns in sequential execution",
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});
