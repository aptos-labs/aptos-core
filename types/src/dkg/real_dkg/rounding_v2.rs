// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{One, ToPrimitive, Zero};
use std::cmp::{max, min};

#[derive(Clone, Debug, Default)]
pub struct RoundedV2 {
    pub ideal_total_weight: u128,
    pub weights: Vec<u64>,
    pub reconstruct_threshold_default_path: u128,
    pub reconstruct_threshold_fast_path: Option<u128>,
}

#[derive(Debug)]
struct ReconstructThresholdInfo {
    in_weights: BigUint,
    in_stakes: BigUint,
}

#[derive(Debug)]
struct Profile {
    ideal_total_weight: BigUint,
    validator_weights: Vec<BigUint>,
    threshold_default_path: ReconstructThresholdInfo,
    threshold_fast_path: Option<ReconstructThresholdInfo>,
}

impl Profile {
    fn naive(n: usize) -> Self {
        Self {
            ideal_total_weight: BigUint::from(n),
            validator_weights: vec![BigUint::one(); n],
            threshold_default_path: ReconstructThresholdInfo {
                in_weights: BigUint::one(),
                in_stakes: BigUint::one(),
            },
            threshold_fast_path: None,
        }
    }
}

pub fn main(
    stakes: Vec<BigUint>,
    mut secrecy_threshold_shl64: BigUint,
    mut recon_threshold_shl64: BigUint,
    fast_secrecy_thresh_shl64: Option<BigUint>,
) -> Result<RoundedV2> {
    let n = stakes.len();
    // Ensure secrecy threshold is in [0,1).
    secrecy_threshold_shl64 = min(
        secrecy_threshold_shl64,
        BigUint::from(0xFFFFFFFFFFFFFFFF_u64),
    );
    // `recon_thresh > secrecy_thresh` should hold, otherwise it is invalid input.
    recon_threshold_shl64 = max(
        recon_threshold_shl64,
        secrecy_threshold_shl64.clone() + BigUint::one(),
    );
    recon_threshold_shl64 = min(recon_threshold_shl64, BigUint::from(1u128 << 64));
    let mut total_weight_max = BigUint::from(n) + BigUint::from(4u64);
    total_weight_max <<= 64;
    total_weight_max = total_weight_max.div_ceil(
        &((recon_threshold_shl64.clone() - secrecy_threshold_shl64.clone()) * BigUint::from(2u64)),
    );
    let stakes_total: BigUint = stakes.clone().into_iter().sum();
    let bar = (stakes_total.clone() * recon_threshold_shl64.clone()) >> 64;
    let mut lo = 0;
    let mut hi = total_weight_max
        .to_u128()
        .ok_or_else(|| anyhow!("total_weight_max is not a u128!"))?
        * 2;
    // This^ ensures the first `ideal_weight` to try is `total_weight_max`,
    // which should always result in a valid weight assignment that satisfies `recon_threshold_shl64`.

    let mut profile = Profile::naive(n);
    while lo + 1 < hi {
        let ideal_weight = (lo + hi) / 2;
        let mut weight_per_stake_shl64 = BigUint::from(ideal_weight);
        weight_per_stake_shl64 <<= 64;
        weight_per_stake_shl64 = weight_per_stake_shl64.div_ceil(&stakes_total);
        let cur_profile = compute_profile(
            secrecy_threshold_shl64.clone(),
            fast_secrecy_thresh_shl64.clone(),
            &stakes,
            BigUint::from(ideal_weight),
            weight_per_stake_shl64,
        );
        if cur_profile.threshold_default_path.in_stakes <= bar {
            hi = ideal_weight;
            profile = cur_profile;
        } else {
            lo = ideal_weight;
        }
    }

    let Profile {
        ideal_total_weight,
        validator_weights,
        threshold_default_path,
        threshold_fast_path,
    } = profile;
    let mut weights = Vec::with_capacity(n);
    for w in validator_weights {
        let w = w.to_u64().ok_or_else(|| anyhow!("some w is not u64!"))?;
        weights.push(w);
    }
    let reconstruct_threshold_fast_path =
        if let Some(t) = threshold_fast_path {
            Some(t.in_weights.to_u128().ok_or_else(|| {
                anyhow!("reconstruct_threshold_fast_path.in_weights is not a u128!")
            })?)
        } else {
            None
        };

    Ok(RoundedV2 {
        ideal_total_weight: ideal_total_weight
            .to_u128()
            .ok_or_else(|| anyhow!("ideal_total_weight is not a u128"))?,
        weights,
        reconstruct_threshold_default_path: threshold_default_path
            .in_weights
            .to_u128()
            .ok_or_else(|| anyhow!("threshold_default_path.in_weights is not a u128!"))?,
        reconstruct_threshold_fast_path,
    })
}

fn compute_threshold(
    secrecy_threshold_shl64: BigUint,
    weight_per_stake_shl64: BigUint,
    stake_total: BigUint,
    weight_total: BigUint,
    weight_gain_shl64: BigUint,
    weight_loss_shl64: BigUint,
) -> ReconstructThresholdInfo {
    let mut final_thresh = (((weight_gain_shl64 << 64)
        + stake_total * secrecy_threshold_shl64 * weight_per_stake_shl64.clone())
        >> 128)
        + BigUint::one();
    final_thresh = min(final_thresh, weight_total);
    let mut stakes_required = final_thresh.clone();
    stakes_required <<= 64;
    stakes_required += weight_loss_shl64;
    stakes_required = stakes_required.div_ceil(&weight_per_stake_shl64);
    ReconstructThresholdInfo {
        in_weights: final_thresh,
        in_stakes: stakes_required,
    }
}

fn compute_profile(
    secrecy_threshold_shl64: BigUint,
    fast_path_secrecy_threshold_shl64: Option<BigUint>,
    stakes: &[BigUint],
    ideal_total_weight: BigUint,
    weight_per_stake_shl64: BigUint,
) -> Profile {
    let n = stakes.len();
    let mut validator_weights = Vec::with_capacity(n);
    let mut weight_loss_shl64 = BigUint::zero();
    let mut weight_gain_shl64 = BigUint::zero();
    for stake in stakes {
        let ideal_weight_shl64 = weight_per_stake_shl64.clone() * stake;
        let mut rounded_weight = ideal_weight_shl64.clone() + BigUint::from(1u64 << 63);
        rounded_weight >>= 64;

        validator_weights.push(rounded_weight.clone());
        let rounded_weight_shl64 = rounded_weight << 64;
        if ideal_weight_shl64 > rounded_weight_shl64 {
            weight_loss_shl64 += ideal_weight_shl64 - rounded_weight_shl64;
        } else {
            weight_gain_shl64 += rounded_weight_shl64 - ideal_weight_shl64;
        }
    }
    let total_stake: BigUint = stakes.iter().cloned().sum();
    let total_weight: BigUint = validator_weights.clone().into_iter().sum();
    let threshold_default_path = compute_threshold(
        secrecy_threshold_shl64,
        weight_per_stake_shl64.clone(),
        total_stake.clone(),
        total_weight.clone(),
        weight_gain_shl64.clone(),
        weight_loss_shl64.clone(),
    );
    let threshold_fast_path = fast_path_secrecy_threshold_shl64.map(|v| {
        compute_threshold(
            v,
            weight_per_stake_shl64,
            total_stake,
            total_weight,
            weight_gain_shl64,
            weight_loss_shl64,
        )
    });
    Profile {
        ideal_total_weight,
        validator_weights,
        threshold_default_path,
        threshold_fast_path,
    }
}
