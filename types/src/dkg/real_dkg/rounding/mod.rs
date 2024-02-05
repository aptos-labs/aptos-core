// Copyright © Aptos Foundation

use aptos_dkg::pvss::WeightedConfig;
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

// dkg todo: move to config file
pub const WEIGHT_PER_VALIDATOR_MIN: usize = 1;
pub const WEIGHT_PER_VALIDATOR_MAX: usize = 30;
pub const STEP_SIZE: usize = 1;
pub const SECRECY_THRESHOLD: f64 = 0.5;
pub const RECONSTRUCT_THRESHOLD: f64 = 0.6667;
// assuming 500 validator each has 100 shares
pub const MAX_STEPS: usize = 50_000;

#[derive(Clone, Debug)]
pub struct DKGRounding {
    pub profile: DKGRoundingProfile,
    pub wconfig: WeightedConfig,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: Vec<u64>,
        total_weight_min: usize,
        total_weight_max: usize,
        step_size: usize,
        secrecy_threshold_in_stake_ratio: f64,
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
        let profile = DKGRoundingProfile::new(
            validator_stakes.clone(),
            total_weight_min,
            total_weight_max,
            step_size,
            secrecy_threshold_in_stake_ratio,
            reconstruct_threshold_in_stake_ratio,
        );

        let total_weights = profile.validator_weights.iter().sum::<usize>();

        if total_weights > total_weight_max {
            // dkg todo: add alert here
            println!(
                "[DKG] error: total_weights {} is larger than threshold {}",
                total_weights, total_weight_max
            );
        }

        let wconfig = WeightedConfig::new(
            profile.reconstruct_threshold_in_weights,
            profile.validator_weights.clone(),
        )
        .unwrap();

        Self { profile, wconfig }
    }
}

#[derive(Clone)]
pub struct DKGRoundingProfile {
    // calculated weights for each validator after rounding
    pub validator_weights: Vec<usize>,
    // The ratio of stake that may reveal the randomness, e.g. 50%
    pub secrecy_threshold_in_stake_ratio: f64,
    // The ratio of stake that always can reconstruct the randomness, e.g. 66.67%
    pub reconstruct_threshold_in_stake_ratio: f64,
    // The number of weights needed to reconstruct the randomness
    pub reconstruct_threshold_in_weights: usize,
}

impl Debug for DKGRoundingProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "total_weight: {}, ",
            self.validator_weights.iter().sum::<usize>()
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
        validator_stakes: Vec<u64>,
        total_weight_min: usize,
        total_weight_max: usize,
        step_size: usize,
        secrecy_threshold_in_stake_ratio: f64,
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
        assert!(total_weight_min >= validator_stakes.len());
        assert!(total_weight_max >= total_weight_min);
        assert!(step_size > 0);
        assert!(secrecy_threshold_in_stake_ratio > 1.0 / 3.0);
        assert!(secrecy_threshold_in_stake_ratio <= reconstruct_threshold_in_stake_ratio);
        assert!(secrecy_threshold_in_stake_ratio <= 2.0 / 3.0);

        let mut step = 0;
        // Search for the feasible rounding profile until found.
        loop {
            let total_weight = total_weight_min + step_size * step;
            step += 1;

            let profile = compute_profile(
                validator_stakes.clone(),
                total_weight,
                secrecy_threshold_in_stake_ratio,
            );
            if step > MAX_STEPS {
                return profile;
            }

            // This check makes sure the randomness is live.
            if profile.reconstruct_threshold_in_stake_ratio > reconstruct_threshold_in_stake_ratio {
                continue;
            }

            // Make sure each validator has at least 1 weight.
            if profile.validator_weights.iter().any(|w| *w == 0) {
                continue;
            }

            return profile;
        }
    }
}

#[allow(clippy::needless_range_loop)]
pub fn compute_profile(
    validator_stakes: Vec<u64>,
    weights_sum: usize,
    secrecy_threshold_in_stake_ratio: f64,
) -> DKGRoundingProfile {
    let hardcoded_best_rounding_threshold = 0.5;
    let stake_sum = validator_stakes.iter().sum::<u64>();
    let stake_per_weight = stake_sum / weights_sum as u64;
    let fractions = validator_stakes
        .iter()
        .map(|stake| {
            (*stake as f64 / stake_per_weight as f64) - ((stake / stake_per_weight) as f64)
        })
        .collect::<Vec<f64>>();
    let mut delta_down = 0.0;
    let mut delta_up = 0.0;
    for j in 0..fractions.len() {
        if fractions[j] + hardcoded_best_rounding_threshold >= 1.0 {
            delta_up += 1.0 - fractions[j];
        } else {
            delta_down += fractions[j];
        }
    }
    let delta_total = delta_down + delta_up;

    let validator_weights = validator_stakes
        .iter()
        .map(|stake| {
            (*stake as f64 / stake_per_weight as f64 + hardcoded_best_rounding_threshold) as usize
        })
        .collect::<Vec<usize>>();

    let reconstruct_threshold_in_weights = ((stake_sum as f64) / (stake_per_weight as f64)
        * secrecy_threshold_in_stake_ratio
        + delta_up)
        .ceil() as usize;
    //dkg todo - productionize - double check if float number operations are deterministic across platform

    let stake_gap = stake_per_weight as f64 * delta_total / stake_sum as f64;
    let reconstruct_threshold_in_stake_ratio = secrecy_threshold_in_stake_ratio + stake_gap;

    DKGRoundingProfile {
        validator_weights,
        secrecy_threshold_in_stake_ratio,
        reconstruct_threshold_in_stake_ratio,
        reconstruct_threshold_in_weights,
    }
}

#[cfg(test)]
mod tests;
