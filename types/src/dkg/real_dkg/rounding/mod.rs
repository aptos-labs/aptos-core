// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
use velor_dkg::pvss::WeightedConfig;
use fixed::types::U64F64;
use once_cell::sync::Lazy;
use std::{
    cmp::{max, min},
    fmt,
    fmt::{Debug, Formatter},
};

pub fn total_weight_lower_bound(validator_stakes: &[u64]) -> usize {
    // Each validator has at least 1 weight.
    validator_stakes.len()
}

/// Compute the smallest `stake_per_weight` which guarantees a valid rounding.
/// Output the corresponding estimated total weight, which is:
/// `(num_validators/2 + 2)/(reconstruct_threshold - secrecy_threshold)`.
///
/// Here is an example with `secrecy_threshold_in_stake_ratio=1/2, reconstruct_threshold_in_stake_ratio=2/3`.
/// Say the estimated total weight is `3n+12`, where `n` is the number of validators.
/// After `compute_profile_fixed_point()` processing,
/// - A validator with stake `cur_stake` gets rounded weight `round((3*n+12)*cur_stake/stake_total)`;
/// - `rounding_weight_gain_total` and `rounding_weight_loss_total` are determined, whose sum is at most `n/2` (since everyone's rounding error is at most `1/2`);
/// - `reconstruction_threshold` is set to be `ceil(1.5n + 6 + rounding_weight_gain_total) + 1`.
/// Now, a validator subset of stake ratio `r` has `weight_sub_total` in range:
///   `[(3*n+12)*r - rounding_weight_loss_total, (3*n+12)*r + rounding_weight_gain_total]`
/// - when `r <= 1/2`, `weight_sub_total <= 1.5*n + 6 + rounding_weight_gain_total < reconstruction_threshold`.
/// - when `r > 2/3`, `weight_sub_total >= 2*n + 8 - rounding_weight_loss_total >= 1.5*n + 8 + rounding_weight_gain_total > reconstruction_threshold`.

pub fn total_weight_upper_bound(
    validator_stakes: &[u64],
    mut reconstruct_threshold_in_stake_ratio: U64F64,
    secrecy_threshold_in_stake_ratio: U64F64,
) -> usize {
    reconstruct_threshold_in_stake_ratio = max(
        reconstruct_threshold_in_stake_ratio,
        secrecy_threshold_in_stake_ratio + U64F64::DELTA,
    );
    let two = U64F64::from_num(2);
    let n = U64F64::from_num(validator_stakes.len());
    ((n / two + two) / (reconstruct_threshold_in_stake_ratio - secrecy_threshold_in_stake_ratio))
        .ceil()
        .to_num::<usize>()
}

#[derive(Clone, Debug)]
pub struct DKGRounding {
    /// Currently either "binary_search" or "infallible".
    pub rounding_method: String,
    pub profile: DKGRoundingProfile,
    pub wconfig: WeightedConfig,
    pub fast_wconfig: Option<WeightedConfig>,
    pub rounding_error: Option<String>,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: &Vec<u64>,
        secrecy_threshold_in_stake_ratio: U64F64,
        mut reconstruct_threshold_in_stake_ratio: U64F64,
        fast_secrecy_threshold_in_stake_ratio: Option<U64F64>,
    ) -> Self {
        reconstruct_threshold_in_stake_ratio = max(
            reconstruct_threshold_in_stake_ratio,
            secrecy_threshold_in_stake_ratio + U64F64::DELTA,
        );

        let total_weight_min = total_weight_lower_bound(validator_stakes);
        let total_weight_max = total_weight_upper_bound(
            validator_stakes,
            reconstruct_threshold_in_stake_ratio,
            secrecy_threshold_in_stake_ratio,
        );

        let (profile, rounding_error, rounding_method) = match DKGRoundingProfile::new(
            validator_stakes,
            total_weight_min,
            total_weight_max,
            secrecy_threshold_in_stake_ratio,
            reconstruct_threshold_in_stake_ratio,
            fast_secrecy_threshold_in_stake_ratio,
        ) {
            Ok(profile) => (profile, None, "binary_search".to_string()),
            Err(e) => {
                let profile = DKGRoundingProfile::infallible(
                    validator_stakes,
                    secrecy_threshold_in_stake_ratio,
                    reconstruct_threshold_in_stake_ratio,
                    fast_secrecy_threshold_in_stake_ratio,
                );
                (profile, Some(format!("{e}")), "infallible".to_string())
            },
        };
        let wconfig = WeightedConfig::new(
            profile.reconstruct_threshold_in_weights as usize,
            profile
                .validator_weights
                .iter()
                .map(|w| *w as usize)
                .collect(),
        )
        .unwrap();

        let fast_wconfig = profile.fast_reconstruct_threshold_in_weights.map(
            |fast_reconstruct_threshold_in_weights| {
                WeightedConfig::new(
                    fast_reconstruct_threshold_in_weights as usize,
                    profile
                        .validator_weights
                        .iter()
                        .map(|w| *w as usize)
                        .collect(),
                )
                .unwrap()
            },
        );

        Self {
            rounding_method,
            profile,
            wconfig,
            fast_wconfig,
            rounding_error,
        }
    }
}

#[derive(Clone, Default)]
pub struct DKGRoundingProfile {
    // calculated weights for each validator after rounding
    pub validator_weights: Vec<u64>,
    // The ratio of stake that may reveal the randomness, e.g. 50%
    pub secrecy_threshold_in_stake_ratio: U64F64,
    // The ratio of stake that always can reconstruct the randomness, e.g. 66.67%
    pub reconstruct_threshold_in_stake_ratio: U64F64,
    // The number of weights needed to reconstruct the randomness
    pub reconstruct_threshold_in_weights: u64,
    // The ratio of stake that always can reconstruct the randomness for the fast path, e.g. 66.67% + delta
    pub fast_reconstruct_threshold_in_stake_ratio: Option<U64F64>,
    // The number of weights needed to reconstruct the randomness for the fast path
    pub fast_reconstruct_threshold_in_weights: Option<u64>,
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
        write!(
            f,
            "fast_reconstruct_threshold_in_stake_ratio: {:?}, ",
            self.fast_reconstruct_threshold_in_stake_ratio
        )?;
        write!(
            f,
            "fast_reconstruct_threshold_in_weights: {:?}, ",
            self.fast_reconstruct_threshold_in_weights
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
        fast_secrecy_threshold_in_stake_ratio: Option<U64F64>,
    ) -> anyhow::Result<Self> {
        ensure!(total_weight_min >= validator_stakes.len());
        ensure!(total_weight_max >= total_weight_min);
        ensure!(secrecy_threshold_in_stake_ratio * U64F64::from_num(3) > U64F64::from_num(1));
        ensure!(secrecy_threshold_in_stake_ratio < reconstruct_threshold_in_stake_ratio);
        ensure!(reconstruct_threshold_in_stake_ratio * U64F64::from_num(3) <= U64F64::from_num(2));

        let stake_total: u64 = validator_stakes.iter().sum();
        let mut weight_low = total_weight_min as u64;
        let mut weight_high = total_weight_max as u64;
        let mut best_profile = compute_profile_fixed_point(
            validator_stakes,
            max(
                U64F64::from_num(1),
                U64F64::from_num(stake_total) / U64F64::from_num(weight_low),
            ),
            secrecy_threshold_in_stake_ratio,
            fast_secrecy_threshold_in_stake_ratio,
        );

        if is_valid_profile(&best_profile, reconstruct_threshold_in_stake_ratio) {
            return Ok(best_profile);
        }

        // binary search for the minimum weight that satisfies the conditions
        while weight_low <= weight_high {
            let weight_mid = weight_low + (weight_high - weight_low) / 2;
            let stake_per_weight = max(
                U64F64::from_num(1),
                U64F64::from_num(stake_total) / U64F64::from_num(weight_mid),
            );
            let profile = compute_profile_fixed_point(
                validator_stakes,
                stake_per_weight,
                secrecy_threshold_in_stake_ratio,
                fast_secrecy_threshold_in_stake_ratio,
            );

            // Check if the current weight satisfies the conditions
            if is_valid_profile(&profile, reconstruct_threshold_in_stake_ratio) {
                best_profile = profile;
                weight_high = weight_mid - 1;
            } else {
                weight_low = weight_mid + 1;
            }
        }

        if is_valid_profile(&best_profile, reconstruct_threshold_in_stake_ratio) {
            Ok(best_profile)
        } else {
            bail!(
                "could not find a valid weight in the given weight range [{}, {}]",
                total_weight_min,
                total_weight_max
            );
        }
    }

    /// Assign weights using a `stake_per_weight` that guarantees liveness and privacy.
    /// See comments of `total_weight_upper_bound()` for the detailed math.
    pub fn infallible(
        validator_stakes: &Vec<u64>,
        mut secrecy_threshold_in_stake_ratio: U64F64,
        mut reconstruct_threshold_in_stake_ratio: U64F64,
        fast_secrecy_threshold_in_stake_ratio: Option<U64F64>,
    ) -> Self {
        let one = U64F64::from_num(1);
        secrecy_threshold_in_stake_ratio = min(one, secrecy_threshold_in_stake_ratio);
        reconstruct_threshold_in_stake_ratio = min(one, reconstruct_threshold_in_stake_ratio);
        reconstruct_threshold_in_stake_ratio = max(
            secrecy_threshold_in_stake_ratio,
            reconstruct_threshold_in_stake_ratio,
        );

        let stake_total = U64F64::from_num(validator_stakes.clone().into_iter().sum::<u64>());

        let estimated_weight_total = total_weight_upper_bound(
            validator_stakes,
            reconstruct_threshold_in_stake_ratio,
            secrecy_threshold_in_stake_ratio,
        );
        let stake_per_weight = stake_total / U64F64::from_num(estimated_weight_total);
        compute_profile_fixed_point(
            validator_stakes,
            stake_per_weight,
            secrecy_threshold_in_stake_ratio,
            fast_secrecy_threshold_in_stake_ratio,
        )
    }
}

fn is_valid_profile(
    profile: &DKGRoundingProfile,
    reconstruct_threshold_in_stake_ratio: U64F64,
) -> bool {
    // ensure the reconstruction is below threshold, and the fast path threshold is valid
    profile.reconstruct_threshold_in_stake_ratio <= reconstruct_threshold_in_stake_ratio
        && (profile.fast_reconstruct_threshold_in_stake_ratio.is_none()
            || profile.fast_reconstruct_threshold_in_stake_ratio.unwrap() <= U64F64::from_num(1))
}

fn compute_profile_fixed_point(
    validator_stakes: &Vec<u64>,
    stake_per_weight: U64F64,
    secrecy_threshold_in_stake_ratio: U64F64,
    maybe_fast_secrecy_threshold_in_stake_ratio: Option<U64F64>,
) -> DKGRoundingProfile {
    // Use fixed-point arithmetic to ensure the same result across machines.
    // See paper for details of the rounding algorithm
    // https://eprint.iacr.org/2024/198
    let one = U64F64::from_num(1);
    let stake_sum: u64 = validator_stakes.iter().sum::<u64>();
    let stake_sum_fixed = U64F64::from_num(stake_sum);
    let mut delta_down_fixed = U64F64::from_num(0);
    let mut delta_up_fixed = U64F64::from_num(0);
    let mut validator_weights: Vec<u64> = vec![];
    for stake in validator_stakes {
        let ideal_weight_fixed = U64F64::from_num(*stake) / stake_per_weight;
        // rounded to the nearest integer
        let rounded_weight_fixed = (ideal_weight_fixed + (one / 2)).floor();
        let rounded_weight = rounded_weight_fixed.to_num::<u64>();
        validator_weights.push(rounded_weight);
        if ideal_weight_fixed > rounded_weight_fixed {
            delta_down_fixed += ideal_weight_fixed - rounded_weight_fixed;
        } else {
            delta_up_fixed += rounded_weight_fixed - ideal_weight_fixed;
        }
    }
    let weight_total: u64 = validator_weights.clone().into_iter().sum();
    let delta_total_fixed = delta_down_fixed + delta_up_fixed;
    let reconstruct_threshold_in_weights_fixed =
        (secrecy_threshold_in_stake_ratio * stake_sum_fixed / stake_per_weight + delta_up_fixed)
            .ceil()
            + one;
    let reconstruct_threshold_in_weights: u64 = min(
        weight_total,
        reconstruct_threshold_in_weights_fixed.to_num::<u64>(),
    );
    let stake_gap_fixed = stake_per_weight * delta_total_fixed / stake_sum_fixed;
    let reconstruct_threshold_in_stake_ratio = secrecy_threshold_in_stake_ratio + stake_gap_fixed;

    let (fast_reconstruct_threshold_in_stake_ratio, fast_reconstruct_threshold_in_weights) =
        if let Some(fast_secrecy_threshold_in_stake_ratio) =
            maybe_fast_secrecy_threshold_in_stake_ratio
        {
            let recon_threshold = fast_secrecy_threshold_in_stake_ratio + stake_gap_fixed;
            let recon_weight = min(
                weight_total,
                ((fast_secrecy_threshold_in_stake_ratio * stake_sum_fixed / stake_per_weight
                    + delta_up_fixed)
                    .ceil()
                    + one)
                    .to_num::<u64>(),
            );
            (Some(recon_threshold), Some(recon_weight))
        } else {
            (None, None)
        };

    DKGRoundingProfile {
        validator_weights,
        secrecy_threshold_in_stake_ratio,
        reconstruct_threshold_in_stake_ratio,
        reconstruct_threshold_in_weights,
        fast_reconstruct_threshold_in_stake_ratio,
        fast_reconstruct_threshold_in_weights,
    }
}

#[cfg(test)]
mod tests;

pub static DEFAULT_SECRECY_THRESHOLD: Lazy<U64F64> =
    Lazy::new(|| U64F64::from_num(1) / U64F64::from_num(2));

pub static DEFAULT_RECONSTRUCT_THRESHOLD: Lazy<U64F64> =
    Lazy::new(|| U64F64::from_num(2) / U64F64::from_num(3));

pub static DEFAULT_FAST_PATH_SECRECY_THRESHOLD: Lazy<U64F64> =
    Lazy::new(|| U64F64::from_num(2) / U64F64::from_num(3));
