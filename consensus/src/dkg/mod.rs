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
use aptos_dkg::{pvss::{das, traits::Transcript, WeightedTranscript}, constants::SEED_PVSS_PUBLIC_PARAMS};
use aptos_logger::debug;
use aptos_types::{dkg::DKGPvssConfig, on_chain_config::ValidatorSet};
pub use types::{DKGAggNode, DKGMessage, DKGNetworkMessage, DKGNode};

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    next_validator_set: &ValidatorSet,
) -> (DKGRounding, DKGPvssConfig) {
    let validator_stakes: Vec<u64> = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.consensus_voting_power())
        .collect();

    let dkg_rounding = DKGRounding::new(
        validator_stakes.clone(),
        dkg_rounding::WEIGHT_PER_VALIDATOR_VEC.to_vec(),
        dkg_rounding::ROUNDING_STEPS,
    );

    debug!(
            "[DKG] Starting DKG with the following parameters: number of validators: {:?}, validator stakes: {:?}, validator weights: {:?}, fallback weights threshold: {:?}, optimistic weights threshold: {:?}",
            validator_stakes.len(),
            validator_stakes,
            dkg_rounding.validator_weights,
            dkg_rounding.threshold_fallback,
            dkg_rounding.threshold_optimistic,
        );


    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validator_set
    .active_validators
    .iter()
    .map(|vi| vi.consensus_public_key().clone())
    .collect();

    let consensus_keys: Vec<<das::Transcript as Transcript>::EncryptPubKey> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();
    
    let wc_1 = dkg_rounding.config_fallback.clone();
    let wc_2 = dkg_rounding.config_optimistic.clone();

    let pp = <WeightedTranscript<das::Transcript> as Transcript>::PvssPublicParameters::new_from_seed_with_bls_base(SEED_PVSS_PUBLIC_PARAMS);

    let dkg_pvss_config =
        DKGPvssConfig::new(cur_epoch, wc_1.clone(), wc_2.clone(), pp, consensus_keys);

    (dkg_rounding, dkg_pvss_config)
}
