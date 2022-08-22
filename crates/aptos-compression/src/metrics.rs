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

/// A simple enum for identifying clients of the compression crate. This
/// allows us to provide a runtime breakdown of compression metrics for
/// each client.
#[derive(Clone, Debug)]
pub enum CompressionClient {
    Consensus,
    Mempool,
    StateSync,
}

impl CompressionClient {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::Consensus => "consensus",
            Self::Mempool => "mempool",
            Self::StateSync => "state_sync",
        }
    }
}

/// Counters for tracking the data compression ratio (i.e., total byte counts)
pub static BYTE_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_compression_byte_count",
        "Counters for tracking the data compression ratio",
        &["data_type", "client"]
    )
    .unwrap()
});

/// Counters for tracking compression/decompression errors
pub static ERROR_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_compression_error_count",
        "Counters for tracking the data compression errors",
        &["operation", "client"]
    )
    .unwrap()
});

/// Time it takes to perform a compression/decompression operation
pub static OPERATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_compression_operation_latency",
        "Time it takes to perform a compression/decompression operation",
        &["operation", "client"]
    )
    .unwrap()
});

/// Increments the compression byte count based on the given data type
pub fn increment_compression_byte_count(
    data_type: &str,
    client: CompressionClient,
    byte_count: u64,
) {
    BYTE_COUNTS
        .with_label_values(&[data_type, client.get_label()])
        .inc_by(byte_count)
}

/// Increments the compression error count based on the given operation
pub fn increment_compression_error(operation: &str, client: CompressionClient) {
    ERROR_COUNTS
        .with_label_values(&[operation, client.get_label()])
        .inc()
}

/// Starts the timer for the compression operation using the label
pub fn start_compression_operation_timer(
    operation: &str,
    client: CompressionClient,
) -> HistogramTimer {
    OPERATION_LATENCY
        .with_label_values(&[operation, client.get_label()])
        .start_timer()
}
