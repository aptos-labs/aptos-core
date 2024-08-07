// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::real_dkg::rounding::{
    is_valid_profile, total_weight_lower_bound, total_weight_upper_bound, DKGRounding,
    DKGRoundingProfile, DEFAULT_FAST_PATH_SECRECY_THRESHOLD, DEFAULT_RECONSTRUCT_THRESHOLD,
    DEFAULT_SECRECY_THRESHOLD,
};
use aptos_dkg::pvss::WeightedConfig;
use claims::assert_le;
use fixed::types::U64F64;
use rand::{thread_rng, Rng};
use std::ops::Deref;

#[test]
fn compute_mainnet_rounding() {
    let validator_stakes = MAINNET_STAKES.to_vec();
    let dkg_rounding = DKGRounding::new(
        &validator_stakes,
        *DEFAULT_SECRECY_THRESHOLD.deref(),
        *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
        Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD.deref()),
    );
    println!("mainnet rounding profile: {:?}", dkg_rounding.profile);
    // Result:
    // mainnet rounding profile: total_weight: 414, secrecy_threshold_in_stake_ratio: 0.5, reconstruct_threshold_in_stake_ratio: 0.60478401144595166257, reconstruct_threshold_in_weights: 228, fast_reconstruct_threshold_in_stake_ratio: Some(0.7714506781126183292), fast_reconstruct_threshold_in_weights: Some(335), validator_weights: [7, 5, 6, 6, 5, 1, 6, 6, 1, 5, 6, 5, 1, 7, 1, 6, 6, 1, 2, 1, 6, 3, 2, 1, 1, 4, 3, 2, 5, 5, 5, 1, 1, 4, 1, 1, 1, 7, 5, 1, 1, 2, 6, 1, 6, 1, 3, 5, 5, 1, 5, 5, 3, 2, 5, 1, 6, 3, 6, 1, 1, 3, 1, 5, 1, 9, 1, 1, 1, 6, 1, 5, 7, 4, 6, 1, 5, 6, 5, 5, 3, 1, 6, 7, 6, 1, 3, 1, 1, 1, 1, 1, 1, 7, 2, 1, 6, 7, 1, 1, 1, 1, 5, 3, 1, 2, 3, 1, 1, 1, 1, 4, 1, 1, 1, 2, 1, 6, 7, 5, 1, 5, 1, 6, 1, 2, 3, 2, 2]

    let total_weight_min = total_weight_lower_bound(&validator_stakes);
    let total_weight_max = total_weight_upper_bound(
        &validator_stakes,
        *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
        *DEFAULT_SECRECY_THRESHOLD.deref(),
    );
    let total_weight = dkg_rounding.profile.validator_weights.iter().sum::<u64>();
    assert!(total_weight >= total_weight_min as u64);
    assert!(total_weight <= total_weight_max as u64);

    assert!(is_valid_profile(
        &dkg_rounding.profile,
        *DEFAULT_RECONSTRUCT_THRESHOLD.deref()
    ));
}

#[test]
fn test_rounding_single_validator() {
    let validator_stakes = vec![1_000_000];
    let dkg_rounding = DKGRounding::new(
        &validator_stakes,
        *DEFAULT_SECRECY_THRESHOLD.deref(),
        *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
        Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD.deref()),
    );
    let wconfig = WeightedConfig::new(1, vec![1]).unwrap();
    assert_eq!(dkg_rounding.wconfig, wconfig);
}

#[test]
fn test_rounding_equal_stakes() {
    let num_runs = 100;
    let mut rng = rand::thread_rng();
    for _ in 0..num_runs {
        let validator_num = rng.gen_range(100, 500);
        let validator_stakes = vec![1_000_000; validator_num];
        let dkg_rounding = DKGRounding::new(
            &validator_stakes,
            *DEFAULT_SECRECY_THRESHOLD.deref(),
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD.deref()),
        );
        let wconfig = WeightedConfig::new(
            (U64F64::from_num(validator_num) * *DEFAULT_SECRECY_THRESHOLD.deref())
                .ceil()
                .to_num::<usize>()
                + 1,
            vec![1; validator_num],
        )
        .unwrap();
        assert_eq!(dkg_rounding.wconfig, wconfig);
    }
}

#[test]
fn test_rounding_small_stakes() {
    let num_runs = 100;
    let mut rng = rand::thread_rng();
    for _ in 0..num_runs {
        let validator_num = rng.gen_range(1, 500);
        let mut validator_stakes = vec![];
        for _ in 0..validator_num {
            validator_stakes.push(rng.gen_range(1, 10));
        }
        let dkg_rounding = DKGRounding::new(
            &validator_stakes,
            *DEFAULT_SECRECY_THRESHOLD.deref(),
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD.deref()),
        );

        let total_weight_min = total_weight_lower_bound(&validator_stakes);
        let total_weight_max = total_weight_upper_bound(
            &validator_stakes,
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            *DEFAULT_SECRECY_THRESHOLD.deref(),
        );
        let total_weight = dkg_rounding.profile.validator_weights.iter().sum::<u64>();
        assert!(total_weight >= total_weight_min as u64);
        assert!(total_weight <= total_weight_max as u64);
        assert!(is_valid_profile(
            &dkg_rounding.profile,
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref()
        ));
    }
}

#[test]
fn test_rounding_uniform_distribution() {
    let num_runs = 100;
    let mut rng = rand::thread_rng();
    // assuming each validator has a stake between 1_000_000 and 50_000_000, following uniform distribution
    // randomly generate 100~500 validators' stake distribution
    for _ in 0..num_runs {
        let validator_num = rng.gen_range(100, 500);
        let mut validator_stakes = vec![];
        for _ in 0..validator_num {
            validator_stakes.push(rng.gen_range(1_000_000, 50_000_000));
        }
        let dkg_rounding = DKGRounding::new(
            &validator_stakes,
            *DEFAULT_SECRECY_THRESHOLD.deref(),
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD.deref()),
        );

        let total_weight_min = total_weight_lower_bound(&validator_stakes);
        let total_weight_max = total_weight_upper_bound(
            &validator_stakes,
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            *DEFAULT_SECRECY_THRESHOLD.deref(),
        );
        let total_weight = dkg_rounding.profile.validator_weights.iter().sum::<u64>();
        assert!(total_weight >= total_weight_min as u64);
        assert!(total_weight <= total_weight_max as u64);
        assert!(is_valid_profile(
            &dkg_rounding.profile,
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref()
        ));
    }
}

#[cfg(test)]
pub fn generate_approximate_zipf(size: usize, a: u64, b: u64, exponent: f64) -> Vec<u64> {
    use num_traits::Float;

    let mut rng = rand::thread_rng();
    (0..size)
        .map(|_| {
            let random_uniform = rng.gen_range(0.0, 1.0);
            let approximate_value =
                a + ((b - a + 1) as f64 * (1.0 - random_uniform).powf(exponent)) as u64;
            // Adjust value to be within the specified range [a, b]
            approximate_value.clamp(a, b)
        })
        .collect()
}

#[test]
fn test_rounding_zipf_distribution() {
    let num_runs = 100;
    let mut rng = rand::thread_rng();
    // assuming each validator has a stake between 1_000_000 and 50_000_000, following zipf distribution
    // randomly generate 100~500 validators' stake distribution
    for _ in 0..num_runs {
        let validator_num = rng.gen_range(100, 500);
        let validator_stakes = generate_approximate_zipf(validator_num, 1_000_000, 50_000_000, 5.0);
        let dkg_rounding = DKGRounding::new(
            &validator_stakes,
            *DEFAULT_SECRECY_THRESHOLD.deref(),
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD.deref()),
        );

        let total_weight_min = total_weight_lower_bound(&validator_stakes);
        let total_weight_max = total_weight_upper_bound(
            &validator_stakes,
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
            *DEFAULT_SECRECY_THRESHOLD.deref(),
        );
        let total_weight = dkg_rounding.profile.validator_weights.iter().sum::<u64>();
        assert!(total_weight >= total_weight_min as u64);
        assert!(total_weight <= total_weight_max as u64);
        assert!(is_valid_profile(
            &dkg_rounding.profile,
            *DEFAULT_RECONSTRUCT_THRESHOLD.deref()
        ));
    }
}

#[test]
fn test_infallible_rounding_with_mainnet() {
    let profile = DKGRoundingProfile::infallible(
        &MAINNET_STAKES.to_vec(),
        *DEFAULT_SECRECY_THRESHOLD,
        *DEFAULT_RECONSTRUCT_THRESHOLD,
        Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD),
    );
    println!("profile={:?}", profile);
}

#[test]
fn test_infallible_rounding_brute_force() {
    let mut rng = thread_rng();
    let two = U64F64::from_num(2);
    for n in 1..=20 {
        let n_fixed = U64F64::from_num(n);
        let n_halved = n_fixed / 2;
        for _ in 0..10 {
            let stakes: Vec<u64> = (0..n).map(|_| rng.gen_range(1, 100)).collect();
            let stake_total = U64F64::from_num(stakes.clone().into_iter().sum::<u64>());
            let stake_secrecy_threshold = stake_total * *DEFAULT_SECRECY_THRESHOLD;
            let stake_reconstruct_threshold = stake_total * *DEFAULT_RECONSTRUCT_THRESHOLD;
            let fast_path_stake_secrecy_threshold =
                stake_total * *DEFAULT_FAST_PATH_SECRECY_THRESHOLD;
            let profile = DKGRoundingProfile::infallible(
                &stakes,
                *DEFAULT_SECRECY_THRESHOLD,
                *DEFAULT_RECONSTRUCT_THRESHOLD,
                Some(*DEFAULT_FAST_PATH_SECRECY_THRESHOLD),
            );
            println!("n={}, stakes={:?}, profile={:?}", n, stakes, profile);
            let num_subsets: u64 = 1 << n;
            let weight_total = U64F64::from_num(profile.validator_weights.iter().sum::<u64>());

            // With default thresholds, weight_total <= (n/2 + 2)/(recon_threshold - secrecy_threshold) + rounding_weight_gain_total <= ceil((n/2 + 2)/(recon_threshold - secrecy_threshold)) + n/2
            assert_le!(
                weight_total,
                ((n_halved + two) / (*DEFAULT_RECONSTRUCT_THRESHOLD - *DEFAULT_SECRECY_THRESHOLD))
                    .ceil()
                    + n_halved
            );

            for subset in 0..num_subsets {
                let stake_sub_total = U64F64::from(get_sub_total(stakes.as_slice(), subset));
                let weight_sub_total = get_sub_total(profile.validator_weights.as_slice(), subset);
                if stake_sub_total <= stake_secrecy_threshold
                    && weight_sub_total >= profile.reconstruct_threshold_in_weights
                {
                    unreachable!();
                }
                if stake_sub_total > stake_reconstruct_threshold
                    && weight_sub_total < profile.reconstruct_threshold_in_weights
                {
                    unreachable!();
                }
                if stake_sub_total <= fast_path_stake_secrecy_threshold
                    && weight_sub_total >= profile.fast_reconstruct_threshold_in_weights.unwrap()
                {
                    unreachable!();
                }
            }
        }
    }
}

fn get_sub_total(vals: &[u64], subset: u64) -> u64 {
    vals.iter()
        .enumerate()
        .map(|(idx, &val)| val * ((subset >> idx) & 1))
        .sum()
}
