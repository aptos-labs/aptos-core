// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::PepperServiceError;
use anyhow::{anyhow, ensure};
use reqwest::Client;
use std::time::Duration;

// Timeout for client requests
const CLIENT_REQUEST_TIMEOUT_SECS: u64 = 10;

/// Creates and returns a reqwest HTTP client with a timeout
pub fn create_request_client() -> Client {
    Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(CLIENT_REQUEST_TIMEOUT_SECS))
        .build()
        .expect("Failed to build the request client!")
}

/// Attempts to read an environment variable and returns an error if it fails
pub fn read_environment_variable(variable_name: &str) -> Result<String, PepperServiceError> {
    std::env::var(variable_name).map_err(|error| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to read environment variable {}: {}",
            variable_name, error
        ))
    })
}

/// Converts a hex-encoded string (with "0x" prefix) to a byte vector
pub fn unhexlify_api_bytes(api_output: &str) -> anyhow::Result<Vec<u8>> {
    // Verify the input format
    ensure!(api_output.len() >= 2);
    let lower = api_output.to_lowercase();
    ensure!(&lower[0..2] == "0x");

    // Decode the hex string
    hex::decode(&lower[2..]).map_err(|error| {
        anyhow!(
            "unhexlify_api_bytes() failed to decode intput {}! Error: {}",
            lower,
            error
        )
    })
}

#[cfg(test)]
mod tests {
    use crate::utils::unhexlify_api_bytes;

    #[test]
    fn test_unhexlify_api_bytes() {
        // Test valid input
        assert!(unhexlify_api_bytes("0x").unwrap().is_empty());
        assert_eq!(
            vec![0x00_u8, 0x01, 0xFF],
            unhexlify_api_bytes("0x0001ff").unwrap()
        );
        assert_eq!(
            vec![0xDE_u8, 0xAD, 0xBE, 0xEF],
            unhexlify_api_bytes("0xdeadbeef").unwrap()
        );

        // Test invalid inputs
        assert!(unhexlify_api_bytes("0001ff").is_err());
        assert!(unhexlify_api_bytes("0x0001fg").is_err());
        assert!(unhexlify_api_bytes("000").is_err());
        assert!(unhexlify_api_bytes("0").is_err());
        assert!(unhexlify_api_bytes("").is_err());
    }
}
