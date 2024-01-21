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
    pub const STORAGE_FEE_REFUND: &'static str = "storage_refund_in_octas";
    pub const TOTAL_GAS: &'static str = "total_gas";
}

pub struct Mode;

impl Mode {
    pub const PARALLEL: &'static str = "parallel";
    pub const SEQUENTIAL: &'static str = "sequential";
}

fn time_buckets() -> std::vec::Vec<f64> {
    exponential_buckets(
        /*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30,
    )
    .unwrap()
}

fn gas_buckets() -> std::vec::Vec<f64> {
    exponential_buckets(
        /*start=*/ 1.0, /*factor=*/ 1.5, /*count=*/ 30,
    )
    .unwrap()
}

fn output_buckets() -> std::vec::Vec<f64> {
    exponential_buckets(
        /*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 30,
    )
    .unwrap()
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

/// Count of times the BlockSTM is early halted due to exceeding the per-block output size limit.
pub static EXCEED_PER_BLOCK_OUTPUT_LIMIT_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_execution_output_limit_count",
        "Count of times the BlockSTM is early halted due to exceeding the per-block output size limit",
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
        time_buckets(),
    )
    .unwrap()
});

pub static RAYON_EXECUTION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_rayon_execution_seconds",
        // metric description
        "The time spent in seconds in rayon thread pool in parallel execution",
        time_buckets(),
    )
    .unwrap()
});

pub static VM_INIT_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_vm_init_seconds",
        // metric description
        "The time spent in seconds in initializing the VM in the block executor",
        time_buckets(),
    )
    .unwrap()
});

pub static TASK_VALIDATE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_task_validate_seconds",
        // metric description
        "The time spent in task validation in Block STM",
        time_buckets(),
    )
    .unwrap()
});

pub static WORK_WITH_TASK_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_work_with_task_seconds",
        // metric description
        "The time spent in work task with scope call in Block STM",
        time_buckets(),
    )
    .unwrap()
});

pub static TASK_EXECUTE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_task_execute_seconds",
        // metric description
        "The time spent in seconds for task execution in Block STM",
        time_buckets(),
    )
    .unwrap()
});

pub static DEPENDENCY_WAIT_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_execution_dependency_wait",
        "The time spent in waiting for dependency in Block STM",
        time_buckets(),
    )
    .unwrap()
});

pub static BLOCK_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_block_gas",
        "Histogram for different block gas costs (execution, io, storage, storage fee, non-storage)",
        &["mode", "stage"],
        gas_buckets(),
    )
    .unwrap()
});

pub static EFFECTIVE_BLOCK_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_effective_block_gas",
        "Histogram for different effective block gas costs - used for evaluating block gas limit. \
        This can be different from actual gas consumed in a block, due to applied adjustements",
        &["mode"],
        gas_buckets(),
    )
    .unwrap()
});

pub static APPROX_BLOCK_OUTPUT_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_approx_block_output_size",
        "Historgram for different approx block output sizes - used for evaluting block ouptut limit.",
        &["mode"],
        output_buckets(),
    )
    .unwrap()
});

pub static TXN_GAS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_execution_txn_gas",
        "Histogram for different average txn gas costs (execution, io, storage, storage fee, non-storage)",
        &["mode", "stage"],
        gas_buckets(),
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

fn observe_gas(counter: &Lazy<HistogramVec>, mode_str: &str, fee_statement: &FeeStatement) {
    counter
        .with_label_values(&[mode_str, GasType::TOTAL_GAS])
        .observe(fee_statement.gas_used() as f64);

    counter
        .with_label_values(&[mode_str, GasType::EXECUTION_GAS])
        .observe(fee_statement.execution_gas_used() as f64);

    counter
        .with_label_values(&[mode_str, GasType::IO_GAS])
        .observe(fee_statement.io_gas_used() as f64);

    counter
        .with_label_values(&[mode_str, GasType::NON_STORAGE_GAS])
        .observe((fee_statement.execution_gas_used() + fee_statement.io_gas_used()) as f64);

    counter
        .with_label_values(&[mode_str, GasType::STORAGE_FEE])
        .observe(fee_statement.storage_fee_used() as f64);

    counter
        .with_label_values(&[mode_str, GasType::STORAGE_FEE_REFUND])
        .observe(fee_statement.storage_fee_refund() as f64);
}

pub(crate) fn update_block_gas_counters(
    accumulated_fee_statement: &FeeStatement,
    accumulated_effective_gas: u64,
    accumulated_approx_output_size: u64,
    num_committed: usize,
    is_parallel: bool,
) {
    let mode_str = if is_parallel {
        Mode::PARALLEL
    } else {
        Mode::SEQUENTIAL
    };

    observe_gas(&BLOCK_GAS, mode_str, accumulated_fee_statement);
    BLOCK_COMMITTED_TXNS
        .with_label_values(&[mode_str])
        .observe(num_committed as f64);

    EFFECTIVE_BLOCK_GAS
        .with_label_values(&[mode_str])
        .observe(accumulated_effective_gas as f64);

    APPROX_BLOCK_OUTPUT_SIZE
        .with_label_values(&[mode_str])
        .observe(accumulated_approx_output_size as f64);
}

pub(crate) fn update_txn_gas_counters(txn_fee_statements: &Vec<FeeStatement>, is_parallel: bool) {
    let mode_str = if is_parallel {
        Mode::PARALLEL
    } else {
        Mode::SEQUENTIAL
    };

    for fee_statement in txn_fee_statements {
        observe_gas(&TXN_GAS, mode_str, fee_statement);
    }
}
