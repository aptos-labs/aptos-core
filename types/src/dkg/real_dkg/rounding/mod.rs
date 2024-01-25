// Copyright © Aptos Foundation

use aptos_dkg::pvss::WeightedConfig;
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

pub const WEIGHT_PER_VALIDATOR_MIN: usize = 1;
pub const WEIGHT_PER_VALIDATOR_MAX: usize = 30;
pub const STEPS: usize = 1_000;
pub const STAKE_GAP_THRESHOLD: f64 = 0.1;
pub const RECONSTRUCT_THRESHOLD: f64 = 0.5;

#[derive(Clone, Debug)]
pub struct DKGRounding {
    pub profile: DKGRoundingProfile,
    pub wconfig: WeightedConfig,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: Vec<u64>,
        stake_gap_threshold: f64,
        weight_per_validator_min: usize,
        weight_per_validator_max: usize,
        steps: usize,
        reconstruct_threshold: f64,
    ) -> Self {
        let profile = DKGRoundingProfile::new(
            validator_stakes.clone(),
            stake_gap_threshold,
            weight_per_validator_min,
            weight_per_validator_max,
            steps,
            reconstruct_threshold,
        );

        if profile.stake_gap > stake_gap_threshold {
            // dkg todo: add alert here
            println!(
                "[DKG] error: stake_gap {} is larger than threshold {}",
                profile.stake_gap, stake_gap_threshold
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
    // The extra percentage of stake that is needed to reconstruct the randomness due to rounding,
    // i.e., reconstruction needs reconstruct_threshold + stake_gap honest stakes to reconstruct the randomness,
    pub stake_gap: f64,
    pub reconstruct_threshold_in_weights: usize,
}

impl Debug for DKGRoundingProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "stake_gap: {}, ", self.stake_gap)?;
        write!(
            f,
            "total_weight: {}, ",
            self.validator_weights.iter().sum::<usize>()
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
        stake_gap_threshold: f64,
        weight_per_validator_min: usize,
        weight_per_validator_max: usize,
        steps: usize,
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
        assert!(0.0 < stake_gap_threshold && stake_gap_threshold < 1.0);
        assert!(
            0 < weight_per_validator_min && weight_per_validator_min <= weight_per_validator_max
        );
        assert!(steps > 0);
        assert!(reconstruct_threshold_in_stake_ratio > 0.0);

        let validator_num = validator_stakes.len();
        let total_weight_min = weight_per_validator_min * validator_num;
        let total_weight_max = weight_per_validator_max * validator_num;
        let mut maybe_best_profile: Option<DKGRoundingProfile> = None;

        for step in 0..steps {
            let total_weight =
                total_weight_min + (total_weight_max - total_weight_min) * step / steps;

            let profile = compute_profile(
                validator_stakes.clone(),
                total_weight,
                reconstruct_threshold_in_stake_ratio,
            );

            assert!(profile.stake_gap < 1.0);

            if maybe_best_profile.is_none() {
                maybe_best_profile = Some(profile.clone());
            }

            // This check makes sure the randomness is live: 2/3 stakes can reconstruct the randomness.
            if reconstruct_threshold_in_stake_ratio + profile.stake_gap > 2.0 / 3.0 {
                continue;
            }

            // Make sure each validator has at least 1 weight.
            if profile.validator_weights.iter().any(|w| *w == 0) {
                continue;
            }

            if maybe_best_profile.as_ref().unwrap().stake_gap > profile.stake_gap {
                maybe_best_profile = Some(profile.clone());
            }

            if profile.stake_gap <= stake_gap_threshold {
                break;
            }
        }
        maybe_best_profile.unwrap()
    }
}

#[allow(clippy::needless_range_loop)]
pub fn compute_profile(
    validator_stakes: Vec<u64>,
    weights_sum: usize,
    reconstruct_threshold_in_stake_ratio: f64,
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
        * reconstruct_threshold_in_stake_ratio
        + delta_up)
        .ceil() as usize;
    //dkg todo - productionize - double check if float number operations are deterministic across platform

    let stake_gap = stake_per_weight as f64 * delta_total / stake_sum as f64;

    DKGRoundingProfile {
        validator_weights,
        stake_gap,
        reconstruct_threshold_in_weights,
    }
}

#[cfg(test)]
mod tests;
