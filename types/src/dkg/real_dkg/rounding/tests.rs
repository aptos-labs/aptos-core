// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::dkg::real_dkg::rounding::{
    is_valid_profile, total_weight_lower_bound, total_weight_upper_bound, DKGRounding,
    DKGRoundingProfile, DEFAULT_FAST_PATH_SECRECY_THRESHOLD, DEFAULT_RECONSTRUCT_THRESHOLD,
    DEFAULT_SECRECY_THRESHOLD,
};
use aptos_dkg::pvss::WeightedConfigBlstrs;
use claims::assert_le;
use fixed::types::U64F64;
use rand::{thread_rng, Rng};
use std::ops::Deref;

/// Thresholds from disable-randomness-fast-path.yaml (Randomness V1: 50% secrecy, 66% reconstruct, no fast path).
fn v1_secrecy_threshold() -> U64F64 {
    U64F64::from_num(50) / U64F64::from_num(100)
}
fn v1_reconstruct_threshold() -> U64F64 {
    U64F64::from_num(66) / U64F64::from_num(100)
}

/// Mainnet validator voting_power (stakes) for V1 rounding test. Snapshot from 0x1::stake::ValidatorSet.
pub const MAINNET_STAKES_V1: [u64; 121] = [
    117253809505204,
    195311866678970,
    1198958522279618,
    1417937347780238,
    304885226418817,
    160458823109233,
    344150625420941,
    289124533763189,
    854901763421965,
    855092844003006,
    289000249712870,
    313251640408293,
    389588690070091,
    169858456899262,
    854902740174904,
    729456853067396,
    807830358661518,
    329617367307420,
    305612981099201,
    1079067863738487,
    807743430639267,
    294028544498534,
    317189556810426,
    306973480211283,
    240890617583765,
    771193504929957,
    870007131164552,
    996027897906475,
    147128803783366,
    1953623404996116,
    203225102560744,
    504565600012818,
    748077261632889,
    859636401621571,
    858396820532061,
    860237274998313,
    232206983586036,
    341867665067879,
    265427737821568,
    1686248681380487,
    538195635528108,
    746412986696168,
    745093859521176,
    745050330999762,
    499933950932256,
    744764824769293,
    1354425472718789,
    1021779338285953,
    1355900621276593,
    1008086402510429,
    1008098624215723,
    415900770352310,
    2490467052077019,
    588583839839258,
    462925719394754,
    1053864655586196,
    759616062654468,
    1001726781035455,
    281887492699783,
    230043140814239,
    512989457679044,
    336068926165701,
    238248412447988,
    1001706398173424,
    108566463736605,
    149322795089050,
    1023882904631239,
    1032612619400875,
    220645844144651,
    548218658355603,
    190131518743791,
    722276451658931,
    1512147511227114,
    990084325967461,
    247035470710922,
    1702673175920816,
    627010527656988,
    1901325221493471,
    1917760514108519,
    2560912546397659,
    416307084013120,
    423728360308727,
    125228177498266,
    137070367008938,
    211672837612296,
    1451601627711626,
    211011786952676,
    581098417474447,
    583428857094457,
    585307317954427,
    1040594791721888,
    477281233543734,
    476045984371681,
    476448688884136,
    632465589653879,
    920573630730192,
    324347863584494,
    171770767028156,
    981797655993635,
    1025829007802591,
    478799011547550,
    1306612085384003,
    1194181730632187,
    261781227456041,
    741740241970041,
    314384239373518,
    214503877844164,
    1440467108788058,
    691645012020740,
    2027612908282052,
    170280310750015,
    307572084275087,
    531117975217181,
    514558734080310,
    204930070719167,
    132038648471431,
    405072364745060,
    189476760921413,
    1512147872347019,
    165544732408544,
    932995244933406,
];

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

/// DKG rounding with Randomness V1 config (disable-randomness-fast-path: 50% secrecy, 66% reconstruct, no fast path).
/// Uses hardcoded mainnet validator stakes (MAINNET_STAKES_V1).
/// Run with: cargo test -p aptos-types compute_mainnet_rounding_v1_no_fast_path -- --nocapture
#[test]
fn compute_mainnet_rounding_v1_no_fast_path() {
    let validator_stakes = MAINNET_STAKES_V1.to_vec();
    println!("validators: {} (MAINNET_STAKES_V1)", validator_stakes.len());

    let dkg_rounding = DKGRounding::new(
        &validator_stakes,
        v1_secrecy_threshold(),
        v1_reconstruct_threshold(),
        None, // no fast path in V1
    );
    println!("mainnet rounding profile (V1, no fast path): {:?}", dkg_rounding.profile);

    let total_weight_min = total_weight_lower_bound(&validator_stakes);
    let total_weight_max = total_weight_upper_bound(
        &validator_stakes,
        v1_reconstruct_threshold(),
        v1_secrecy_threshold(),
    );
    let total_weight = dkg_rounding.profile.validator_weights.iter().sum::<u64>();
    assert!(total_weight >= total_weight_min as u64);
    assert!(total_weight <= total_weight_max as u64);

    assert!(is_valid_profile(&dkg_rounding.profile, v1_reconstruct_threshold()));
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
    let wconfig = WeightedConfigBlstrs::new(1, vec![1]).unwrap();
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
        let wconfig = WeightedConfigBlstrs::new(
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

#[cfg(test)]
pub const MAINNET_STAKES: [u64; 129] = [
    145363920367444000,
    100779935896493000,
    134154255721034000,
    134234783226671000,
    103686549105772000,
    23681356495150300,
    134577645972875000,
    134580712197205000,
    10857449995312600,
    105977831715137000,
    120872333189108000,
    102057954967375000,
    25968006480319300,
    150210261808047000,
    11047428320304400,
    134184685206018000,
    128117795425337000,
    12497398680912800,
    50533200268704000,
    10856898438192700,
    134176595090811000,
    60011869592362800,
    39694335719301200,
    12863458468719700,
    11046541966772300,
    94427102742955200,
    58714132394437700,
    40145680094791300,
    100137028609146000,
    111809600151787000,
    114998912365121000,
    17951622559336400,
    10857216258421800,
    94429160256130900,
    11047450111666100,
    10856891072965200,
    10857020952663600,
    162161975531451000,
    104073248041097000,
    10857355385008300,
    10901683472171900,
    50198365896562800,
    134182311143049000,
    10857000567453500,
    134562155405120000,
    11046814332439800,
    60322390260321900,
    101055534219498000,
    114872371828424000,
    10929001624435000,
    106067388838015000,
    100152796096971000,
    54964069016926100,
    34040647636753800,
    102049697282067000,
    10856874306102900,
    134514110645862000,
    70499650588096400,
    134224065841861000,
    10857370648417500,
    28409091651485400,
    64302033587393400,
    16659350234613100,
    112568696851409000,
    10857782940276500,
    200882335168720000,
    10856846459964200,
    10856305691600600,
    10856576121655500,
    123961368695808000,
    20275491671732600,
    112069796227716000,
    148078356637657000,
    76893226659146600,
    135298123702389000,
    10856788596777500,
    107821720522194000,
    134626203055928000,
    106466193065101000,
    102103040930732000,
    62682920098289700,
    26223235705449200,
    134234424849999000,
    150210282994581000,
    134913703983987000,
    10857227273097400,
    57413947132891200,
    10900450777364100,
    12022049676664200,
    11047053887431400,
    10857590490261700,
    10857257627847700,
    26223774458854400,
    145363467800313000,
    49332020110088600,
    11047476999099800,
    126201751573407000,
    150458532010203000,
    10856470759531000,
    10857203409232300,
    11047327948099800,
    10856521489540500,
    99511242999587700,
    74202213386306600,
    11047051193450900,
    32601393370365500,
    70855459554627300,
    10857401127909800,
    25271130862928600,
    18684565615586300,
    10876016328079300,
    95866473180260000,
    10857461985291600,
    10857176446687100,
    17291414467856800,
    47528452645556600,
    17051109168257700,
    134912746458206000,
    150461059796590000,
    116315225160302000,
    10855453497812600,
    100865713263752000,
    10928177475521100,
    124293961561247000,
    26223786196810300,
    39221522777191400,
    73810826128117800,
    53685896908423700,
    40216803848486900,
];

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
