// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::CompressionClient,
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
pub mod client;
mod metrics;
#[cfg(test)]
mod tests;

// Useful data compression constants. These were determined anecdotally.
pub const MAX_SIZE_FOR_FAST_COMPRESSION: usize = 1024 * 1024; // 1 MiBi
pub const MAX_SIZE_FOR_MEDIUM_COMPRESSION: usize = 3 * 1024 * 1024; // 3 MiBi
pub const MAX_SIZE_FOR_HIGH_COMPRESSION: usize = 5 * 1024 * 1024; // 5 MiBi

// The parameter to use for fast compression (determined anecdotally)
const FAST_ACCELERATION_PARAMETER: i32 = 1;

// The parameters to use for high compression (determined anecdotally)
const MED_COMPRESSION_PARAMETER: i32 = 3;
const HIGH_COMPRESSION_PARAMETER: i32 = 9;
const MAX_COMPRESSION_PARAMETER: i32 = 12;

/// A useful wrapper for representing compressed data
pub type CompressedData = Vec<u8>;

/// An error type for capturing compression/decompression failures
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("Encountered a compression error! Error: {0}")]
    CompressionError(String),
    #[error("Encountered a decompression error! Error: {0}")]
    DecompressionError(String),
}

/// Compresses the raw data using the default compression mode
/// (i.e., FAST compression), which is good enough for most use cases.
pub fn compress(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
) -> Result<CompressedData, Error> {
    compress_using_mode(raw_data, client, max_bytes, fast_compression_mode())
}

/// Compresses the raw data using variable compression
/// based on the raw data size.
pub fn compress_with_variable_compression(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
) -> Result<CompressedData, Error> {
    let compression_mode = determine_compression_mode(&raw_data);
    compress_using_mode(raw_data, client, max_bytes, compression_mode)
}

/// Determines the compression mode based on the raw data size
fn determine_compression_mode(_raw_data: &[u8]) -> CompressionMode {
    fast_compression_mode()

    /*
    let data_length = raw_data.len();
    if data_length <= MAX_SIZE_FOR_FAST_COMPRESSION {
        fast_compression_mode()
    } else if data_length <= MAX_SIZE_FOR_MEDIUM_COMPRESSION {
        medium_compression_mode()
    } else if data_length <= MAX_SIZE_FOR_HIGH_COMPRESSION {
        high_compression_mode()
    } else {
        max_compression_mode()
    }*/
}

/// Compresses the raw data using the specified compression mode
fn compress_using_mode(
    raw_data: Vec<u8>,
    client: CompressionClient,
    max_bytes: usize,
    compression_mode: CompressionMode,
) -> Result<CompressedData, Error> {
    // Ensure that the raw data size is not greater than the max byte limit
    if raw_data.len() > max_bytes {
        let error_string = format!(
            "Raw data size greater than maximum size: {}, max: {}",
            raw_data.len(),
            max_bytes
        );
        return create_compression_error(&client, error_string);
    }

    // Start the compression timer
    let timer = metrics::start_compression_operation_timer(&client);

    // Compress the data
    let compressed_data = match lz4::block::compress(&raw_data, Some(compression_mode), true) {
        Ok(compressed_data) => compressed_data,
        Err(error) => {
            let error_string = format!("Failed to compress the data: {}", error);
            return create_compression_error(&client, error_string);
        },
    };

    // Ensure that the compressed data size is not greater than the max byte
    // limit. This can happen in the case of uncompressible data, where the
    // compressed data is larger than the uncompressed data.
    if compressed_data.len() > max_bytes {
        let error_string = format!(
            "Compressed data size greater than maximum size: {}, max: {}",
            compressed_data.len(),
            max_bytes
        );
        return create_compression_error(&client, error_string);
    }

    // Stop the timer and update the metrics
    timer.stop_and_record();
    metrics::update_compression_metrics(&client, &raw_data, &compressed_data);

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
            let error_string = format!("Failed to get decompressed size: {}", error);
            return create_decompression_error(&client, error_string);
        },
    };
    let mut raw_data = vec![0u8; decompressed_size];

    // Start the decompression timer
    let timer = metrics::start_decompression_operation_timer(&client);

    // Decompress the data
    if let Err(error) = lz4::block::decompress_to_buffer(compressed_data, None, &mut raw_data) {
        let error_string = format!("Failed to decompress the data: {}", error);
        return create_decompression_error(&client, error_string);
    };

    // Stop the timer and update the metrics
    timer.stop_and_record();
    metrics::update_decompression_metrics(&client, compressed_data, &raw_data);

    Ok(raw_data)
}

/// A simple utility function that wraps the given error string in a compression error
fn create_compression_error(
    client: &CompressionClient,
    error_string: String,
) -> Result<CompressedData, Error> {
    // Increment the compression error counter
    metrics::increment_compression_error(client);

    // Create and return the error
    Err(CompressionError(error_string))
}

/// A simple utility function that wraps the given error string in a decompression error
fn create_decompression_error(
    client: &CompressionClient,
    error_string: String,
) -> Result<Vec<u8>, Error> {
    // Increment the decompression error counter
    metrics::increment_decompression_error(client);

    // Create and return the error
    Err(DecompressionError(error_string))
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

/// A helpful utility function for getting the FAST compression mode
fn fast_compression_mode() -> CompressionMode {
    CompressionMode::FAST(FAST_ACCELERATION_PARAMETER)
}

/// A helpful utility function for getting the MEDIUM compression mode
fn medium_compression_mode() -> CompressionMode {
    CompressionMode::HIGHCOMPRESSION(MED_COMPRESSION_PARAMETER)
}

/// A helpful utility function for getting the HIGH compression mode
fn high_compression_mode() -> CompressionMode {
    CompressionMode::HIGHCOMPRESSION(HIGH_COMPRESSION_PARAMETER)
}

/// A helpful utility function for getting the MAX compression mode
fn max_compression_mode() -> CompressionMode {
    CompressionMode::HIGHCOMPRESSION(MAX_COMPRESSION_PARAMETER)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_determine_compression_mode() {
        // Verify that fast compression is used for tiny data
        let data = vec![0u8; 1];
        verify_selected_compression_mode(&data, fast_compression_mode());

        // Verify that fast compression is still used at the threshold
        let data = vec![0u8; MAX_SIZE_FOR_FAST_COMPRESSION];
        verify_selected_compression_mode(&data, fast_compression_mode());

        // Verify that medium compression is used for medium data
        let data = vec![0u8; MAX_SIZE_FOR_MEDIUM_COMPRESSION];
        verify_selected_compression_mode(&data, medium_compression_mode());

        // Verify that high compression is used for large data
        let data = vec![0u8; MAX_SIZE_FOR_HIGH_COMPRESSION];
        verify_selected_compression_mode(&data, high_compression_mode());

        // Verify that max compression is used for huge data
        let data = vec![0u8; MAX_SIZE_FOR_HIGH_COMPRESSION + 1];
        verify_selected_compression_mode(&data, max_compression_mode());
    }

    #[test]
    fn test_get_decompressed_size() {
        // Create some test data
        let max_compression_size = 100;

        // Verify that an error is returned when the compressed data length is zero
        let result = get_decompressed_size(&vec![0u8; 0], max_compression_size);
        assert!(result.is_err());

        // Verify that an error is returned when the compressed data length is too small
        let result = get_decompressed_size(&vec![0u8; 3], max_compression_size);
        assert!(result.is_err());

        // Verify that an error is returned when the compressed data length is too large
        let mut compressed_data = vec![0u8; max_compression_size];
        compressed_data[0] = (max_compression_size + 1) as u8;
        let result = get_decompressed_size(&compressed_data, max_compression_size);
        assert!(result.is_err());

        // Verify that the correct decompressed size is returned
        let raw_data = vec![0u8; max_compression_size];
        let compressed_data = compress(
            raw_data.clone(),
            CompressionClient::StateSync,
            max_compression_size,
        )
        .unwrap();
        let result = get_decompressed_size(&compressed_data, max_compression_size);
        assert_eq!(result.unwrap(), raw_data.len());
    }

    /// Verifies that the expected compression mode is used for the given data
    fn verify_selected_compression_mode(
        data: &Vec<u8>,
        expected_compression_mode: CompressionMode,
    ) {
        // Determinate the compression mode to use for the data
        let compression_mode = determine_compression_mode(data);

        // Verify that the expected mode is used. Note: we can't compare the
        // CompressionMode enum directly, so we compare the string representations.
        assert_eq!(
            format!("{:?}", compression_mode),
            format!("{:?}", expected_compression_mode),
        );
    }
}
