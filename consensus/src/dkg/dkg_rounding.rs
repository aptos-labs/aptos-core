// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381;
use aptos_dkg::pvss::WeightedConfig;

pub const MAX_NUM_SHARES: usize = 1000;

#[derive(Clone)]
pub struct DKGRounding {
    pub validator_addresses: Vec<Author>,
    pub validator_stakes: Vec<u64>,
    pub validator_consensus_keys: Vec<bls12381::PublicKey>,
    // pub validator_indexes: Vec<u64>,
    pub validator_weights: Vec<usize>,
    pub weights_of_one_third_stake: usize,
    pub weights_of_two_third_stake: usize,
    pub weighted_config_1: WeightedConfig,
    pub weighted_config_2: WeightedConfig,
}

impl DKGRounding {
    pub fn validator_stakes(&self) -> &Vec<u64> {
        &self.validator_stakes
    }
    pub fn validator_consensus_keys(&self) -> &Vec<bls12381::PublicKey> {
        &self.validator_consensus_keys
    }
    pub fn validator_weights(&self) -> &Vec<usize> {
        &self.validator_weights
    }
    // pub fn weights_of_one_third_stake(&self) -> usize {
    //     self.weights_of_one_third_stake
    // }
    // pub fn weights_of_two_third_stake(&self) -> usize {
    //     self.weights_of_two_third_stake
    // }

    pub fn weighted_config_1(&self) -> WeightedConfig {
        self.weighted_config_1.clone()
    }

    pub fn weighted_config_2(&self) -> WeightedConfig {
        self.weighted_config_2.clone()
    }

    pub fn new(
        validator_addresses: Vec<Author>,
        validator_stakes: Vec<u64>,
        validator_consensus_keys: Vec<bls12381::PublicKey>,
    ) -> Self {
        let (validator_weights, weights_of_one_third_stake, weights_of_two_third_stake) =
            rounding_scheme(validator_stakes.clone(), MAX_NUM_SHARES);

        // dkg todo: can different weights of two transcripts help to reduce rounding error?
        let weighted_config_1 =
            WeightedConfig::new(weights_of_one_third_stake, validator_weights.clone()).unwrap();
        let weighted_config_2 =
            WeightedConfig::new(weights_of_two_third_stake, validator_weights.clone()).unwrap();

        Self {
            validator_addresses,
            validator_stakes,
            validator_consensus_keys,
            // validator_indexes,
            validator_weights,
            weights_of_one_third_stake,
            weights_of_two_third_stake,
            weighted_config_1,
            weighted_config_2,
        }
    }
}

pub fn rounding_scheme(
    validator_stakes: Vec<u64>,
    _max_num_shares: usize,
) -> (Vec<usize>, usize, usize) {
    // naive rounding by dividing an unit and round down
    // dkg todo: better rounding?
    let validator_weights = validator_stakes
        .iter()
        .map(|_s| 10)
        .collect::<Vec<usize>>();
    let total_weight = validator_weights.iter().sum::<usize>();
    // dkg todo: calculate the actual weights of one third stake and two third stake
    let weights_of_one_third_stake = (total_weight - 1) / 3 + 1;
    let weights_of_two_third_stake = (total_weight - 1) / 3 * 2 + 1;
    (
        validator_weights,
        weights_of_one_third_stake,
        weights_of_two_third_stake,
    )
}
