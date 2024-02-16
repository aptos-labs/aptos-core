// Copyright © Aptos Foundation

use aptos_dkg::pvss::WeightedConfig;
use fixed::types::U64F64;
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

// dkg todo: move to config file
pub const SECRECY_THRESHOLD: f64 = 0.5;
pub const RECONSTRUCT_THRESHOLD: f64 = 2.0 / 3.0;

pub fn total_weight_lower_bound(validator_stakes: &Vec<u64>) -> usize {
    // Each validator has at least 1 weight.
    validator_stakes.len()
}

pub fn total_weight_upper_bound(
    validator_stakes: &Vec<u64>,
    reconstruct_threshold_in_stake_ratio: f64,
    secrecy_threshold_in_stake_ratio: f64,
) -> usize {
    let factor = (1.0 / (reconstruct_threshold_in_stake_ratio - secrecy_threshold_in_stake_ratio))
        as usize
        + 1;
    // todo: use a better upper bound
    validator_stakes.len() * factor * 2
}

#[derive(Clone, Debug)]
pub struct DKGRounding {
    pub profile: DKGRoundingProfile,
    pub wconfig: WeightedConfig,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: &Vec<u64>,
        secrecy_threshold_in_stake_ratio: f64,
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
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
    pub secrecy_threshold_in_stake_ratio: f64,
    // The ratio of stake that always can reconstruct the randomness, e.g. 66.67%
    pub reconstruct_threshold_in_stake_ratio: f64,
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
        secrecy_threshold_in_stake_ratio: f64,
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
        assert!(total_weight_min >= validator_stakes.len());
        assert!(total_weight_max >= total_weight_min);
        assert!(secrecy_threshold_in_stake_ratio > 1.0 / 3.0);
        assert!(secrecy_threshold_in_stake_ratio <= reconstruct_threshold_in_stake_ratio);
        assert!(reconstruct_threshold_in_stake_ratio <= 2.0 / 3.0);

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

        let profile_with_total_weight_max = compute_profile_fixed_point(
            validator_stakes,
            weight_high,
            secrecy_threshold_in_stake_ratio,
        );
        if !is_valid_profile(
            &profile_with_total_weight_max,
            reconstruct_threshold_in_stake_ratio,
        ) {
            // randomness todo: alert error
            println!("[Randomness] Rounding error! The rounding algorithm is not working, temporarily using unweighted config to ensure progress. Details: total_weight_min: {}, total_weight_max: {}, profile {:?}", total_weight_min, total_weight_max, profile_with_total_weight_max);

            return Self::default(
                validator_stakes.len(),
                secrecy_threshold_in_stake_ratio,
                reconstruct_threshold_in_stake_ratio,
            );
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
        best_profile
    }

    pub fn default(
        num_validators: usize,
        secrecy_threshold_in_stake_ratio: f64,
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
        Self {
            validator_weights: vec![1; num_validators],
            secrecy_threshold_in_stake_ratio,
            reconstruct_threshold_in_stake_ratio,
            reconstruct_threshold_in_weights: (num_validators as f64 * SECRECY_THRESHOLD) as u64,
        }
    }
}

fn is_valid_profile(
    profile: &DKGRoundingProfile,
    reconstruct_threshold_in_stake_ratio: f64,
) -> bool {
    // ensure the reconstruction is below threshold and all validators have at least 1 weight
    profile.reconstruct_threshold_in_stake_ratio <= reconstruct_threshold_in_stake_ratio
        && profile.validator_weights.iter().all(|&w| w > 0)
}

#[allow(dead_code)]
fn compute_profile_floating_point(
    validator_stakes: Vec<u64>,
    weights_sum: u64,
    secrecy_threshold_in_stake_ratio: f64,
) -> DKGRoundingProfile {
    // Use float-point arithmetic may not ensure the same result across machines, so replaced with the compute_profile_fixed below.
    // See paper for details of the rounding algorithm
    // https://eprint.iacr.org/2024/198
    let hardcoded_best_rounding_threshold: f64 = 0.5;
    let stake_sum: u64 = validator_stakes.iter().sum::<u64>();
    let stake_per_weight: u64 = stake_sum / weights_sum;
    let mut delta_down: f64 = 0.0;
    let mut delta_up: f64 = 0.0;
    let mut validator_weights: Vec<u64> = vec![];
    for stake in validator_stakes {
        let ideal_weight = stake as f64 / stake_per_weight as f64;
        let rounded_weight =
            (stake as f64 / stake_per_weight as f64 + hardcoded_best_rounding_threshold).floor();
        validator_weights.push(rounded_weight as u64);
        if ideal_weight > rounded_weight {
            delta_down += ideal_weight - rounded_weight;
        } else {
            delta_up += rounded_weight - ideal_weight;
        }
    }
    let delta_total = delta_down + delta_up;
    let reconstruct_threshold_in_weights = ((secrecy_threshold_in_stake_ratio * stake_sum as f64)
        / (stake_per_weight as f64)
        + delta_up)
        .ceil() as u64;
    let stake_gap: f64 = stake_per_weight as f64 * delta_total / stake_sum as f64;
    let reconstruct_threshold_in_stake_ratio: f64 = secrecy_threshold_in_stake_ratio + stake_gap;

    DKGRoundingProfile {
        validator_weights,
        secrecy_threshold_in_stake_ratio,
        reconstruct_threshold_in_stake_ratio,
        reconstruct_threshold_in_weights,
    }
}

fn compute_profile_fixed_point(
    validator_stakes: &Vec<u64>,
    weights_sum: u64,
    secrecy_threshold_in_stake_ratio: f64,
) -> DKGRoundingProfile {
    // Use fixed-point arithmetic to ensure the same result across machines.
    // See paper for details of the rounding algorithm
    // https://eprint.iacr.org/2024/198
    let hardcoded_threshold_fixed = U64F64::from_num(0.5);
    let stake_sum: u64 = validator_stakes.iter().sum::<u64>();
    let stake_sum_fixed = U64F64::from_num(stake_sum);
    let stake_per_weight: u64 = stake_sum / weights_sum;
    let stake_per_weight_fixed = U64F64::from_num(stake_per_weight);
    let mut delta_down_fixed = U64F64::from_num(0);
    let mut delta_up_fixed = U64F64::from_num(0);
    let mut validator_weights: Vec<u64> = vec![];
    for stake in validator_stakes {
        let ideal_weight_fixed = U64F64::from_num(*stake) / stake_per_weight_fixed;
        let rounded_weight_fixed =
            (U64F64::from_num(*stake) / stake_per_weight_fixed + hardcoded_threshold_fixed).floor();
        validator_weights.push(rounded_weight_fixed.to_num::<u64>());
        if ideal_weight_fixed > rounded_weight_fixed {
            delta_down_fixed += ideal_weight_fixed - rounded_weight_fixed;
        } else {
            delta_up_fixed += rounded_weight_fixed - ideal_weight_fixed;
        }
    }
    // let stake_per_weight_fixed = U64F64::from_num(stake_sum / validator_weights.iter().sum::<u64>() as u64);
    let delta_total_fixed = delta_down_fixed + delta_up_fixed;
    let secrecy_threshold_in_stake_ratio_fixed = U64F64::from_num(secrecy_threshold_in_stake_ratio);
    let reconstruct_threshold_in_weights_fixed =
        (secrecy_threshold_in_stake_ratio_fixed * stake_sum_fixed / stake_per_weight_fixed
            + delta_up_fixed)
            .ceil();
    let reconstruct_threshold_in_weights: u64 =
        reconstruct_threshold_in_weights_fixed.to_num::<u64>();
    let stake_gap_fixed = stake_per_weight_fixed * delta_total_fixed / stake_sum_fixed;
    let reconstruct_threshold_in_stake_ratio: f64 =
        (secrecy_threshold_in_stake_ratio_fixed + stake_gap_fixed).to_num::<f64>();

    DKGRoundingProfile {
        validator_weights,
        secrecy_threshold_in_stake_ratio,
        reconstruct_threshold_in_stake_ratio,
        reconstruct_threshold_in_weights,
    }
}

#[cfg(test)]
mod tests;
