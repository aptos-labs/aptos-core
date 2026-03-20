// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_consensus_types::common::Round;
use aptos_types::{
    on_chain_config::{OnChainChunkyDKGConfig, OnChainJWKConsensusConfig, OnChainRandomnessConfig},
    validator_txn::ValidatorTransaction,
};

pub mod db_tool;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_time_service;
pub mod time_service;

pub fn is_vtxn_expected(
    randomness_config: &OnChainRandomnessConfig,
    jwk_consensus_config: &OnChainJWKConsensusConfig,
    chunky_dkg_config: &OnChainChunkyDKGConfig,
    vtxn: &ValidatorTransaction,
) -> bool {
    match vtxn {
        ValidatorTransaction::DKGResult(_) => randomness_config.randomness_enabled(),
        ValidatorTransaction::ObservedJWKUpdate(_) => jwk_consensus_config.jwk_consensus_enabled(),
        ValidatorTransaction::ChunkyDKGResult(_) => chunky_dkg_config.chunky_dkg_enabled(),
    }
}

pub fn calculate_window_start_round(current_round: Round, window_size: u64) -> Round {
    assert!(window_size > 0);
    (current_round + 1).saturating_sub(window_size)
}
