// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::cmp::{max, min};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_types::dkg::real_dkg::rounding::DKGRounding;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::vec_deque::VecDeque;
use num_integer::Integer;

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        (
            "rounding_internal",
            rounding_internal as RawSafeNative,
        ),
        (
            "rounding_v0_internal",
            rounding_v0_internal as RawSafeNative,
        ),
    ]);

    builder.make_named_natives(natives)
}

use fixed::types::U64F64;
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};

struct RoundingResult {
    ideal_total_weight: u128,
    weights: Vec<u64>,
    reconstruct_threshold_default_path: u128,
    reconstruct_threshold_fast_path: Option<u128>,
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

#[derive(Debug)]
struct ReconstructThresholdInfo {
    in_weights: BigUint,
    in_stakes: BigUint,
}

fn compute_threshold(
    secrecy_threshold_shl64: BigUint,
    weight_per_stake_shl64: BigUint,
    stake_total: BigUint,
    weight_total: BigUint,
    weight_gain_shl64: BigUint,
    weight_loss_shl64: BigUint
) -> ReconstructThresholdInfo {
    let mut final_thresh = (((weight_gain_shl64 << 64) + stake_total * secrecy_threshold_shl64 * weight_per_stake_shl64.clone()) >> 128) + BigUint::one();
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
    };
    let total_stake: BigUint = stakes.iter().map(|s|s.clone()).sum();
    let total_weight: BigUint = validator_weights.clone().into_iter().sum();
    let threshold_default_path = compute_threshold(secrecy_threshold_shl64, weight_per_stake_shl64.clone(), total_stake.clone(), total_weight.clone(), weight_gain_shl64.clone(), weight_loss_shl64.clone());
    let threshold_fast_path = fast_path_secrecy_threshold_shl64.map(|v|compute_threshold(v, weight_per_stake_shl64, total_stake, total_weight, weight_gain_shl64, weight_loss_shl64));
    Profile {
        ideal_total_weight,
        validator_weights,
        threshold_default_path,
        threshold_fast_path,
    }
}

fn rounding(
    stakes: Vec<BigUint>,
    mut secrecy_threshold_shl64: BigUint,
    mut recon_threshold_shl64: BigUint,
    fast_secrecy_thresh_shl64: Option<BigUint>,
) -> RoundingResult {
    let n = stakes.len();
    // Ensure secrecy threshold is in [0,1).
    secrecy_threshold_shl64 = min(secrecy_threshold_shl64, BigUint::from(0xffffffffffffffff_u64));
    // `recon_thresh > secrecy_thresh` should hold, otherwise it is invalid input.
    recon_threshold_shl64 = max(recon_threshold_shl64, secrecy_threshold_shl64.clone() + BigUint::one());
    recon_threshold_shl64 = min(recon_threshold_shl64, BigUint::from(1u128 << 64));
    let mut total_weight_max = BigUint::from(n) + BigUint::from(4u64);
    total_weight_max <<= 64;
    total_weight_max = total_weight_max.div_ceil(&((recon_threshold_shl64.clone() - secrecy_threshold_shl64.clone()) * BigUint::from(2u64)));
    let stakes_total: BigUint = stakes.clone().into_iter().sum();
    let bar = (stakes_total.clone() * recon_threshold_shl64.clone()) >> 64;
    let mut lo = 0;
    let mut hi = total_weight_max.to_u128().unwrap() * 2;
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
            weight_per_stake_shl64
        );
        if cur_profile.threshold_default_path.in_stakes <= bar {
            hi = ideal_weight;
            profile = cur_profile;
        } else {
            lo = ideal_weight;
        }
    }

    let Profile { ideal_total_weight, validator_weights, threshold_default_path, threshold_fast_path } = profile;

    RoundingResult {
        ideal_total_weight: ideal_total_weight.to_u128().unwrap(),
        weights: validator_weights.into_iter().map(|w|w.to_u64().unwrap()).collect(),
        reconstruct_threshold_default_path: threshold_default_path.in_weights.to_u128().unwrap(),
        reconstruct_threshold_fast_path: threshold_fast_path.map(|t|t.in_weights.to_u128().unwrap()),
    }
}

pub fn rounding_internal(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let fast_secrecy_thresh_raw = safely_pop_arg!(arguments, u128);
    let recon_thresh_raw = safely_pop_arg!(arguments, u128);
    let secrecy_thresh_raw = safely_pop_arg!(arguments, u128);
    let stakes = safely_pop_arg!(arguments, Vec<u64>);

    let stakes = stakes.into_iter().map(BigUint::from).collect();
    let secrecy_tresh_shl64 = BigUint::from(secrecy_thresh_raw);
    let recon_thresh_shl64 = BigUint::from(recon_thresh_raw);
    let fast_secrecy_thresh_shl64 = if fast_secrecy_thresh_raw == 0 {
        None
    } else {
        Some(BigUint::from(fast_secrecy_thresh_raw))
    };

    let RoundingResult {
        ideal_total_weight,
        weights,
        reconstruct_threshold_default_path,
        reconstruct_threshold_fast_path,
    } = rounding(
        stakes,
        secrecy_tresh_shl64,
        recon_thresh_shl64,
        fast_secrecy_thresh_shl64,
        );

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u128(ideal_total_weight),
        Value::vector_u64(weights),
        Value::u128(reconstruct_threshold_default_path),
        Value::struct_(Struct::pack(vec![
            Value::vector_u128(
                reconstruct_threshold_fast_path
                    .map(|x| x as u128)
                    .into_iter()
                    .collect::<Vec<_>>()
            )
        ])),
    ]))])
}

pub fn rounding_v0_internal(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let fast_secrecy_thresh_raw = safely_pop_arg!(arguments, u128);
    let recon_thresh_raw = safely_pop_arg!(arguments, u128);
    let secrecy_thresh_raw = safely_pop_arg!(arguments, u128);
    let stakes = safely_pop_arg!(arguments, Vec<u64>);
    let secrecy_thresh = U64F64::from_bits(secrecy_thresh_raw);
    let recon_thresh = U64F64::from_bits(recon_thresh_raw);
    let fast_secrecy_thresh = if fast_secrecy_thresh_raw == 0 {
        None
    } else {
        Some(U64F64::from_bits(fast_secrecy_thresh_raw))
    };
    let result = DKGRounding::new(&stakes, secrecy_thresh, recon_thresh, fast_secrecy_thresh);
    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u128(result.profile.ideal_total_weight),
        Value::vector_u64(result.profile.validator_weights.clone()),
        Value::u128(result.profile.reconstruct_threshold_in_weights as u128),
        Value::struct_(Struct::pack(vec![
            Value::vector_u128(
                result
                    .profile
                    .fast_reconstruct_threshold_in_weights
                    .map(|x| x as u128)
                    .into_iter()
                    .collect::<Vec<_>>()
            )
        ])),
    ]))])
}
