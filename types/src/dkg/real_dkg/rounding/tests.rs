// Copyright © Aptos Foundation

use crate::dkg::real_dkg::rounding::{
    is_valid_profile, total_weight_lower_bound, total_weight_upper_bound, DKGRounding,
    DEFAULT_RECONSTRUCT_THRESHOLD, DEFAULT_SECRECY_THRESHOLD,
};
use aptos_dkg::pvss::WeightedConfig;
use fixed::types::U64F64;
use rand::Rng;
use std::ops::Deref;

#[test]
fn compute_mainnet_rounding() {
    let validator_stakes = MAINNET_STAKES.to_vec();
    let dkg_rounding = DKGRounding::new(
        &validator_stakes,
        *DEFAULT_SECRECY_THRESHOLD.deref(),
        *DEFAULT_RECONSTRUCT_THRESHOLD.deref(),
    );
    // println!("mainnet rounding profile: {:?}", dkg_rounding.profile);
    // Result:
    // mainnet rounding profile: total_weight: 437, secrecy_threshold_in_stake_ratio: 0.5, reconstruct_threshold_in_stake_ratio: 0.5859020899996102, reconstruct_threshold_in_weights: 237, validator_weights: [10, 1, 9, 9, 1, 1, 9, 9, 1, 7, 8, 5, 2, 1, 9, 7, 1, 2, 1, 9, 2, 1, 1, 9, 1, 8, 10, 1, 1, 9, 1, 1, 1, 7, 9, 1, 1, 9, 1, 9, 1, 3, 1, 8, 1, 1, 7, 10, 3, 2, 1, 9, 1, 9, 1, 3, 8, 1, 10, 1, 1, 1, 9, 3, 8, 8, 3, 10, 1, 1, 7, 9, 2, 5, 2, 9, 9, 1, 4, 1, 1, 1, 1, 1, 2, 10, 1, 1, 9, 1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 2, 1, 9, 8, 1, 1, 9, 2, 1]

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
        );
        let wconfig = WeightedConfig::new(
            (U64F64::from_num(validator_num) * *DEFAULT_SECRECY_THRESHOLD.deref())
                .ceil()
                .to_num::<usize>(),
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
pub const MAINNET_STAKES: [u64; 112] = [
    210500217584363000,
    19015034427309200,
    190269409955015000,
    190372712607660000,
    13695461583653900,
    23008441599765600,
    190710275073260000,
    190710280752007000,
    10610983628971600,
    154224802732739000,
    175900128414965000,
    99375343208846800,
    33975409124588400,
    10741696639154700,
    190296758443194000,
    146931795395201000,
    17136059081003400,
    50029051467899600,
    10610346785890000,
    190293387423510000,
    38649607904320700,
    10599959445206200,
    10741007619737700,
    181012458336443000,
    12476986507395000,
    162711519739867000,
    210473652405885000,
    17652549388174200,
    10602173827686000,
    181016968624497000,
    10741717083802200,
    10601364932429600,
    10626550439528100,
    157588554433899000,
    190368494070257000,
    10602102958015200,
    10659605390935200,
    190296749885358000,
    10602246540607000,
    190691643530347000,
    10741129232477400,
    71848511917757900,
    10741464265442800,
    167168618455916000,
    10626776626668800,
    10899006338732500,
    154355154034690000,
    200386024285735000,
    53519567070710700,
    49607201233899200,
    10601653390317000,
    190575467847849000,
    16797596395552600,
    190366710793058000,
    10602477251277100,
    62443725129072300,
    163816210803988000,
    10610954198660500,
    201023046191587000,
    10601464591446000,
    10609852486777200,
    10601487012558200,
    180360219576606000,
    70316229167094400,
    163090136300726000,
    165716856572893000,
    64007132243756300,
    210458282376492000,
    12244035421744000,
    10601711009001400,
    156908154902803000,
    190688831761348000,
    40078251173380300,
    110184163534171000,
    38221801093982600,
    190373486881563000,
    191035674729349000,
    10602120712089200,
    76636833488874800,
    10602114283230900,
    12257823010913900,
    10741509540453600,
    10602136737656500,
    10602078523390900,
    38222380945714300,
    210500003057396000,
    10789031621748400,
    10741733031173300,
    183655787790140000,
    10610791490932400,
    10602182576946400,
    10741639855953200,
    10602203255280800,
    11938813410693300,
    10741355256561700,
    68993421760499900,
    10610344082022600,
    25112384536164900,
    22886710016497000,
    10602439528909000,
    10602834493124000,
    10602101852821800,
    16812894183934200,
    46140391561066400,
    16579223362042600,
    191035150659780000,
    169268334324248000,
    10600667662818000,
    10625918567828000,
    180685941615229000,
    38221788594331900,
    10516889883063100,
];
