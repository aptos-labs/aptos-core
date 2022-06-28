// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::config::SafetyRulesConfig;
use aptos_types::{account_address::AccountAddress, block_info::Round};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConsensusConfig {
    pub contiguous_rounds: u32,
    pub max_block_size: u64,
    pub max_pruned_blocks_in_mem: usize,
    // Timeout for consensus to get an ack from mempool for executed transactions (in milliseconds)
    pub mempool_executed_txn_timeout_ms: u64,
    // Timeout for consensus to pull transactions from mempool and get a response (in milliseconds)
    pub mempool_txn_pull_timeout_ms: u64,
    pub round_initial_timeout_ms: u64,
    pub proposer_type: ConsensusProposerType,
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
    pub max_failed_authors_to_store: usize,
}

impl Default for ConsensusConfig {
    fn default() -> ConsensusConfig {
        ConsensusConfig {
            contiguous_rounds: 2,
            max_block_size: 3000,
            max_pruned_blocks_in_mem: 100,
            mempool_executed_txn_timeout_ms: 1000,
            mempool_txn_pull_timeout_ms: 1000,
            round_initial_timeout_ms: 1000,
            proposer_type: ConsensusProposerType::LeaderReputation(
                LeaderReputationType::ProposerAndVoter(ProposerAndVoterConfig {
                    active_weight: 1000,
                    inactive_weight: 10,
                    failed_weight: 1,
                    failure_threshold_percent: 10, // = 10%
                    // In each round we get stastics for the single proposer
                    // and large number of validators. So the window for
                    // the proposers needs to be significantly larger
                    // to have enough useful statistics.
                    proposer_window_num_validators_multiplier: 10,
                    voter_window_num_validators_multiplier: 1,
                }),
            ),
            safety_rules: SafetyRulesConfig::default(),
            sync_only: false,
            channel_size: 30, // hard-coded
            use_quorum_store: false,
            quorum_store_pull_timeout_ms: 1000,
            quorum_store_poll_count: 20,
            intra_consensus_channel_buffer_size: 10,
            max_failed_authors_to_store: 10,
        }
    }
}

impl ConsensusConfig {
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.safety_rules.set_data_dir(data_dir);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work.
pub enum ConsensusProposerType {
    // Choose the smallest PeerId as the proposer
    FixedProposer,
    // Round robin rotation of proposers
    RotatingProposer,
    // Committed history based proposer election
    LeaderReputation(LeaderReputationType),
    // Pre-specified proposers for each round,
    // or default proposer if round proposer not
    // specified
    RoundProposer(HashMap<Round, AccountAddress>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LeaderReputationType {
    // Proposer election based on whether nodes were active
    ActiveInactive(ActiveInactiveConfig),
    // Proposer election based on whether nodes succeeded or failed
    // their proposer election rounds, and whether they voted.
    ProposerAndVoter(ProposerAndVoterConfig),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveInactiveConfig {
    pub active_weight: u64,
    pub inactive_weight: u64,
    pub window_num_validators_multiplier: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposerAndVoterConfig {
    // Selection weight for active validators with proposer failures below threshold
    pub active_weight: u64,
    // Selection weight for inactive validators with proposer failures below threshold
    pub inactive_weight: u64,
    // Selection weight for validators with proposer failures above threshold
    pub failed_weight: u64,
    // Thresholed of failures in the rounds validator was selected to be proposer
    // integer values representing percentages, i.e. 12 is 12%.
    pub failure_threshold_percent: u32,
    // Window into history considered for proposer statistics, multiplier
    // on top of number of validators
    pub proposer_window_num_validators_multiplier: usize,
    // Window into history considered for votre statistics, multiplier
    // on top of number of validators
    pub voter_window_num_validators_multiplier: usize,
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
