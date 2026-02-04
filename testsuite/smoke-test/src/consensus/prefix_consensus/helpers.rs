// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for prefix consensus smoke tests

use anyhow::{bail, Result};
use aptos_crypto::HashValue;
use aptos_sdk::types::PeerId;
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

/// Generate deterministic test hashes
///
/// Creates a vector of HashValues using SHA3-256 of sequential integers.
/// This matches the pattern used in prefix consensus unit tests.
pub fn generate_test_hashes(count: usize) -> Vec<HashValue> {
    (1..=count)
        .map(|i| HashValue::sha3_256_of(&(i as u64).to_le_bytes()))
        .collect()
}

/// Output structure matching what validators write to files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixConsensusOutputFile {
    pub party_id: String,
    pub epoch: u64,
    pub v_low: Vec<String>,  // Hex-encoded hashes
    pub v_high: Vec<String>, // Hex-encoded hashes
}

impl PrefixConsensusOutputFile {
    /// Parse v_low hashes from hex strings
    pub fn v_low_hashes(&self) -> Result<Vec<HashValue>> {
        self.v_low
            .iter()
            .map(|hex| HashValue::from_hex(hex).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e)))
            .collect()
    }

    /// Parse v_high hashes from hex strings
    pub fn v_high_hashes(&self) -> Result<Vec<HashValue>> {
        self.v_high
            .iter()
            .map(|hex| HashValue::from_hex(hex).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e)))
            .collect()
    }
}

/// Wait for all validators to write their prefix consensus output files
///
/// Polls the filesystem for output files from all validators, returning when
/// all files exist or timing out if they don't appear in time.
///
/// # Arguments
/// * `peer_ids` - List of validator peer IDs to wait for
/// * `timeout` - Maximum time to wait for all outputs
///
/// # Returns
/// Vector of parsed outputs in same order as peer_ids, or error if timeout
pub async fn wait_for_prefix_consensus_outputs(
    peer_ids: &[PeerId],
    timeout: Duration,
) -> Result<Vec<PrefixConsensusOutputFile>> {
    let start_time = std::time::Instant::now();

    loop {
        let mut all_outputs = Vec::new();
        let mut all_exist = true;

        for peer_id in peer_ids {
            let output_file = format!("/tmp/prefix_consensus_output_{}.json", peer_id);

            if let Ok(contents) = std::fs::read_to_string(&output_file) {
                match serde_json::from_str::<PrefixConsensusOutputFile>(&contents) {
                    Ok(output) => {
                        all_outputs.push(output);
                        continue;
                    }
                    Err(e) => {
                        // File exists but invalid JSON - might be mid-write, continue polling
                        eprintln!("Output file {} has invalid JSON (might be incomplete): {}", output_file, e);
                    }
                }
            }

            // File doesn't exist or has invalid JSON
            all_exist = false;
            break;
        }

        if all_exist && all_outputs.len() == peer_ids.len() {
            return Ok(all_outputs);
        }

        // Check timeout
        if start_time.elapsed() >= timeout {
            let missing: Vec<_> = peer_ids
                .iter()
                .filter(|peer_id| {
                    let output_file = format!("/tmp/prefix_consensus_output_{}.json", peer_id);
                    !Path::new(&output_file).exists()
                })
                .collect();

            bail!(
                "Timeout waiting for prefix consensus outputs. Missing {} out of {} validators: {:?}",
                missing.len(),
                peer_ids.len(),
                missing
            );
        }

        // Sleep before next poll
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Clean up output files from a previous test run
pub fn cleanup_output_files(peer_ids: &[PeerId]) {
    for peer_id in peer_ids {
        let output_file = format!("/tmp/prefix_consensus_output_{}.json", peer_id);
        let _ = std::fs::remove_file(&output_file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_hashes() {
        let hashes = generate_test_hashes(3);
        assert_eq!(hashes.len(), 3);

        // Hashes should be deterministic
        let hashes2 = generate_test_hashes(3);
        assert_eq!(hashes, hashes2);

        // Different counts should produce different first elements when extended
        let hashes_5 = generate_test_hashes(5);
        assert_eq!(hashes[0], hashes_5[0]); // First 3 should match
        assert_eq!(hashes[1], hashes_5[1]);
        assert_eq!(hashes[2], hashes_5[2]);
    }
}
