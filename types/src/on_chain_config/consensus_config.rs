// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block_info::Round, on_chain_config::OnChainConfig};
use anyhow::{format_err, Result};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The on-chain consensus config, in order to be able to add fields, we use enum to wrap the actual struct.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainConsensusConfig {
    V1(ConsensusConfigV1),
    V2(ConsensusConfigV1),
}

/// The public interface that exposes all values with safe fallback.
impl OnChainConsensusConfig {
    /// The number of recent rounds that don't count into reputations.
    pub fn leader_reputation_exclude_round(&self) -> u64 {
        match &self {
            OnChainConsensusConfig::V1(config) | OnChainConsensusConfig::V2(config) => {
                config.exclude_round
            },
        }
    }

    /// Decouple execution from consensus or not.
    pub fn decoupled_execution(&self) -> bool {
        true
    }

    // Trim the list of failed authors from immediatelly preceeding rounds
    // to this max size.
    pub fn max_failed_authors_to_store(&self) -> usize {
        match &self {
            OnChainConsensusConfig::V1(config) | OnChainConsensusConfig::V2(config) => {
                config.max_failed_authors_to_store
            },
        }
    }

    // Type and configuration used for proposer election.
    pub fn proposer_election_type(&self) -> &ProposerElectionType {
        match &self {
            OnChainConsensusConfig::V1(config) | OnChainConsensusConfig::V2(config) => {
                &config.proposer_election_type
            },
        }
    }

    pub fn quorum_store_enabled(&self) -> bool {
        match &self {
            OnChainConsensusConfig::V1(_config) => false,
            OnChainConsensusConfig::V2(_config) => true,
        }
    }
}

/// This is used when on-chain config is not initialized.
impl Default for OnChainConsensusConfig {
    fn default() -> Self {
        OnChainConsensusConfig::V2(ConsensusConfigV1::default())
    }
}

impl OnChainConfig for OnChainConsensusConfig {
    const MODULE_IDENTIFIER: &'static str = "consensus_config";
    const TYPE_IDENTIFIER: &'static str = "ConsensusConfig";

    /// The Move resource is
    /// ```ignore
    /// struct AptosConsensusConfig has copy, drop, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserilization to turn it back to OnChainConsensusConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConsensusConfigV1 {
    pub decoupled_execution: bool,
    // Deprecated and unused, cannot be renamed easily, due to yaml on framework_upgrade test
    pub back_pressure_limit: u64,
    pub exclude_round: u64,
    pub proposer_election_type: ProposerElectionType,
    pub max_failed_authors_to_store: usize,
}

impl Default for ConsensusConfigV1 {
    fn default() -> Self {
        Self {
            decoupled_execution: true,
            back_pressure_limit: 10,
            exclude_round: 40,
            max_failed_authors_to_store: 10,
            proposer_election_type: ProposerElectionType::LeaderReputation(
                LeaderReputationType::ProposerAndVoterV2(ProposerAndVoterConfig {
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
                    weight_by_voting_power: true,
                    use_history_from_previous_epoch_max_count: 5,
                }),
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")] // cannot use tag = "type" as nested enums cannot work, and bcs doesn't support it
pub enum ProposerElectionType {
    // Choose the smallest PeerId as the proposer
    // with specified param contiguous_rounds
    FixedProposer(u32),
    // Round robin rotation of proposers
    // with specified param contiguous_rounds
    RotatingProposer(u32),
    // Committed history based proposer election
    LeaderReputation(LeaderReputationType),
    // Pre-specified proposers for each round,
    // or default proposer if round proposer not
    // specified
    RoundProposer(HashMap<Round, AccountAddress>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderReputationType {
    // Proposer election based on whether nodes succeeded or failed
    // their proposer election rounds, and whether they voted.
    // Version 1:
    // * use reputation window from stale end
    // * simple (predictable) seed
    ProposerAndVoter(ProposerAndVoterConfig),
    // Version 2:
    // * use reputation window from recent end
    // * unpredictable seed, based on root hash
    ProposerAndVoterV2(ProposerAndVoterConfig),
}

impl LeaderReputationType {
    pub fn use_root_hash_for_seed(&self) -> bool {
        // all versions after V1 should use root hash
        !matches!(self, Self::ProposerAndVoter(_))
    }

    pub fn use_reputation_window_from_stale_end(&self) -> bool {
        // all versions after V1 shouldn't use from stale end
        matches!(self, Self::ProposerAndVoter(_))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    // Flag whether to use voting power as multiplier to the weights
    pub weight_by_voting_power: bool,
    // Flag whether to use history from previous epoch (0 if not),
    // representing a number of historical epochs (beyond the current one)
    // to consider.
    pub use_history_from_previous_epoch_max_count: u32,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::on_chain_config::{InMemoryOnChainConfig, OnChainConfigPayload};

    #[test]
    fn test_config_yaml_serialization() {
        let config = OnChainConsensusConfig::default();
        let s = serde_yaml::to_string(&config).unwrap();

        serde_yaml::from_str::<OnChainConsensusConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_bcs_serialization() {
        let config = OnChainConsensusConfig::default();
        let s = bcs::to_bytes(&config).unwrap();

        bcs::from_bytes::<OnChainConsensusConfig>(&s).unwrap();
    }

    #[test]
    fn test_config_serialization_non_default() {
        let config = OnChainConsensusConfig::V1(ConsensusConfigV1 {
            proposer_election_type: ProposerElectionType::RoundProposer(HashMap::from([(
                1,
                AccountAddress::random(),
            )])),
            ..ConsensusConfigV1::default()
        });

        let s = serde_yaml::to_string(&config).unwrap();
        let result = serde_yaml::from_str::<OnChainConsensusConfig>(&s).unwrap();
        assert!(matches!(
            result.proposer_election_type(),
            ProposerElectionType::RoundProposer(_value)
        ));
    }

    #[test]
    fn test_config_onchain_payload() {
        let consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1 {
            proposer_election_type: ProposerElectionType::RoundProposer(HashMap::from([(
                1,
                AccountAddress::random(),
            )])),
            ..ConsensusConfigV1::default()
        });

        let mut configs = HashMap::new();
        configs.insert(
            OnChainConsensusConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&consensus_config).unwrap()).unwrap(),
        );

        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));

        let result: OnChainConsensusConfig = payload.get().unwrap();
        assert!(matches!(
            result.proposer_election_type(),
            ProposerElectionType::RoundProposer(_value)
        ));
    }
}
