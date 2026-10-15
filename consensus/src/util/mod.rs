// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_consensus_types::common::Round;
use aptos_types::{
    on_chain_config::{OnChainChunkyDKGConfig, OnChainJWKConsensusConfig, OnChainRandomnessConfig},
    trust_observation::Config as TrustObservationConfig,
    validator_txn::ValidatorTransaction,
};

pub mod db_tool;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_time_service;
pub mod time_service;

pub fn is_vtxn_expected(
    randomness_config: &OnChainRandomnessConfig,
    jwk_consensus_config: &OnChainJWKConsensusConfig,
    trust_observation_config: &TrustObservationConfig,
    chunky_dkg_config: &OnChainChunkyDKGConfig,
    vtxn: &ValidatorTransaction,
) -> bool {
    match vtxn {
        ValidatorTransaction::DKGResult(_) => randomness_config.randomness_enabled(),
        ValidatorTransaction::ObservedJWKUpdate(_) => jwk_consensus_config.jwk_consensus_enabled(),
        ValidatorTransaction::ObservedTrustUpdate(qc) => trust_observation_config
            .get(&qc.update.target_id)
            .is_some_and(|target| target.config_version == qc.update.config_version),
        ValidatorTransaction::ChunkyDKGResult(_) => chunky_dkg_config.chunky_dkg_enabled(),
    }
}

pub fn calculate_window_start_round(current_round: Round, window_size: u64) -> Round {
    assert!(window_size > 0);
    (current_round + 1).saturating_sub(window_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        trust_observation::{ObservedValue, QuorumCertifiedUpdate, TargetSpec},
    };

    #[test]
    fn trust_update_requires_governed_target_and_matching_config_version() {
        let target = TargetSpec {
            id: b"intel-root".to_vec(),
            url: "https://example.com/root.der".into(),
            format: 1,
            max_bytes: 1024,
            config_version: 7,
        };
        let config = TrustObservationConfig {
            targets: vec![target],
        };
        let mut txn = ValidatorTransaction::ObservedTrustUpdate(QuorumCertifiedUpdate {
            update: ObservedValue {
                target_id: b"intel-root".to_vec(),
                config_version: 7,
                epoch: 42,
                version: 1,
                value: vec![0x30, 0x00],
            },
            multi_sig: AggregateSignature::empty(),
        });
        let randomness = OnChainRandomnessConfig::default_disabled();
        let jwk = OnChainJWKConsensusConfig::default_disabled();
        let chunky = OnChainChunkyDKGConfig::default_disabled();
        assert!(is_vtxn_expected(&randomness, &jwk, &config, &chunky, &txn));
        assert!(!is_vtxn_expected(
            &randomness,
            &jwk,
            &TrustObservationConfig::default(),
            &chunky,
            &txn
        ));
        if let ValidatorTransaction::ObservedTrustUpdate(update) = &mut txn {
            update.update.config_version = 6;
        }
        assert!(!is_vtxn_expected(&randomness, &jwk, &config, &chunky, &txn));
    }
}
