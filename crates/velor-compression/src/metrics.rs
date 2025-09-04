// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::client::CompressionClient;
use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, HistogramVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;
use std::time::Instant;

/// Useful metric constants for compression and decompression
pub const COMPRESS: &str = "compress";
pub const DECOMPRESS: &str = "decompress";
pub const COMPRESSED_BYTES: &str = "compressed_bytes";
pub const RAW_BYTES: &str = "raw_bytes";

/// Counters for tracking the data compression ratio (i.e., total byte counts)
pub static BYTE_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_compression_byte_count",
        "Counters for tracking the data compression ratio",
        &["operation", "data_type", "client"]
    )
    .unwrap()
});

/// Counters for tracking compression/decompression errors
pub static ERROR_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_compression_error_count",
        "Counters for tracking the data compression errors",
        &["operation", "client"]
    )
    .unwrap()
});

/// Time it takes to perform a compression/decompression operation
pub static OPERATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_compression_operation_latency",
        "Time it takes to perform a compression/decompression operation",
        &["operation", "client"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

/// Increments the compression byte count based on the given data type
fn increment_compression_byte_count(
    operation: &str,
    data_type: &str,
    client: &CompressionClient,
    byte_count: u64,
) {
    BYTE_COUNTS
        .with_label_values(&[operation, data_type, client.get_label()])
        .inc_by(byte_count)
}

/// Increments the compression error count based on the given operation
pub fn increment_compression_error(client: &CompressionClient) {
    increment_error_count(COMPRESS, client)
}

/// Increments the decompression error count based on the given operation
pub fn increment_decompression_error(client: &CompressionClient) {
    increment_error_count(DECOMPRESS, client)
}

/// Increments the error count based on the given operation
fn increment_error_count(operation: &str, client: &CompressionClient) {
    ERROR_COUNTS
        .with_label_values(&[operation, client.get_label()])
        .inc()
}

/// Observes the compression operation time
pub fn observe_compression_operation_time(client: &CompressionClient, start_time: Instant) {
    observe_operation_time(COMPRESS, client, start_time)
}

/// Observes the decompression operation time
pub fn observe_decompression_operation_time(client: &CompressionClient, start_time: Instant) {
    observe_operation_time(DECOMPRESS, client, start_time)
}

/// Observes the operation time based on the given operation
fn observe_operation_time(operation: &str, client: &CompressionClient, start_time: Instant) {
    OPERATION_LATENCY
        .with_label_values(&[operation, client.get_label()])
        .observe(start_time.elapsed().as_secs_f64());
}

/// Updates the compression metrics for the given data sets
pub fn update_compression_metrics(
    client: &CompressionClient,
    raw_data: &[u8],
    compressed_data: &[u8],
) {
    update_operation_metrics(COMPRESS, client, raw_data, compressed_data);
}

/// Updates the decompression metrics for the given data sets
pub fn update_decompression_metrics(
    client: &CompressionClient,
    compressed_data: &[u8],
    raw_data: &[u8],
) {
    update_operation_metrics(DECOMPRESS, client, raw_data, compressed_data);
}

/// Updates the operation metrics based on the given data
/// (e.g., raw and compressed data sizes).
fn update_operation_metrics(
    operation: &str,
    client: &CompressionClient,
    raw_data: &[u8],
    compressed_data: &[u8],
) {
    increment_compression_byte_count(operation, RAW_BYTES, client, raw_data.len() as u64);
    increment_compression_byte_count(
        operation,
        COMPRESSED_BYTES,
        client,
        compressed_data.len() as u64,
    );
}
