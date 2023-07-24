// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, Histogram, HistogramVec, IntCounter, IntCounterVec,
};
use aptos_types::fee_statement::FeeStatement;
use once_cell::sync::Lazy;

pub struct GasType;

impl GasType {
    pub const EXECUTION_GAS: &'static str = "execution_gas";
    pub const IO_GAS: &'static str = "io_gas";
    pub const NON_STORAGE_GAS: &'static str = "non_storage_gas";
    pub const STORAGE_FEE: &'static str = "storage_in_octas";
    pub const STORAGE_GAS: &'static str = "storage_in_gas";
    pub const TOTAL_GAS: &'static str = "total_gas";
}

pub struct Mode;

impl Mode {
    pub const PARALLEL: &'static str = "parallel";
    pub const SEQUENTIAL: &'static str = "sequential";
}

/// Record the block gas during parallel execution.
fn observe_parallel_execution_block_gas(cost: u64, gas_type: &'static str) {
    BLOCK_GAS
        .with_label_values(&[Mode::PARALLEL, gas_type])
        .observe(cost as f64);
}

/// Record the txn gas during parallel execution.
fn observe_parallel_execution_txn_gas(cost: u64, gas_type: &'static str) {
    TXN_GAS
        .with_label_values(&[Mode::PARALLEL, gas_type])
        .observe(cost as f64);
}

/// Record the block gas during sequential execution.
fn observe_sequential_execution_block_gas(cost: u64, gas_type: &'static str) {
    BLOCK_GAS
        .with_label_values(&[Mode::SEQUENTIAL, gas_type])
        .observe(cost as f64);
}

/// Record the txn gas during sequential execution.
fn observe_sequential_execution_txn_gas(cost: u64, gas_type: &'static str) {
    TXN_GAS
        .with_label_values(&[Mode::SEQUENTIAL, gas_type])
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
pub static EXCEED_PER_BLOCK_GAS_LIMIT_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_execution_gas_limit_count",
        "Count of times the BlockSTM is early halted due to exceeding the per-block gas limit",
        &["mode"]
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

pub static BLOCK_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_block_gas",
        "Histogram for different block gas costs (execution, io, storage, storage fee, non-storage)",
        &["mode", "stage"]
    )
    .unwrap()
});

pub static TXN_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_txn_gas",
        "Histogram for different average txn gas costs (execution, io, storage, storage fee, non-storage)",
        &["mode", "stage"]
    )
    .unwrap()
});

pub static BLOCK_COMMITTED_TXNS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_block_committed_txns",
        "The per-block committed txns (Block STM)",
        &["mode"],
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub(crate) fn update_parallel_block_gas_counters(
    accumulated_fee_statement: &FeeStatement,
    num_committed: usize,
) {
    observe_parallel_execution_block_gas(accumulated_fee_statement.gas_used(), GasType::TOTAL_GAS);
    observe_parallel_execution_block_gas(
        accumulated_fee_statement.execution_gas_used(),
        GasType::EXECUTION_GAS,
    );
    observe_parallel_execution_block_gas(accumulated_fee_statement.io_gas_used(), GasType::IO_GAS);
    observe_parallel_execution_block_gas(
        accumulated_fee_statement.storage_gas_used(),
        GasType::STORAGE_GAS,
    );
    observe_parallel_execution_block_gas(
        accumulated_fee_statement.execution_gas_used() + accumulated_fee_statement.io_gas_used(),
        GasType::NON_STORAGE_GAS,
    );
    observe_parallel_execution_block_gas(
        accumulated_fee_statement.storage_fee_used(),
        GasType::STORAGE_FEE,
    );
    BLOCK_COMMITTED_TXNS
        .with_label_values(&[Mode::PARALLEL])
        .observe(num_committed as f64);
}

pub(crate) fn update_parallel_txn_gas_counters(txn_fee_statements: &Vec<FeeStatement>) {
    for fee_statement in txn_fee_statements {
        observe_parallel_execution_txn_gas(fee_statement.gas_used(), GasType::TOTAL_GAS);
        observe_parallel_execution_txn_gas(
            fee_statement.execution_gas_used(),
            GasType::EXECUTION_GAS,
        );
        observe_parallel_execution_txn_gas(fee_statement.io_gas_used(), GasType::IO_GAS);
        observe_parallel_execution_txn_gas(fee_statement.storage_gas_used(), GasType::STORAGE_GAS);
        observe_parallel_execution_txn_gas(
            fee_statement.execution_gas_used() + fee_statement.io_gas_used(),
            GasType::NON_STORAGE_GAS,
        );
        observe_parallel_execution_txn_gas(fee_statement.storage_fee_used(), GasType::STORAGE_FEE);
    }
}

pub(crate) fn update_sequential_block_gas_counters(
    accumulated_fee_statement: &FeeStatement,
    num_committed: usize,
) {
    observe_sequential_execution_block_gas(
        accumulated_fee_statement.gas_used(),
        GasType::TOTAL_GAS,
    );
    observe_sequential_execution_block_gas(
        accumulated_fee_statement.execution_gas_used(),
        GasType::EXECUTION_GAS,
    );
    observe_sequential_execution_block_gas(
        accumulated_fee_statement.io_gas_used(),
        GasType::IO_GAS,
    );
    observe_sequential_execution_block_gas(
        accumulated_fee_statement.storage_gas_used(),
        GasType::STORAGE_GAS,
    );
    observe_sequential_execution_block_gas(
        accumulated_fee_statement.execution_gas_used() + accumulated_fee_statement.io_gas_used(),
        GasType::NON_STORAGE_GAS,
    );
    observe_sequential_execution_block_gas(
        accumulated_fee_statement.storage_fee_used(),
        GasType::STORAGE_FEE,
    );
    BLOCK_COMMITTED_TXNS
        .with_label_values(&[Mode::PARALLEL])
        .observe(num_committed as f64);
}

pub(crate) fn update_sequential_txn_gas_counters(fee_statement: &FeeStatement) {
    observe_sequential_execution_txn_gas(fee_statement.gas_used(), GasType::TOTAL_GAS);
    observe_sequential_execution_txn_gas(
        fee_statement.execution_gas_used(),
        GasType::EXECUTION_GAS,
    );
    observe_sequential_execution_txn_gas(fee_statement.io_gas_used(), GasType::IO_GAS);
    observe_sequential_execution_txn_gas(fee_statement.storage_gas_used(), GasType::STORAGE_GAS);
    observe_sequential_execution_txn_gas(
        fee_statement.execution_gas_used() + fee_statement.io_gas_used(),
        GasType::NON_STORAGE_GAS,
    );
    observe_sequential_execution_txn_gas(fee_statement.storage_fee_used(), GasType::STORAGE_FEE);
}
