use crate::on_chain_config::{LeaderReputationType, ProposerAndVoterConfig, ProposerElectionType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct JolteonConfig {
    pub exclude_round: u64,
    pub proposer_election_type: ProposerElectionType,
    pub max_failed_authors_to_store: usize,
    pub quorum_store_enabled: bool,
}

impl JolteonConfig {
    pub fn default_if_missing() -> Self {
        Self {
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
            quorum_store_enabled: false,
        }
    }

    pub fn default_for_genesis() -> Self {
        Self {
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
            quorum_store_enabled: true,
        }
    }
}
