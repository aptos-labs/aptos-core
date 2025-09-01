// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_avg_counter_vec, register_histogram, register_histogram_vec,
    register_int_counter, register_int_counter_vec, register_int_gauge, Histogram, HistogramVec,
    IntCounter, IntCounterVec, IntGauge,
};
use aptos_mvhashmap::BlockStateStats;
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

pub static BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_executor_block_executor_inner_execute_block_seconds",
        // metric description
        "The time spent in the most-inner part of executing a block of transactions, \
        i.e. for BlockSTM that is how long parallel or sequential execution took.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

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
        "Histogram for different approx block output sizes - used for evaluating block output limit.",
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

pub static BLOCK_VIEW_DISTINCT_KEYS: Lazy<HistogramVec> = Lazy::new(|| {
    register_avg_counter_vec(
        "aptos_execution_block_view_distinct_keys",
        "Size (number of keys) ",
        &["mode", "object_type"],
    )
});

pub static BLOCK_VIEW_BASE_VALUES_MEMORY_USAGE: Lazy<HistogramVec> = Lazy::new(|| {
    register_avg_counter_vec(
        "aptos_execution_block_view_base_values_memory_usage",
        "Memory usage (in bytes) for base values",
        &["mode", "object_type"],
    )
});

fn observe_gas(_counter: &Lazy<HistogramVec>, _mode_str: &str, _fee_statement: &FeeStatement) {}

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

pub(crate) fn update_state_counters(block_state_stats: BlockStateStats, is_parallel: bool) {
    let mode_str = if is_parallel {
        Mode::PARALLEL
    } else {
        Mode::SEQUENTIAL
    };
}

pub static GLOBAL_MODULE_CACHE_SIZE_IN_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_module_cache_size_in_bytes",
        "Sum of sizes of all serialized modules stored in global module cache"
    )
    .unwrap()
});

pub static GLOBAL_MODULE_CACHE_NUM_MODULES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "global_module_cache_num_modules",
        "Number of modules cached in global module cache"
    )
    .unwrap()
});

pub static GLOBAL_MODULE_CACHE_MISS_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "global_module_cache_miss_seconds",
        // metric description
        "The time spent in seconds after global module cache miss to access per-block module cache",
        time_buckets(),
    )
    .unwrap()
});

pub static STRUCT_NAME_INDEX_MAP_NUM_ENTRIES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "struct_name_index_map_num_entries",
        "Number of struct names interned and cached in execution environment"
    )
    .unwrap()
});

pub static HOT_STATE_OP_ACCUMULATOR_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        // metric name
        "aptos_hot_state_op_accumulator_counter",
        // metric description
        "Various counters for BlockHotStateOpAccumulator",
        // metric labels (dimensions)
        &["name"],
    )
    .unwrap()
});
