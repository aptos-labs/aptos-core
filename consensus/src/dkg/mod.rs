// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod dkg_handler;
pub mod dkg_manager;
pub mod dkg_rounding;
mod dkg_store;
pub mod types;
mod tracing;

use crate::dkg::dkg_rounding::DKGRounding;
use aptos_crypto::bls12381;
use aptos_dkg::{constants::SEED_PVSS_PUBLIC_PARAMS};
use aptos_logger::debug;
use aptos_types::{dkg::{DKGPvssConfig, EncPK, DkgPP}, on_chain_config::ValidatorSet};
pub use types::{DKGAggNode, DKGMessage, DKGNetworkMessage, DKGNode};

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    next_validator_set: &ValidatorSet,
) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.consensus_voting_power())
        .collect();

    // // For mainnet-like testing
    // let validator_stakes: Vec<u64> = MAINNET_STAKES.to_vec();
    // assert!(validator_stakes.len() == next_validator_set.active_validators.len());

    let dkg_rounding = DKGRounding::new(
        validator_stakes.clone(),
        dkg_rounding::STAKE_GAP_THRESHOLD,
        dkg_rounding::WEIGHT_PER_VALIDATOR_MIN,
        dkg_rounding::WEIGHT_PER_VALIDATOR_MAX,
        dkg_rounding::STEPS,
        dkg_rounding::FALLBACK_RECONSTRUCT_THRESHOLD,
        dkg_rounding::OPTIMISTIC_RECONSTRUCT_THRESHOLD,
    );

    debug!(
            "[DKG] Starting DKG with the following parameters: number of validators: {:?}, validator stakes: {:?}, validator weights: {:?}, fallback weights threshold: {:?}, optimistic weights threshold: {:?}",
            validator_stakes.len(),
            validator_stakes,
            dkg_rounding.profile.validator_weights,
            dkg_rounding.profile.threshold_f,
            dkg_rounding.profile.threshold_o,
        );


    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validator_set
    .active_validators
    .iter()
    .map(|vi| vi.consensus_public_key().clone())
    .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();
    
    let wc_f = dkg_rounding.config_f.clone();
    let wc_o = dkg_rounding.config_o.clone();

    let pp = DkgPP::new_from_seed_with_bls_base(SEED_PVSS_PUBLIC_PARAMS);

    let dkg_pvss_config =
        DKGPvssConfig::new(cur_epoch,  wc_f.clone(), wc_o.clone(), pp, consensus_keys);

    dkg_pvss_config
}
