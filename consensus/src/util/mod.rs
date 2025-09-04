// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_consensus_types::common::Round;
use velor_types::{
    on_chain_config::{OnChainJWKConsensusConfig, OnChainRandomnessConfig},
    validator_txn::ValidatorTransaction,
};

pub mod db_tool;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_time_service;
pub mod time_service;

pub fn is_vtxn_expected(
    randomness_config: &OnChainRandomnessConfig,
    jwk_consensus_config: &OnChainJWKConsensusConfig,
    vtxn: &ValidatorTransaction,
) -> bool {
    match vtxn {
        ValidatorTransaction::DKGResult(_) => randomness_config.randomness_enabled(),
        ValidatorTransaction::ObservedJWKUpdate(_) => jwk_consensus_config.jwk_consensus_enabled(),
    }
}

pub fn calculate_window_start_round(current_round: Round, window_size: u64) -> Round {
    assert!(window_size > 0);
    (current_round + 1).saturating_sub(window_size)
}
