// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Helper functions for prefix consensus smoke tests

use anyhow::{bail, Result};
use aptos_crypto::HashValue;
use aptos_sdk::types::PeerId;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub input: Vec<String>,  // Hex-encoded hashes (party's input vector)
    pub v_low: Vec<String>,  // Hex-encoded hashes
    pub v_high: Vec<String>, // Hex-encoded hashes
}

impl PrefixConsensusOutputFile {
    /// Parse input hashes from hex strings
    pub fn input_hashes(&self) -> Result<Vec<HashValue>> {
        self.input
            .iter()
            .map(|hex| HashValue::from_hex(hex).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e)))
            .collect()
    }

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

/// Wait for all validators to write to the shared prefix consensus output file
///
/// Polls the filesystem for the shared output file in /tmp/, waiting until all expected
/// validator outputs appear in the file or timing out.
///
/// # Arguments
/// * `_swarm_dir` - The swarm's root directory (unused, kept for API compatibility)
/// * `peer_ids` - List of validator peer IDs to wait for
/// * `timeout` - Maximum time to wait for all outputs
///
/// # Returns
/// Vector of parsed outputs, or error if timeout
pub async fn wait_for_prefix_consensus_outputs(
    _swarm_dir: &std::path::Path,
    peer_ids: &[PeerId],
    timeout: Duration,
) -> Result<Vec<PrefixConsensusOutputFile>> {
    let start_time = std::time::Instant::now();
    let shared_file = "/tmp/prefix_consensus_results.jsonl";

    loop {
        // Try to read the shared file
        if let Ok(contents) = std::fs::read_to_string(shared_file) {
            // Parse newline-delimited JSON
            let mut outputs = Vec::new();
            for line in contents.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<PrefixConsensusOutputFile>(line) {
                    Ok(output) => outputs.push(output),
                    Err(e) => {
                        eprintln!("Failed to parse line in {}: {}", shared_file, e);
                    }
                }
            }

            // Check if we have all expected outputs
            if outputs.len() >= peer_ids.len() {
                return Ok(outputs);
            }
        }

        // Check timeout
        if start_time.elapsed() >= timeout {
            let count = std::fs::read_to_string(shared_file)
                .ok()
                .map(|c| c.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0);

            bail!(
                "Timeout waiting for prefix consensus outputs. Got {} out of {} validators in {}",
                count,
                peer_ids.len(),
                shared_file
            );
        }

        // Sleep before next poll
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Clean up the shared output file from a previous test run
pub fn cleanup_output_files(_swarm_dir: &std::path::Path, _peer_ids: &[PeerId]) {
    let shared_file = "/tmp/prefix_consensus_results.jsonl";
    let _ = std::fs::remove_file(shared_file);
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
