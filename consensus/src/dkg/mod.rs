// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod dkg_handler;
pub mod dkg_manager;
pub mod dkg_rounding;
mod dkg_store;
pub mod types;
mod tracing;

use crate::dkg::dkg_rounding::DKGRounding;
use aptos_consensus_types::common::Author;
use aptos_dkg::{pvss::{das, traits::Transcript, WeightedTranscript}, constants::SEED_PVSS_PUBLIC_PARAMS};
use aptos_logger::debug;
use aptos_types::{dkg::DKGPvssConfig, on_chain_config::ValidatorSet};
pub use types::{DKGAggNode, DKGMessage, DKGNetworkMessage, DKGNode};

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    next_validator_set: &ValidatorSet,
) -> DKGPvssConfig {
    let validator_addresses: Vec<Author> = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.account_address)
        .collect();
    
    let validator_stakes: Vec<u64> = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.consensus_voting_power())
        .collect();

    let validator_consensus_keys = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.consensus_public_key().clone())
        .collect();

    let dkg_rounding = DKGRounding::new(
        validator_addresses,
        validator_stakes,
        validator_consensus_keys,
    );

    debug!(
            "[DKG] Starting DKG with the following parameters: number of validators: {:?}, validator stakes: {:?}, validator weights: {:?}, validator 1/3 weights: {:?}, validator 2/3 weights: {:?}",
            dkg_rounding.validator_stakes().len(),
            dkg_rounding.validator_stakes(),
            dkg_rounding.validator_weights(),
            dkg_rounding.weighted_config_1().get_threshold_weight(),
            dkg_rounding.weighted_config_2().get_threshold_weight(),
        );

    // dkg todo: decide whether to use consensus key as encryption key
    let consensus_keys: Vec<<das::Transcript as Transcript>::EncryptPubKey> = dkg_rounding
        .validator_consensus_keys()
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();
    let wc_1 = dkg_rounding.weighted_config_1().clone();
    let wc_2 = dkg_rounding.weighted_config_2().clone();

    let pp = <WeightedTranscript<das::Transcript> as Transcript>::PvssPublicParameters::new_from_seed_with_bls_base(SEED_PVSS_PUBLIC_PARAMS);
    let dkg_pvss_config =
        DKGPvssConfig::new(cur_epoch, wc_1.clone(), wc_2.clone(), pp, consensus_keys);

    dkg_pvss_config
}
