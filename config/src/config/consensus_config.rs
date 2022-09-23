// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::config::SafetyRulesConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConsensusConfig {
    pub max_block_txns: u64,
    pub max_block_bytes: u64,
    pub max_pruned_blocks_in_mem: usize,
    // Timeout for consensus to get an ack from mempool for executed transactions (in milliseconds)
    pub mempool_executed_txn_timeout_ms: u64,
    // Timeout for consensus to pull transactions from mempool and get a response (in milliseconds)
    pub mempool_txn_pull_timeout_ms: u64,
    pub round_initial_timeout_ms: u64,
    pub round_timeout_backoff_exponent_base: f64,
    pub round_timeout_backoff_max_exponent: usize,
    pub safety_rules: SafetyRulesConfig,
    // Only sync committed transactions but not vote for any pending blocks. This is useful when
    // validators coordinate on the latest version to apply a manual transaction.
    pub sync_only: bool,
    pub channel_size: usize,
    // When false, use the Direct Mempool Quorum Store
    pub use_quorum_store: bool,
    pub quorum_store_pull_timeout_ms: u64,
    // Decides how long the leader waits before proposing empty block if there's no txns in mempool
    // the period = (poll_count - 1) * 30ms
    pub quorum_store_poll_count: u64,
    pub intra_consensus_channel_buffer_size: usize,
}

impl Default for ConsensusConfig {
    fn default() -> ConsensusConfig {
        ConsensusConfig {
            max_block_txns: 2500,
            // defaulting to under 0.5s to broadcast the proposal to 100 validators
            // over 1gbps link
            max_block_bytes: 600 * 1024, // 600 KB
            max_pruned_blocks_in_mem: 100,
            mempool_executed_txn_timeout_ms: 1000,
            mempool_txn_pull_timeout_ms: 1000,
            round_initial_timeout_ms: 1500,
            // 1.2^6 ~= 3
            // Timeout goes from initial_timeout to initial_timeout*3 in 6 steps
            round_timeout_backoff_exponent_base: 1.2,
            round_timeout_backoff_max_exponent: 6,
            safety_rules: SafetyRulesConfig::default(),
            sync_only: false,
            channel_size: 30, // hard-coded
            use_quorum_store: false,

            quorum_store_pull_timeout_ms: 1000,
            quorum_store_poll_count: 10,
            intra_consensus_channel_buffer_size: 10,
        }
    }
}

impl ConsensusConfig {
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.safety_rules.set_data_dir(data_dir);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = ConsensusConfig::default();
        let s = serde_yaml::to_string(&config).unwrap();

        serde_yaml::from_str::<ConsensusConfig>(&s).unwrap();
    }
}
