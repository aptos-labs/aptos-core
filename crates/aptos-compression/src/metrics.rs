// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramTimer, HistogramVec, IntCounterVec,
};
use once_cell::sync::Lazy;

/// Useful metric constants for compression and decompression
pub const COMPRESS: &str = "compress";
pub const DECOMPRESS: &str = "decompress";
pub const COMPRESSED_BYTES: &str = "compressed_bytes";
pub const RAW_BYTES: &str = "raw_bytes";

/// Counters for tracking the data compression ratio (i.e., total byte counts)
pub static BYTE_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_compression_byte_count",
        "Counters for tracking the data compression ratio",
        &["data_type"]
    )
    .unwrap()
});

/// Counters for tracking compression/decompression errors
pub static ERROR_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_compression_error_count",
        "Counters for tracking the data compression errors",
        &["operation"]
    )
    .unwrap()
});

/// Time it takes to perform a compression/decompression operation
pub static OPERATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_compression_operation_latency",
        "Time it takes to perform a compression/decompression operation",
        &["operation"]
    )
    .unwrap()
});

/// Increments the compression byte count based on the given data type
pub fn increment_compression_byte_count(data_type: &str, byte_count: u64) {
    BYTE_COUNTS
        .with_label_values(&[data_type])
        .inc_by(byte_count)
}

/// Increments the compression error count based on the given operation
pub fn increment_compression_error(operation: &str) {
    ERROR_COUNTS.with_label_values(&[operation]).inc()
}

/// Starts the timer for the compression operation using the label
pub fn start_compression_operation_timer(operation: &str) -> HistogramTimer {
    OPERATION_LATENCY
        .with_label_values(&[operation])
        .start_timer()
}
