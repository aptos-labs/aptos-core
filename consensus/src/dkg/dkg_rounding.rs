// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::pvss::WeightedConfig;
use aptos_logger::debug;

pub const WEIGHT_PER_VALIDATOR_VEC: &[usize] = &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
pub const ROUNDING_STEPS: usize = 100;

#[derive(Clone)]
pub struct DKGRounding {
    pub validator_weights: Vec<usize>,
    pub threshold_fallback: usize,
    pub threshold_optimistic: usize,
    pub config_fallback: WeightedConfig,
    pub config_optimistic: WeightedConfig,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: Vec<u64>,
        weight_per_validator_vec: Vec<usize>,
        rounding_steps: usize,
    ) -> Self {
        let (
            best_validator_weights,
            best_threshold_fallback,
            best_threshold_optimistic,
            _best_stake_per_weight,
            _best_stake_gap,
        ) = dkg_rounding(validator_stakes, weight_per_validator_vec, rounding_steps);

        let config_fallback =
            WeightedConfig::new(best_threshold_fallback, best_validator_weights.clone()).unwrap();
        let config_optimistic =
            WeightedConfig::new(best_threshold_optimistic, best_validator_weights.clone()).unwrap();

        Self {
            validator_weights: best_validator_weights,
            threshold_fallback: best_threshold_fallback,
            threshold_optimistic: best_threshold_optimistic,
            config_fallback,
            config_optimistic,
        }
    }
}

pub fn dkg_rounding(
    validator_stakes: Vec<u64>,
    weight_per_validator_vec: Vec<usize>,
    rounding_steps: usize,
) -> (Vec<usize>, usize, usize, usize, f64) {
    let mut best_stake_gap = 1.0;
    let mut best_validator_weights = vec![];
    let mut best_threshold_fallback = 0;
    let mut best_threshold_optimistic = 0;
    let mut best_stake_per_weight = 0;
    for weight_per_validator in weight_per_validator_vec {
        let total_weight = weight_per_validator * validator_stakes.len();
        
        // let (validator_weights, threshold_fallback, threshold_optimistic, stake_per_weight, stake_gap) = rounding_scheme_dummy(validator_stakes.clone(), total_weight, rounding_steps);

        let (validator_weights, threshold_fallback, threshold_optimistic, stake_per_weight, stake_gap) = rounding_scheme_advanced(validator_stakes.clone(), total_weight, rounding_steps);

        // This check makes sure the fallback path is live: 2/3 stakes can reconstruct the randomness.
        assert!(stake_gap <= 1.0 / 3.0);

        if stake_gap < best_stake_gap {
            best_stake_gap = stake_gap;
            best_validator_weights = validator_weights;
            best_threshold_fallback = threshold_fallback;
            best_threshold_optimistic = threshold_optimistic;
            best_stake_per_weight = stake_per_weight;
        }
    }

    debug!(
        "[DKG] Rounding finished! \n threshold_fallback = {}, threshold_optimistic = {} \n stake_gap = {} \n best_stake_per_weight = {} \n validator_weights = {:?}",
        best_threshold_fallback,
        best_threshold_optimistic,
        best_stake_gap,
        best_stake_per_weight,
        best_validator_weights,
    );
        
    (
        best_validator_weights,
        best_threshold_fallback,
        best_threshold_optimistic,
        best_stake_per_weight,
        best_stake_gap,
    )
}

// only for testing, assign each validator weight of 10
pub fn rounding_scheme_dummy(
    validator_stakes: Vec<u64>,
    _weights_sum: usize,
    _rounding_steps: usize,
) -> (Vec<usize>, usize, usize, usize, f64) {
    let validator_weights = vec![10; validator_stakes.len()];
    let total_weight = validator_weights.iter().sum::<usize>();
    let threshold_fallback = (total_weight - 1) / 3 + 1;
    let threshold_optimistic = (total_weight - 1) / 3 * 2 + 1;
    let stake_per_weight: usize = validator_stakes.iter().sum::<u64>() as usize / total_weight;
    let stake_gap = 0.0;
    (
        validator_weights,
        threshold_fallback,
        threshold_optimistic,
        stake_per_weight,
        stake_gap,
    )
}

pub fn rounding_scheme_advanced(
    validator_stakes: Vec<u64>,
    weights_sum: usize,
    rounding_steps: usize,
) -> (Vec<usize>, usize, usize, usize, f64) {
    let stake_sum = validator_stakes.iter().sum::<u64>();
    let stake_per_weight = stake_sum / weights_sum as u64;
    let fractions = validator_stakes
        .iter()
        .map(|stake| (*stake as f64 / stake_per_weight as f64) - ((stake / stake_per_weight) as f64))
        .collect::<Vec<f64>>();
    let mut best_c = 0.0;
    let mut best_delta = fractions.len() as f64;
    // let mut best_delta_d = 0.0;
    let mut best_delta_u = 0.0;
    for i in 0..rounding_steps {
        let mut delta_d = 0.0;
        let mut delta_u = 0.0;
        let c = i as f64 / rounding_steps as f64;
        for j in 0..fractions.len() {
            if fractions[j] + c >= 1.0 {
                delta_u += 1.0 - fractions[j];
            } else {
                delta_d += fractions[j];
            }
            if delta_u + delta_d < best_delta {
                best_delta = delta_u + delta_d;
                // best_delta_d = delta_d;
                best_delta_u = delta_u;
                best_c = c;
            }
        }
    }

    let validator_weights = validator_stakes
        .iter()
        .map(|stake| (*stake as f64 / stake_per_weight as f64 + best_c) as u64)
        .collect::<Vec<u64>>();

    let threshold_fallback = ((stake_sum as f64) / (3.0 * stake_per_weight as f64) + best_delta_u).ceil();
    let threshold_optimistic = ((2.0 * stake_sum as f64) / (3.0 * stake_per_weight as f64) + best_delta_u).ceil();

    let stake_gap = stake_per_weight as f64 * best_delta / stake_sum as f64;

    debug!(
        "[DKG] Rounding in progress! \n threshold_fallback = {}, threshold_optimistic = {} \n stake_per_weight = {}, best_delta = {}, stake_gap = {} \n validator_weights = {:?}",
        threshold_fallback,
        threshold_optimistic,
        stake_per_weight,
        best_delta,
        stake_gap,
        validator_weights,
    );

    (
        validator_weights.iter().map(|w| *w as usize).collect::<Vec<usize>>(),
        threshold_fallback as usize,
        threshold_optimistic as usize,
        stake_per_weight as usize,
        stake_gap,
    )
}

#[test]
    fn test_rounding_scheme() {
        // even stake distribution
        let num_validators = 100;
        let validator_stakes = vec![1_000_000; num_validators];
        let dkg_rounding = DKGRounding::new(validator_stakes.clone(), WEIGHT_PER_VALIDATOR_VEC.to_vec(), ROUNDING_STEPS);

        let validator_weights_true = vec![WEIGHT_PER_VALIDATOR_VEC[0]; num_validators];
        let threshold_fallback_true = ((WEIGHT_PER_VALIDATOR_VEC[0] * num_validators - 1) as f64 / 3.0).floor() as usize + 1;
        let threshold_optimistic_true = ((WEIGHT_PER_VALIDATOR_VEC[0] * num_validators - 1) as f64 / 3.0).floor() as usize * 2 + 1;
        assert_eq!(dkg_rounding.validator_weights, validator_weights_true);
        assert_eq!(dkg_rounding.threshold_fallback, threshold_fallback_true);
        assert_eq!(dkg_rounding.threshold_optimistic, threshold_optimistic_true);
    }
