// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::PepperServiceError;
use anyhow::{anyhow, ensure};

/// Attempts to read an environment variable and returns an error if it fails
pub fn read_environment_variable(variable_name: &str) -> Result<String, PepperServiceError> {
    std::env::var(variable_name).map_err(|error| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to read environment variable {}: {}",
            variable_name, error
        ))
    })
}

pub fn unhexlify_api_bytes(api_output: &str) -> anyhow::Result<Vec<u8>> {
    ensure!(api_output.len() >= 2);
    let lower = api_output.to_lowercase();
    ensure!(&lower[0..2] == "0x");
    let bytes = hex::decode(&lower[2..])
        .map_err(|e| anyhow!("unhexlify_api_bytes() failed at decoding: {e}"))?;
    Ok(bytes)
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
