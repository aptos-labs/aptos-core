// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{
        increment_compression_byte_count, increment_compression_error,
        start_compression_operation_timer, CompressionClient, COMPRESS, COMPRESSED_BYTES,
        DECOMPRESS, RAW_BYTES,
    },
    Error::{CompressionError, DecompressionError},
};
use aptos_logger::prelude::*;
use lz4::block::CompressionMode;
use thiserror::Error;

/// This crate provides a simple library interface for data compression.
/// It is useful for compressing large data chunks that are
/// sent across the network (e.g., by state sync and consensus).
/// Internally, it uses LZ4 to compress the data.
/// See <https://github.com/10xGenomics/lz4-rs> for more information.
///
/// Note: the crate also exposes some basic compression metrics
/// that can be used to track the cumulative compression ratio
/// and compression/decompression durations during the runtime.
pub mod metrics;
#[cfg(test)]
mod tests;

// Useful data size constants
pub const KIB: usize = 1024;
pub const MIB: usize = 1024 * 1024;

// The acceleration parameter to use for FAST compression mode.
// This was determined anecdotally.
const FAST_ACCELERATION_PARAMETER: i32 = 1;

// The acceleration parameters to use for various HIGH compression modes.
// These were determined anecdotally.
const LOW_VARIABLE_COMPRESSION_PARAMETER: i32 = 1;
const MEDIUM_VARIABLE_COMPRESSION_PARAMETER: i32 = 4;
const HIGH_VARIABLE_COMPRESSION_PARAMETER: i32 = 8;
const MAX_VARIABLE_COMPRESSION_PARAMETER: i32 = 12;

/// A useful wrapper for representing compressed data
pub type CompressedData = Vec<u8>;

/// An error type for capturing compression/decompression failures
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Encountered a compression error! Error: {0}")]
    CompressionError(String),
    #[error("Encountered a decompression error! Error: {0}")]
    DecompressionError(String),
}

/// Compresses the raw data stream using the default compression mode
pub fn compress(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
) -> Result<CompressedData, Error> {
    // Use FAST compression mode (this seems to be good enough for most use cases)
    let compression_mode = CompressionMode::FAST(FAST_ACCELERATION_PARAMETER);
    compress_using_mode(raw_data, client, max_bytes, compression_mode)
}

/// Compresses the raw data stream using variable compression
/// based on the raw data size. Note: the compression modes were
/// calibrated using a max TPS workload.
pub fn compress_with_variable_compression(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
) -> Result<CompressedData, Error> {
    // Determine the compression mode
    let data_length = raw_data.len();
    let compression_mode = if data_length < 20 * KIB {
        CompressionMode::FAST(FAST_ACCELERATION_PARAMETER)
    } else if data_length < 100 * KIB {
        CompressionMode::HIGHCOMPRESSION(LOW_VARIABLE_COMPRESSION_PARAMETER)
    } else if data_length < 500 * KIB {
        CompressionMode::HIGHCOMPRESSION(MEDIUM_VARIABLE_COMPRESSION_PARAMETER)
    } else if data_length < MIB {
        CompressionMode::HIGHCOMPRESSION(HIGH_VARIABLE_COMPRESSION_PARAMETER)
    } else {
        CompressionMode::HIGHCOMPRESSION(MAX_VARIABLE_COMPRESSION_PARAMETER)
    };

    // Compress the data
    compress_using_mode(raw_data, client, max_bytes, compression_mode)
}

/// Compresses the raw data stream using the specified compression mode
fn compress_using_mode(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
    compression_mode: CompressionMode,
) -> Result<CompressedData, Error> {
    // Ensure that the raw data size is not greater than the max byte limit
    if raw_data.len() > max_bytes {
        increment_compression_error(COMPRESS, client);
        return Err(CompressionError(format!(
            "Uncompressed data size greater than maximum size: {}, max: {}",
            raw_data.len(),
            max_bytes
        )));
    }

    // Start the compression timer
    let timer = start_compression_operation_timer(COMPRESS, client.clone());

    // Compress the data
    let compressed_data = match lz4::block::compress(&raw_data, Some(compression_mode), true) {
        Ok(compressed_data) => compressed_data,
        Err(error) => {
            increment_compression_error(COMPRESS, client);
            return Err(CompressionError(format!(
                "Failed to compress the data: {}",
                error
            )));
        },
    };

    // Ensure that the compressed data size is not greater than the max byte
    // limit. This can happen in the case of uncompressible data, where the
    // compressed data is larger than the uncompressed data.
    if compressed_data.len() > max_bytes {
        increment_compression_error(COMPRESS, client);
        return Err(CompressionError(format!(
            "Compressed data size greater than maximum size: {}, max: {}",
            compressed_data.len(),
            max_bytes
        )));
    }

    // Stop the timer and update the metrics
    let compression_duration = timer.stop_and_record();
    increment_compression_byte_count(COMPRESS, RAW_BYTES, client.clone(), raw_data.len() as u64);
    increment_compression_byte_count(
        COMPRESS,
        COMPRESSED_BYTES,
        client,
        compressed_data.len() as u64,
    );

    // Log the relative data compression statistics
    let relative_data_size = calculate_relative_size(&raw_data, &compressed_data);
    trace!(
        "Compressed {} bytes to {} bytes ({} %) in {} seconds.",
        raw_data.len(),
        compressed_data.len(),
        relative_data_size,
        compression_duration
    );

    Ok(compressed_data)
}

/// Decompresses the compressed data stream
pub fn decompress(
    compressed_data: &CompressedData,
    client: CompressionClient,
    max_size: usize,
) -> Result<Vec<u8>, Error> {
    // Check the size of the data and initialize raw_data
    let decompressed_size = match get_decompressed_size(compressed_data, max_size) {
        Ok(size) => size,
        Err(error) => {
            increment_compression_error(DECOMPRESS, client);
            return Err(DecompressionError(format!(
                "Failed to get decompressed size: {}",
                error
            )));
        },
    };
    let mut raw_data = vec![0u8; decompressed_size];

    // Start the decompression timer
    let timer = start_compression_operation_timer(DECOMPRESS, client.clone());

    // Decompress the data
    if let Err(error) = lz4::block::decompress_to_buffer(compressed_data, None, &mut raw_data) {
        increment_compression_error(DECOMPRESS, client);
        return Err(DecompressionError(format!(
            "Failed to decompress the data: {}",
            error
        )));
    };

    // Stop the timer and update the metrics
    let decompression_duration = timer.stop_and_record();
    increment_compression_byte_count(DECOMPRESS, RAW_BYTES, client.clone(), raw_data.len() as u64);
    increment_compression_byte_count(
        DECOMPRESS,
        COMPRESSED_BYTES,
        client,
        compressed_data.len() as u64,
    );

    // Log the relative data decompression statistics
    let relative_data_size = calculate_relative_size(compressed_data, &raw_data);
    trace!(
        "Decompressed {} bytes to {} bytes ({} %) in {} seconds.",
        compressed_data.len(),
        raw_data.len(),
        relative_data_size,
        decompression_duration
    );

    Ok(raw_data)
}

/// Derived from the lz4-rs crate, which prepends the compressed payload
/// with the original data size as i32.
/// See: https://github.com/10XGenomics/lz4-rs/blob/0abc0a52af1f6010f9a57640b1dc8eb8d2d697aa/src/block/mod.rs#L162
fn get_decompressed_size(
    compressed_data: &CompressedData,
    max_size: usize,
) -> Result<usize, Error> {
    // Ensure that the compressed data is at least 4 bytes long
    if compressed_data.len() < 4 {
        return Err(DecompressionError(format!(
            "Compressed data must be at least 4 bytes long! Got: {}",
            compressed_data.len()
        )));
    }

    // Parse the size prefix
    let size = (compressed_data[0] as i32)
        | (compressed_data[1] as i32) << 8
        | (compressed_data[2] as i32) << 16
        | (compressed_data[3] as i32) << 24;
    if size < 0 {
        return Err(DecompressionError(format!(
            "Parsed size prefix in buffer must not be negative! Got: {}",
            size
        )));
    }

    // Ensure that the size is not greater than the max size limit
    let size = size as usize;
    if size > max_size {
        return Err(DecompressionError(format!(
            "Parsed size prefix in buffer is too big: {} > {}",
            size, max_size
        )));
    }

    Ok(size)
}

/// Calculates the relative size (%) between the input and output after a
/// compression/decompression operation, i.e., (output / input) * 100.
fn calculate_relative_size(input: &[u8], output: &[u8]) -> f64 {
    // Calculate the relative sizes
    let input_len = input.len();
    let output_len = output.len();

    // Ensure the lengths aren't zero
    if input_len == 0 || output_len == 0 {
        return 0.0;
    }

    // Calculate the relative size
    (output_len as f64 / input_len as f64) * 100.0
}
