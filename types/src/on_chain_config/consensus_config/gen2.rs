use crate::on_chain_config::{
    consensus_config::{
        extra_features::ConsensusExtraFeatures, jolteon::JolteonConfig,
        state_sync_notifier::StateSyncNotifierConfig, DagConsensusConfigV2,
    },
    ConsensusExtraFeature, ProposerElectionType,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConsensusConfigGen2 {
    pub alg: AlgorithmSpecificConfig,
    pub state_sync_notifier: StateSyncNotifierConfig,
    pub extra_features: ConsensusExtraFeatures,
}

impl ConsensusConfigGen2 {
    pub fn default_for_genesis() -> Self {
        Self {
            alg: AlgorithmSpecificConfig::Jolteon(JolteonConfig::default_for_genesis()),
            state_sync_notifier: StateSyncNotifierConfig::default_for_genesis(),
            extra_features: ConsensusExtraFeatures::default_for_genesis(),
        }
    }

    pub fn leader_reputation_exclude_round(&self) -> u64 {
        match &self.alg {
            AlgorithmSpecificConfig::Jolteon(jolteon) => jolteon.exclude_round,
            AlgorithmSpecificConfig::DAG(_) => unimplemented!("method not supported"),
        }
    }

    pub fn max_failed_authors_to_store(&self) -> usize {
        match &self.alg {
            AlgorithmSpecificConfig::Jolteon(jolteon) => jolteon.max_failed_authors_to_store,
            AlgorithmSpecificConfig::DAG(_) => unimplemented!("method not supported"),
        }
    }

    pub fn proposer_election_type(&self) -> &ProposerElectionType {
        match &self.alg {
            AlgorithmSpecificConfig::Jolteon(jolteon) => &jolteon.proposer_election_type,
            AlgorithmSpecificConfig::DAG(_) => unimplemented!("method not supported"),
        }
    }

    pub fn quorum_store_enabled(&self) -> bool {
        match &self.alg {
            AlgorithmSpecificConfig::Jolteon(jolteon) => jolteon.quorum_store_enabled,
            AlgorithmSpecificConfig::DAG(_) => false,
        }
    }

    pub fn is_dag_enabled(&self) -> bool {
        match &self.alg {
            AlgorithmSpecificConfig::Jolteon(_) => false,
            AlgorithmSpecificConfig::DAG(_) => true,
        }
    }

    pub fn as_dag_config_v2(&self) -> DagConsensusConfigV2 {
        match &self.alg {
            AlgorithmSpecificConfig::Jolteon(_) => unreachable!("not a dag config"),
            AlgorithmSpecificConfig::DAG(dag) => dag.clone(),
        }
    }

    pub fn validator_txn_enabled(&self) -> bool {
        self.extra_features
            .is_enabled(ConsensusExtraFeature::ValidatorTransaction)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum AlgorithmSpecificConfig {
    Jolteon(JolteonConfig),
    DAG(DagConsensusConfigV2),
}
