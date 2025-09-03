// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block_info::Round, on_chain_config::OnChainConfig};
use anyhow::{format_err, Result};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default Window Size for Execution Pool.
/// This describes the number of blocks in the Execution Pool Window
pub const DEFAULT_WINDOW_SIZE: Option<u64> = None;
pub const DEFAULT_ENABLED_WINDOW_SIZE: Option<u64> = Some(1);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ConsensusAlgorithmConfig {
    Jolteon {
        main: ConsensusConfigV1,
        quorum_store_enabled: bool,
    },
    DAG(DagConsensusConfigV1),
    JolteonV2 {
        main: ConsensusConfigV1,
        quorum_store_enabled: bool,
        order_vote_enabled: bool,
    },
}

impl ConsensusAlgorithmConfig {
    pub fn default_for_genesis() -> Self {
        Self::JolteonV2 {
            main: ConsensusConfigV1::default(),
            quorum_store_enabled: true,
            order_vote_enabled: true,
        }
    }

    pub fn default_with_quorum_store_disabled() -> Self {
        Self::JolteonV2 {
            main: ConsensusConfigV1::default(),
            quorum_store_enabled: false,
            order_vote_enabled: true,
        }
    }

    pub fn default_if_missing() -> Self {
        Self::JolteonV2 {
            main: ConsensusConfigV1::default(),
            quorum_store_enabled: true,
            order_vote_enabled: false,
        }
    }

    pub fn quorum_store_enabled(&self) -> bool {
        match self {
            ConsensusAlgorithmConfig::Jolteon {
                quorum_store_enabled,
                ..
            }
            | ConsensusAlgorithmConfig::JolteonV2 {
                quorum_store_enabled,
                ..
            } => *quorum_store_enabled,
            ConsensusAlgorithmConfig::DAG(_) => true,
        }
    }

    pub fn order_vote_enabled(&self) -> bool {
        match self {
            ConsensusAlgorithmConfig::JolteonV2 {
                order_vote_enabled, ..
            } => *order_vote_enabled,
            _ => false,
        }
    }

    pub fn is_dag_enabled(&self) -> bool {
        match self {
            ConsensusAlgorithmConfig::Jolteon { .. }
            | ConsensusAlgorithmConfig::JolteonV2 { .. } => false,
            ConsensusAlgorithmConfig::DAG(_) => true,
        }
    }

    pub fn leader_reputation_exclude_round(&self) -> u64 {
        match self {
            ConsensusAlgorithmConfig::Jolteon { main, .. }
            | ConsensusAlgorithmConfig::JolteonV2 { main, .. } => main.exclude_round,
            _ => unimplemented!("method not supported"),
        }
    }

    pub fn max_failed_authors_to_store(&self) -> usize {
        match self {
            ConsensusAlgorithmConfig::Jolteon { main, .. }
            | ConsensusAlgorithmConfig::JolteonV2 { main, .. } => main.max_failed_authors_to_store,
            _ => unimplemented!("method not supported"),
        }
    }

    pub fn proposer_election_type(&self) -> &ProposerElectionType {
        match self {
            ConsensusAlgorithmConfig::Jolteon { main, .. } => &main.proposer_election_type,
            ConsensusAlgorithmConfig::JolteonV2 { main, .. } => &main.proposer_election_type,
            _ => unimplemented!("method not supported"),
        }
    }

    pub fn unwrap_dag_config_v1(&self) -> &DagConsensusConfigV1 {
        match self {
            ConsensusAlgorithmConfig::DAG(dag) => dag,
            _ => unreachable!("not a dag config"),
        }
    }

    pub fn unwrap_jolteon_config_v1(&self) -> &ConsensusConfigV1 {
        match self {
            ConsensusAlgorithmConfig::Jolteon { main, .. } => main,
            ConsensusAlgorithmConfig::JolteonV2 { main, .. } => main,
            _ => unreachable!("not a jolteon config"),
        }
    }
}

const VTXN_CONFIG_PER_BLOCK_LIMIT_TXN_COUNT_DEFAULT: u64 = 2;
const VTXN_CONFIG_PER_BLOCK_LIMIT_TOTAL_BYTES_DEFAULT: u64 = 2097152; //2MB

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ValidatorTxnConfig {
    /// Disabled. In Jolteon, it also means to not use `BlockType::ProposalExt`.
    V0,
    /// Enabled. Per-block vtxn count and their total bytes are limited.
    V1 {
        per_block_limit_txn_count: u64,
        per_block_limit_total_bytes: u64,
    },
}

impl ValidatorTxnConfig {
    pub fn default_for_genesis() -> Self {
        Self::V1 {
            per_block_limit_txn_count: VTXN_CONFIG_PER_BLOCK_LIMIT_TXN_COUNT_DEFAULT,
            per_block_limit_total_bytes: VTXN_CONFIG_PER_BLOCK_LIMIT_TOTAL_BYTES_DEFAULT,
        }
    }

    pub fn default_if_missing() -> Self {
        Self::V0
    }

    pub fn default_disabled() -> Self {
        Self::V0
    }

    pub fn default_enabled() -> Self {
        Self::V1 {
            per_block_limit_txn_count: VTXN_CONFIG_PER_BLOCK_LIMIT_TXN_COUNT_DEFAULT,
            per_block_limit_total_bytes: VTXN_CONFIG_PER_BLOCK_LIMIT_TOTAL_BYTES_DEFAULT,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            ValidatorTxnConfig::V0 => false,
            ValidatorTxnConfig::V1 { .. } => true,
        }
    }

    pub fn per_block_limit_txn_count(&self) -> u64 {
        match self {
            ValidatorTxnConfig::V0 => 0,
            ValidatorTxnConfig::V1 {
                per_block_limit_txn_count,
                ..
            } => *per_block_limit_txn_count,
        }
    }

    pub fn per_block_limit_total_bytes(&self) -> u64 {
        match self {
            ValidatorTxnConfig::V0 => 0,
            ValidatorTxnConfig::V1 {
                per_block_limit_total_bytes,
                ..
            } => *per_block_limit_total_bytes,
        }
    }
}

/// The on-chain consensus config, in order to be able to add fields, we use enum to wrap the actual struct.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainConsensusConfig {
    V1(ConsensusConfigV1),
    V2(ConsensusConfigV1),
    V3 {
        alg: ConsensusAlgorithmConfig,
        vtxn: ValidatorTxnConfig,
    },
    V4 {
        alg: ConsensusAlgorithmConfig,
        vtxn: ValidatorTxnConfig,
        // Execution pool block window
        window_size: Option<u64>,
    },
    V5 {
        alg: ConsensusAlgorithmConfig,
        vtxn: ValidatorTxnConfig,
        // Execution pool block window
        window_size: Option<u64>,
        // Whether to check if we can skip generating randomness for blocks
        rand_check_enabled: bool,
    },
}

/// The public interface that exposes all values with safe fallback.
impl OnChainConsensusConfig {
    pub fn default_for_genesis() -> Self {
        OnChainConsensusConfig::V5 {
            alg: ConsensusAlgorithmConfig::default_for_genesis(),
            vtxn: ValidatorTxnConfig::default_for_genesis(),
            window_size: DEFAULT_WINDOW_SIZE,
            rand_check_enabled: true,
        }
    }

    /// The number of recent rounds that don't count into reputations.
    pub fn leader_reputation_exclude_round(&self) -> u64 {
        match &self {
            OnChainConsensusConfig::V1(config) | OnChainConsensusConfig::V2(config) => {
                config.exclude_round
            },
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.leader_reputation_exclude_round(),
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
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.max_failed_authors_to_store(),
        }
    }

    // Type and configuration used for proposer election.
    pub fn proposer_election_type(&self) -> &ProposerElectionType {
        match &self {
            OnChainConsensusConfig::V1(config) | OnChainConsensusConfig::V2(config) => {
                &config.proposer_election_type
            },
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.proposer_election_type(),
        }
    }

    pub fn quorum_store_enabled(&self) -> bool {
        match &self {
            OnChainConsensusConfig::V1(_config) => false,
            OnChainConsensusConfig::V2(_) => true,
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.quorum_store_enabled(),
        }
    }

    pub fn order_vote_enabled(&self) -> bool {
        match &self {
            OnChainConsensusConfig::V1(_config) => false,
            OnChainConsensusConfig::V2(_) => false,
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.order_vote_enabled(),
        }
    }

    pub fn is_dag_enabled(&self) -> bool {
        match self {
            OnChainConsensusConfig::V1(_) => false,
            OnChainConsensusConfig::V2(_) => false,
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.is_dag_enabled(),
        }
    }

    pub fn unwrap_dag_config_v1(&self) -> &DagConsensusConfigV1 {
        match &self {
            OnChainConsensusConfig::V1(_) | OnChainConsensusConfig::V2(_) => {
                unreachable!("not a dag config")
            },
            OnChainConsensusConfig::V3 { alg, .. }
            | OnChainConsensusConfig::V4 { alg, .. }
            | OnChainConsensusConfig::V5 { alg, .. } => alg.unwrap_dag_config_v1(),
        }
    }

    pub fn effective_validator_txn_config(&self) -> ValidatorTxnConfig {
        match self {
            OnChainConsensusConfig::V1(_) | OnChainConsensusConfig::V2(_) => {
                ValidatorTxnConfig::default_disabled()
            },
            OnChainConsensusConfig::V3 { vtxn, .. }
            | OnChainConsensusConfig::V4 { vtxn, .. }
            | OnChainConsensusConfig::V5 { vtxn, .. } => vtxn.clone(),
        }
    }

    pub fn is_vtxn_enabled(&self) -> bool {
        self.effective_validator_txn_config().enabled()
    }

    pub fn disable_validator_txns(&mut self) {
        match self {
            OnChainConsensusConfig::V1(_) | OnChainConsensusConfig::V2(_) => {
                // vtxn not supported. No-op.
            },
            OnChainConsensusConfig::V3 { vtxn, .. }
            | OnChainConsensusConfig::V4 { vtxn, .. }
            | OnChainConsensusConfig::V5 { vtxn, .. } => {
                *vtxn = ValidatorTxnConfig::V0;
            },
        }
    }

    pub fn enable_validator_txns(&mut self) {
        let new_self = match std::mem::take(self) {
            OnChainConsensusConfig::V1(config) => OnChainConsensusConfig::V5 {
                alg: ConsensusAlgorithmConfig::JolteonV2 {
                    main: config,
                    quorum_store_enabled: false,
                    order_vote_enabled: false,
                },
                vtxn: ValidatorTxnConfig::default_enabled(),
                window_size: DEFAULT_WINDOW_SIZE,
                rand_check_enabled: true,
            },
            OnChainConsensusConfig::V2(config) => OnChainConsensusConfig::V5 {
                alg: ConsensusAlgorithmConfig::JolteonV2 {
                    main: config,
                    quorum_store_enabled: true,
                    order_vote_enabled: false,
                },
                vtxn: ValidatorTxnConfig::default_enabled(),
                window_size: DEFAULT_WINDOW_SIZE,
                rand_check_enabled: true,
            },
            OnChainConsensusConfig::V3 {
                vtxn: ValidatorTxnConfig::V0,
                alg,
            } => OnChainConsensusConfig::V5 {
                alg,
                vtxn: ValidatorTxnConfig::default_enabled(),
                window_size: DEFAULT_WINDOW_SIZE,
                rand_check_enabled: true,
            },
            OnChainConsensusConfig::V4 {
                alg,
                vtxn: ValidatorTxnConfig::V0,
                window_size,
            } => OnChainConsensusConfig::V4 {
                alg,
                vtxn: ValidatorTxnConfig::default_enabled(),
                window_size,
            },
            OnChainConsensusConfig::V5 {
                alg,
                vtxn: ValidatorTxnConfig::V0,
                window_size,
                rand_check_enabled: rand_check,
            } => OnChainConsensusConfig::V5 {
                alg,
                vtxn: ValidatorTxnConfig::default_enabled(),
                window_size,
                rand_check_enabled: rand_check,
            },
            item @ OnChainConsensusConfig::V3 {
                vtxn: ValidatorTxnConfig::V1 { .. },
                ..
            } => item,
            item @ OnChainConsensusConfig::V4 {
                vtxn: ValidatorTxnConfig::V1 { .. },
                ..
            } => item,
            item @ OnChainConsensusConfig::V5 {
                vtxn: ValidatorTxnConfig::V1 { .. },
                ..
            } => item,
        };
        *self = new_self;
    }

    pub fn window_size(&self) -> Option<u64> {
        match self {
            OnChainConsensusConfig::V1(_)
            | OnChainConsensusConfig::V2(_)
            | OnChainConsensusConfig::V3 { .. } => None,
            OnChainConsensusConfig::V4 { window_size, .. }
            | OnChainConsensusConfig::V5 { window_size, .. } => *window_size,
        }
    }

    pub fn rand_check_enabled(&self) -> bool {
        match self {
            OnChainConsensusConfig::V1(_)
            | OnChainConsensusConfig::V2(_)
            | OnChainConsensusConfig::V3 { .. }
            | OnChainConsensusConfig::V4 { .. } => false,
            OnChainConsensusConfig::V5 {
                rand_check_enabled: rand_check,
                ..
            } => *rand_check,
        }
    }
}

/// This is used when on-chain config is not initialized.
/// TODO: rename to "default_if_missing()" to be consistent with others?
impl Default for OnChainConsensusConfig {
    fn default() -> Self {
        OnChainConsensusConfig::V4 {
            alg: ConsensusAlgorithmConfig::default_if_missing(),
            vtxn: ValidatorTxnConfig::default_if_missing(),
            window_size: DEFAULT_WINDOW_SIZE,
        }
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorElectionMode {
    RoundRobin,
    LeaderReputation(LeaderReputationType),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DagConsensusConfigV1 {
    pub dag_ordering_causal_history_window: usize,
    pub anchor_election_mode: AnchorElectionMode,
}

impl Default for DagConsensusConfigV1 {
    /// It is primarily used as `default_if_missing()`.
    fn default() -> Self {
        Self {
            dag_ordering_causal_history_window: 10,
            anchor_election_mode: AnchorElectionMode::LeaderReputation(
                LeaderReputationType::ProposerAndVoterV2(ProposerAndVoterConfig {
                    active_weight: 1000,
                    inactive_weight: 10,
                    failed_weight: 1,
                    failure_threshold_percent: 10,
                    proposer_window_num_validators_multiplier: 10,
                    voter_window_num_validators_multiplier: 1,
                    weight_by_voting_power: true,
                    use_history_from_previous_epoch_max_count: 5,
                }),
            ),
        }
    }
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
