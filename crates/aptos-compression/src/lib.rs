// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    increment_compression_byte_count, increment_compression_error,
    start_compression_operation_timer, CompressionClient, COMPRESS, COMPRESSED_BYTES, DECOMPRESS,
    RAW_BYTES,
};
use aptos_logger::prelude::*;
use lz4::block::CompressionMode;
use std::io::{Error, ErrorKind};
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
pub mod metrics;
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
pub fn compress(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
) -> Result<CompressedData, CompressionError> {
    // Start the compression timer
    let timer = start_compression_operation_timer(COMPRESS, client.clone());

    // Compress the data
    let compression_mode = CompressionMode::FAST(ACCELERATION_PARAMETER);
    let compressed_data = match lz4::block::compress(&raw_data, Some(compression_mode), true) {
        Ok(compressed_data) => compressed_data,
        Err(error) => {
            increment_compression_error(COMPRESS, client);
            return Err(CompressionError(format!(
                "Failed to compress the data: {}",
                error
            )));
        }
    };

    if compressed_data.len() > max_bytes {
        return Err(CompressionError(format!(
            "Compressed size greater than max. size: {}, max: {}",
            compressed_data.len(),
            max_bytes
        )));
    }

    // Stop the timer and update the metrics
    let compression_duration = timer.stop_and_record();
    increment_compression_byte_count(RAW_BYTES, client.clone(), raw_data.len() as u64);
    increment_compression_byte_count(COMPRESSED_BYTES, client, compressed_data.len() as u64);

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
) -> Result<Vec<u8>, CompressionError> {
    // Start the decompression timer
    let timer = start_compression_operation_timer(DECOMPRESS, client.clone());

    // Check size of the data and initialize raw_data
    let size = match get_decompressed_size(compressed_data, max_size) {
        Ok(size) => size,
        Err(error) => {
            increment_compression_error(DECOMPRESS, client);
            return Err(CompressionError(format!(
                "Failed to get decompressed size: {}",
                error
            )));
        }
    };
    let mut raw_data = vec![0u8; size];

    // Decompress the data
    if let Err(error) = lz4::block::decompress_to_buffer(compressed_data, None, &mut raw_data) {
        increment_compression_error(DECOMPRESS, client);
        return Err(CompressionError(format!(
            "Failed to decompress the data: {}",
            error
        )));
    };

    // Stop the timer and log the relative data compression statistics
    let decompression_duration = timer.stop_and_record();
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

/// Derived from lz4-rs crate, which starts the compressed payload with the original data size as i32
/// see: https://github.com/10XGenomics/lz4-rs/blob/0abc0a52af1f6010f9a57640b1dc8eb8d2d697aa/src/block/mod.rs#L162
fn get_decompressed_size(src: &CompressedData, max_size: usize) -> std::io::Result<usize> {
    if src.len() < 4 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Source buffer must at least contain size prefix.",
        ));
    }

    let size =
        (src[0] as i32) | (src[1] as i32) << 8 | (src[2] as i32) << 16 | (src[3] as i32) << 24;

    if size < 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Parsed size prefix in buffer must not be negative.",
        ));
    }

    let size = size as usize;

    if size > max_size {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Given size parameter is too big: {} > {}", size, max_size),
        ));
    }

    Ok(size)
}

/// Calculates the relative size (%) between the input and output after a
/// compression/decompression operation, i.e., (output / input) * 100.
fn calculate_relative_size(input: &[u8], output: &[u8]) -> f64 {
    (output.len() as f64 / input.len() as f64) * 100.0
}
