// Copyright © Aptos Foundation

use aptos_dkg::pvss::WeightedConfig;
use fixed::types::U64F64;
use once_cell::sync::Lazy;
use std::{
    cmp::max,
    fmt,
    fmt::{Debug, Formatter},
};

pub fn total_weight_lower_bound(validator_stakes: &Vec<u64>) -> usize {
    // Each validator has at least 1 weight.
    validator_stakes.len()
}

pub fn total_weight_upper_bound(
    validator_stakes: &Vec<u64>,
    reconstruct_threshold_in_stake_ratio: U64F64,
    secrecy_threshold_in_stake_ratio: U64F64,
) -> usize {
    assert!(reconstruct_threshold_in_stake_ratio > secrecy_threshold_in_stake_ratio);
    let bound_1 = ((U64F64::from_num(1)
        / (reconstruct_threshold_in_stake_ratio - secrecy_threshold_in_stake_ratio))
        + U64F64::from_num(1))
    .to_num::<u64>()
        * (validator_stakes.len() as u64);

    let stake_sum = validator_stakes.iter().sum::<u64>();
    let stake_min = *validator_stakes.iter().min().unwrap();
    let bound_2 = stake_sum / max(1, stake_min) + 1;

    max(bound_1 as usize, bound_2 as usize)
}

#[derive(Clone, Debug)]
pub struct DKGRounding {
    pub profile: DKGRoundingProfile,
    pub wconfig: WeightedConfig,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: &Vec<u64>,
        secrecy_threshold_in_stake_ratio: U64F64,
        reconstruct_threshold_in_stake_ratio: U64F64,
    ) -> Self {
        assert!(reconstruct_threshold_in_stake_ratio > secrecy_threshold_in_stake_ratio);

        let total_weight_min = total_weight_lower_bound(validator_stakes);
        let total_weight_max = total_weight_upper_bound(
            validator_stakes,
            reconstruct_threshold_in_stake_ratio,
            secrecy_threshold_in_stake_ratio,
        );

        let profile = DKGRoundingProfile::new(
            validator_stakes,
            total_weight_min,
            total_weight_max,
            secrecy_threshold_in_stake_ratio,
            reconstruct_threshold_in_stake_ratio,
        );

        let wconfig = WeightedConfig::new(
            profile.reconstruct_threshold_in_weights as usize,
            profile
                .validator_weights
                .iter()
                .map(|w| *w as usize)
                .collect(),
        )
        .unwrap();

        Self { profile, wconfig }
    }
}

#[derive(Clone)]
pub struct DKGRoundingProfile {
    // calculated weights for each validator after rounding
    pub validator_weights: Vec<u64>,
    // The ratio of stake that may reveal the randomness, e.g. 50%
    pub secrecy_threshold_in_stake_ratio: U64F64,
    // The ratio of stake that always can reconstruct the randomness, e.g. 66.67%
    pub reconstruct_threshold_in_stake_ratio: U64F64,
    // The number of weights needed to reconstruct the randomness
    pub reconstruct_threshold_in_weights: u64,
}

impl Debug for DKGRoundingProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "total_weight: {}, ",
            self.validator_weights.iter().sum::<u64>()
        )?;
        write!(
            f,
            "secrecy_threshold_in_stake_ratio: {}, ",
            self.secrecy_threshold_in_stake_ratio
        )?;
        write!(
            f,
            "reconstruct_threshold_in_stake_ratio: {}, ",
            self.reconstruct_threshold_in_stake_ratio
        )?;
        write!(
            f,
            "reconstruct_threshold_in_weights: {}, ",
            self.reconstruct_threshold_in_weights
        )?;
        writeln!(f, "validator_weights: {:?}", self.validator_weights)?;

        Ok(())
    }
}

impl DKGRoundingProfile {
    pub fn new(
        validator_stakes: &Vec<u64>,
        total_weight_min: usize,
        total_weight_max: usize,
        secrecy_threshold_in_stake_ratio: U64F64,
        reconstruct_threshold_in_stake_ratio: U64F64,
    ) -> Self {
        assert!(total_weight_min >= validator_stakes.len());
        assert!(total_weight_max >= total_weight_min);
        assert!(secrecy_threshold_in_stake_ratio * U64F64::from_num(3) > U64F64::from_num(1));
        assert!(secrecy_threshold_in_stake_ratio < reconstruct_threshold_in_stake_ratio);
        assert!(reconstruct_threshold_in_stake_ratio * U64F64::from_num(3) <= U64F64::from_num(2));

        let mut weight_low = total_weight_min as u64;
        let mut weight_high = total_weight_max as u64;
        let mut best_profile = compute_profile_fixed_point(
            validator_stakes,
            weight_low,
            secrecy_threshold_in_stake_ratio,
        );

        if is_valid_profile(&best_profile, reconstruct_threshold_in_stake_ratio) {
            return best_profile;
        }

        // binary search for the minimum weight that satisfies the conditions
        while weight_low <= weight_high {
            let weight_mid = weight_low + (weight_high - weight_low) / 2;
            let profile = compute_profile_fixed_point(
                validator_stakes,
                weight_mid,
                secrecy_threshold_in_stake_ratio,
            );

            // Check if the current weight satisfies the conditions
            if is_valid_profile(&profile, reconstruct_threshold_in_stake_ratio) {
                best_profile = profile;
                weight_high = weight_mid - 1;
            } else {
                weight_low = weight_mid + 1;
            }
        }

        // todo: remove once aptos-dkg supports 0 weights
        if !is_valid_profile(&best_profile, reconstruct_threshold_in_stake_ratio) {
            println!("[Randomness] Rounding error: failed to find a valid profile, using default");
            return Self::default(
                validator_stakes.len(),
                secrecy_threshold_in_stake_ratio,
                reconstruct_threshold_in_stake_ratio,
            );
        }

        best_profile
    }

    pub fn default(
        num_validators: usize,
        secrecy_threshold_in_stake_ratio: U64F64,
        reconstruct_threshold_in_stake_ratio: U64F64,
    ) -> Self {
        Self {
            validator_weights: vec![1; num_validators],
            secrecy_threshold_in_stake_ratio,
            reconstruct_threshold_in_stake_ratio,
            reconstruct_threshold_in_weights: (U64F64::from_num(num_validators)
                * secrecy_threshold_in_stake_ratio)
                .to_num::<u64>(),
        }
    }
}

fn is_valid_profile(
    profile: &DKGRoundingProfile,
    reconstruct_threshold_in_stake_ratio: U64F64,
) -> bool {
    // ensure the reconstruction is below threshold and all validators have at least 1 weight
    profile.reconstruct_threshold_in_stake_ratio <= reconstruct_threshold_in_stake_ratio
        && profile.validator_weights.iter().all(|&w| w > 0)
}

fn compute_profile_fixed_point(
    validator_stakes: &Vec<u64>,
    weights_sum: u64,
    secrecy_threshold_in_stake_ratio: U64F64,
) -> DKGRoundingProfile {
    // Use fixed-point arithmetic to ensure the same result across machines.
    // See paper for details of the rounding algorithm
    // https://eprint.iacr.org/2024/198
    let stake_sum: u64 = validator_stakes.iter().sum::<u64>();
    let stake_sum_fixed = U64F64::from_num(stake_sum);
    let stake_per_weight: u64 = max(1, stake_sum / weights_sum);
    let stake_per_weight_fixed = U64F64::from_num(stake_per_weight);
    let mut delta_down_fixed = U64F64::from_num(0);
    let mut delta_up_fixed = U64F64::from_num(0);
    let mut validator_weights: Vec<u64> = vec![];
    for stake in validator_stakes {
        let ideal_weight_fixed = U64F64::from_num(*stake) / stake_per_weight_fixed;
        // rounded to the nearest integer
        let rounded_weight_fixed =
            (U64F64::from_num(*stake) / stake_per_weight_fixed + U64F64::from_num(0.5)).floor();
        validator_weights.push(rounded_weight_fixed.to_num::<u64>());
        if ideal_weight_fixed > rounded_weight_fixed {
            delta_down_fixed += ideal_weight_fixed - rounded_weight_fixed;
        } else {
            delta_up_fixed += rounded_weight_fixed - ideal_weight_fixed;
        }
    }
    let delta_total_fixed = delta_down_fixed + delta_up_fixed;
    let reconstruct_threshold_in_weights_fixed =
        (secrecy_threshold_in_stake_ratio * stake_sum_fixed / stake_per_weight_fixed
            + delta_up_fixed)
            .ceil();
    let reconstruct_threshold_in_weights: u64 =
        reconstruct_threshold_in_weights_fixed.to_num::<u64>();
    let stake_gap_fixed = stake_per_weight_fixed * delta_total_fixed / stake_sum_fixed;
    let reconstruct_threshold_in_stake_ratio = secrecy_threshold_in_stake_ratio + stake_gap_fixed;

    DKGRoundingProfile {
        validator_weights,
        secrecy_threshold_in_stake_ratio,
        reconstruct_threshold_in_stake_ratio,
        reconstruct_threshold_in_weights,
    }
}

#[cfg(test)]
mod tests;

pub static DEFAULT_SECRECY_THRESHOLD: Lazy<U64F64> =
    Lazy::new(|| U64F64::from_num(1) / U64F64::from_num(2));

pub static DEFAULT_RECONSTRUCT_THRESHOLD: Lazy<U64F64> =
    Lazy::new(|| U64F64::from_num(2) / U64F64::from_num(3));
