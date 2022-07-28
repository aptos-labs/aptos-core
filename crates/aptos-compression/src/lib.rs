// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    increment_compression_byte_count, increment_compression_error,
    start_compression_operation_timer, COMPRESS, COMPRESSED_BYTES, DECOMPRESS, RAW_BYTES,
};
use aptos_logger::prelude::*;
use lz4::block::CompressionMode;
use thiserror::Error;

/// This crate provides a simple library interface for data compression.
/// It is useful for compressing large data chunks that are
/// sent across the network (e.g., by state sync and consensus).
/// Internally, it uses LZ4 in fast mode to compress the data.
/// See https://github.com/10xGenomics/lz4-rs for more information.
///
/// Note: the crate also exposes some basic compression metrics
/// that can be used to track the cumulative compression ratio
/// and compression/decompression durations during the runtime.
mod metrics;
#[cfg(test)]
mod tests;

/// The acceleration parameter to use for FAST compression mode.
/// This was determined anecdotally.
const ACCELERATION_PARAMETER: i32 = 1;

/// A useful wrapper for representing compressed data
pub type CompressedData = Vec<u8>;

/// An error type for capturing compression/decompression failures
#[derive(Clone, Debug, Error)]
#[error("Encountered a compression error! Error: {0}")]
pub struct CompressionError(String);

/// Compresses the raw data stream
pub fn compress(raw_data: Vec<u8>) -> Result<CompressedData, CompressionError> {
    // Start the compression timer
    let timer = start_compression_operation_timer(COMPRESS);

    // Compress the data
    let compression_mode = CompressionMode::FAST(ACCELERATION_PARAMETER);
    let compressed_data = match lz4::block::compress(&raw_data, Some(compression_mode), true) {
        Ok(compressed_data) => compressed_data,
        Err(error) => {
            increment_compression_error(COMPRESS);
            return Err(CompressionError(format!(
                "Failed to compress the data: {:?}",
                error.to_string()
            )));
        }
    };

    // Stop the timer and update the metrics
    let compression_duration = timer.stop_and_record();
    increment_compression_byte_count(RAW_BYTES, raw_data.len() as u64);
    increment_compression_byte_count(COMPRESSED_BYTES, compressed_data.len() as u64);

    // Log the relative data compression statistics
    let relative_data_size = calculate_relative_size(&raw_data, &compressed_data);
    trace!(
        "Compressed {:?} bytes to {:?} bytes ({:?} %) in {:?} seconds.",
        raw_data.len(),
        compressed_data.len(),
        relative_data_size,
        compression_duration
    );

    Ok(compressed_data)
}

/// Decompresses the compressed data stream
pub fn decompress(compressed_data: &CompressedData) -> Result<Vec<u8>, CompressionError> {
    // Start the decompression timer
    let timer = start_compression_operation_timer(DECOMPRESS);

    // Decompress the data
    let raw_data = match lz4::block::decompress(compressed_data, None) {
        Ok(raw_data) => raw_data,
        Err(error) => {
            increment_compression_error(DECOMPRESS);
            return Err(CompressionError(format!(
                "Failed to decompress the data: {:?}",
                error.to_string()
            )));
        }
    };

    // Stop the timer and log the relative data compression statistics
    let decompression_duration = timer.stop_and_record();
    let relative_data_size = calculate_relative_size(compressed_data, &raw_data);
    trace!(
        "Decompressed {:?} bytes to {:?} bytes ({:?} %) in {:?} seconds.",
        compressed_data.len(),
        raw_data.len(),
        relative_data_size,
        decompression_duration
    );

    Ok(raw_data)
}

/// Calculates the relative size (%) between the input and output after a
/// compression/decompression operation, i.e., (output / input) * 100.
fn calculate_relative_size(input: &Vec<u8>, output: &Vec<u8>) -> f64 {
    (output.len() as f64 / input.len() as f64) * 100.0
}
