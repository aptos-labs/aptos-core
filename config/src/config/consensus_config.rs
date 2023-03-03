// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{QuorumStoreConfig, SafetyRulesConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const MAX_SENDING_BLOCK_TXNS_QUORUM_STORE_OVERRIDE: u64 = 4000;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConsensusConfig {
    pub max_sending_block_txns: u64,
    pub max_sending_block_txns_quorum_store_override: u64,
    pub max_sending_block_bytes: u64,
    pub max_sending_block_bytes_quorum_store_override: u64,
    pub max_receiving_block_txns: u64,
    pub max_receiving_block_txns_quorum_store_override: u64,
    pub max_receiving_block_bytes: u64,
    pub max_receiving_block_bytes_quorum_store_override: u64,
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
    pub quorum_store_pull_timeout_ms: u64,
    // Decides how long the leader waits before proposing empty block if there's no txns in mempool
    // the period = (poll_count - 1) * 30ms
    pub quorum_store_poll_count: u64,
    // Whether to create partial blocks when few transactions exist, or empty blocks when there is
    // pending ordering, or to wait for quorum_store_poll_count * 30ms to collect transactions for a block
    //
    // It is more efficient to execute larger blocks, as it creates less overhead. On the other hand
    // waiting increases latency (unless we are under high load that added waiting latency
    // is compensated by faster execution time). So we want to balance the two, by waiting only
    // when we are saturating the execution pipeline:
    // - if there are more pending blocks then usual in the execution pipeline,
    //   block is going to wait there anyways, so we can wait to create a bigger/more efificent block
    // - in case our node is faster than others, and we don't have many pending blocks,
    //   but we still see very large recent (pending) blocks, we know that there is demand
    //   and others are creating large blocks, so we can wait as well.
    pub wait_for_full_blocks_above_pending_blocks: usize,
    pub wait_for_full_blocks_above_recent_fill_threshold: f32,
    pub intra_consensus_channel_buffer_size: usize,
    pub quorum_store_configs: QuorumStoreConfig,
    // Used to decide if backoff is needed.
    // must match one of the CHAIN_HEALTH_WINDOW_SIZES values.
    pub window_for_chain_health: usize,
    pub chain_health_backoff: Vec<ChainHealthBackoffValues>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ChainHealthBackoffValues {
    pub backoff_if_below_participating_voting_power_percentage: usize,

    pub max_sending_block_txns_override: u64,
    pub max_sending_block_bytes_override: u64,
}

impl Default for ConsensusConfig {
    fn default() -> ConsensusConfig {
        ConsensusConfig {
            max_sending_block_txns: 2500,
            max_sending_block_txns_quorum_store_override:
                MAX_SENDING_BLOCK_TXNS_QUORUM_STORE_OVERRIDE,
            // defaulting to under 0.5s to broadcast the proposal to 100 validators
            // over 1gbps link
            max_sending_block_bytes: 600 * 1024, // 600 KB
            max_sending_block_bytes_quorum_store_override: 5 * 1024 * 1024, // 5MB
            max_receiving_block_txns: 10000,
            max_receiving_block_txns_quorum_store_override: 2
                * MAX_SENDING_BLOCK_TXNS_QUORUM_STORE_OVERRIDE,
            max_receiving_block_bytes: 3 * 1024 * 1024, // 3MB
            max_receiving_block_bytes_quorum_store_override: 6 * 1024 * 1024, // 6MB
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
            quorum_store_pull_timeout_ms: 1000,
            quorum_store_poll_count: 10,
            // disable wait_for_full until fully tested
            // We never go above 20-30 pending blocks, so this disables it
            wait_for_full_blocks_above_pending_blocks: 100,
            // Max is 1, so 1.1 disables it.
            wait_for_full_blocks_above_recent_fill_threshold: 1.1,
            intra_consensus_channel_buffer_size: 10,
            quorum_store_configs: QuorumStoreConfig::default(),

            window_for_chain_health: 100,
            chain_health_backoff: vec![
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 80,
                    max_sending_block_txns_override: 2000,
                    max_sending_block_bytes_override: 500 * 1024,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 77,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: 250 * 1024,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 75,
                    max_sending_block_txns_override: 400,
                    max_sending_block_bytes_override: 100 * 1024,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 72,
                    max_sending_block_txns_override: 200,
                    max_sending_block_bytes_override: 50 * 1024,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 69,
                    // in practice, latencies make it such that 2-4 blocks/s is max,
                    // meaning that most aggressively we limit to ~200-400 TPS
                    max_sending_block_txns_override: 100,
                    max_sending_block_bytes_override: 25 * 1024,
                },
            ],
        }
    }
}

impl ConsensusConfig {
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.safety_rules.set_data_dir(data_dir);
    }

    // TODO: This is ugly. Remove this and configs when quorum store is always the default.
    pub fn apply_quorum_store_overrides(&mut self) {
        self.max_sending_block_txns = self.max_sending_block_txns_quorum_store_override;
        self.max_sending_block_bytes = self.max_sending_block_bytes_quorum_store_override;
        self.max_receiving_block_txns = self.max_receiving_block_txns_quorum_store_override;
        self.max_receiving_block_bytes = self.max_receiving_block_bytes_quorum_store_override;
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
