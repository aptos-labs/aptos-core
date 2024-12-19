// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure, Result};
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
    ensure!(
        recon_threshold_shl64 > secrecy_threshold_shl64,
        "main() failed: recon_thre > secrecy_thre not satisfied!"
    );
    total_weight_max = total_weight_max.div_ceil(
        &((recon_threshold_shl64.clone() - secrecy_threshold_shl64.clone()) * BigUint::from(2u64)),
    );
    let stakes_total: BigUint = stakes.clone().into_iter().sum();
    ensure!(!stakes_total.is_zero(), "main() failed: stakes_total is 0!");
    let bar = (stakes_total.clone() * recon_threshold_shl64.clone()) >> 64;
    let mut lo = 0;
    let mut hi = (total_weight_max * BigUint::from(2_u64))
        .to_u128()
        .ok_or_else(|| anyhow!("main() failed: total_weight_max*2 is not a u128!"))?;
    // This^ ensures the first `ideal_weight` to try is `total_weight_max`,
    // which should always result in a valid weight assignment that satisfies `recon_threshold_shl64`.

    let mut profile = Profile::naive(n);
    while lo + 1 < hi {
        let ideal_weight = lo + (hi - lo) / 2;
        let mut weight_per_stake_shl64 = BigUint::from(ideal_weight);
        weight_per_stake_shl64 <<= 64;
        weight_per_stake_shl64 = weight_per_stake_shl64.div_ceil(&stakes_total);
        let cur_profile = compute_profile(
            secrecy_threshold_shl64.clone(),
            fast_secrecy_thresh_shl64.clone(),
            &stakes,
            BigUint::from(ideal_weight),
            weight_per_stake_shl64,
        )
        .map_err(|e| anyhow!("main() failed with profile err: {e}"))?;
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
        let w = w
            .to_u64()
            .ok_or_else(|| anyhow!("main() failed: some weight is not u64!"))?;
        weights.push(w);
    }
    let reconstruct_threshold_fast_path = if let Some(t) = threshold_fast_path {
        Some(t.in_weights.to_u128().ok_or_else(|| {
            anyhow!("main() failed: recon_thre_fast_path.in_weights is not a u128!")
        })?)
    } else {
        None
    };

    Ok(RoundedV2 {
        ideal_total_weight: ideal_total_weight
            .to_u128()
            .ok_or_else(|| anyhow!("main() failed: ideal_total_weight is not a u128"))?,
        weights,
        reconstruct_threshold_default_path: threshold_default_path
            .in_weights
            .to_u128()
            .ok_or_else(|| {
                anyhow!("main() failed: recon_thre_default_path.in_weights is not a u128!")
            })?,
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
) -> Result<ReconstructThresholdInfo> {
    let mut final_thresh = (((weight_gain_shl64 << 64)
        + stake_total * secrecy_threshold_shl64 * weight_per_stake_shl64.clone())
        >> 128)
        + BigUint::one();
    final_thresh = min(final_thresh, weight_total);
    let mut stakes_required = final_thresh.clone();
    stakes_required <<= 64;
    stakes_required += weight_loss_shl64;
    ensure!(
        !weight_per_stake_shl64.is_zero(),
        "compute_threshold() failed with weight_per_stake=0!"
    );
    stakes_required = stakes_required.div_ceil(&weight_per_stake_shl64);
    Ok(ReconstructThresholdInfo {
        in_weights: final_thresh,
        in_stakes: stakes_required,
    })
}

fn compute_profile(
    secrecy_threshold_shl64: BigUint,
    fast_path_secrecy_threshold_shl64: Option<BigUint>,
    stakes: &[BigUint],
    ideal_total_weight: BigUint,
    weight_per_stake_shl64: BigUint,
) -> Result<Profile> {
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
    )
    .map_err(|e| anyhow!("compute_profile() failed with default threshold err: {e}"))?;
    let threshold_fast_path = if let Some(v) = fast_path_secrecy_threshold_shl64 {
        let t = compute_threshold(
            v,
            weight_per_stake_shl64,
            total_stake,
            total_weight,
            weight_gain_shl64,
            weight_loss_shl64,
        )
        .map_err(|e| anyhow!("compute_profile() failed with fast threshold err: {e}"))?;
        Some(t)
    } else {
        None
    };
    Ok(Profile {
        ideal_total_weight,
        validator_weights,
        threshold_default_path,
        threshold_fast_path,
    })
}

#[test]
fn test_mainnet() {
    let stakes = MAINNET_STAKES.map(BigUint::from).to_vec();
    let secrecy_thresh_shl64 = BigUint::from(1_u64 << 63); // 1/2
    let recon_thresh_shl64 = BigUint::from(66_u128 << 64) / BigUint::from(100_u64); // 66/100
    let fast_secrecy_thresh_shl64 = Some(BigUint::from(67_u128 << 64) / BigUint::from(100_u64));
    let rounded = main(
        stakes,
        secrecy_thresh_shl64,
        recon_thresh_shl64,
        fast_secrecy_thresh_shl64,
    );
    println!("rounded={:?}", rounded);
}

#[test]
fn test_single_validator() {
    let stakes = vec![1_000_000_u64 * 100000000]
        .into_iter()
        .map(BigUint::from)
        .collect();
    let secrecy_thresh_shl64 = BigUint::from(1_u64 << 63); // 1/2
    let recon_thresh_shl64 = BigUint::from(66_u128 << 64) / BigUint::from(100_u64); // 66/100
    let fast_secrecy_thresh_shl64 = Some(BigUint::from(67_u128 << 64) / BigUint::from(100_u64));
    let rounded = main(
        stakes,
        secrecy_thresh_shl64,
        recon_thresh_shl64,
        fast_secrecy_thresh_shl64,
    );
    println!("rounded={:?}", rounded);
}

#[test]
fn test_almost_equal_stakes() {
    let stakes = vec![
        100000000000001_u64,
        100000000000010_u64,
        100000000000100_u64,
        100000000001000_u64,
    ]
    .into_iter()
    .map(BigUint::from)
    .collect();
    let secrecy_thresh_shl64 = BigUint::from(1_u64 << 63); // 1/2
    let recon_thresh_shl64 = BigUint::from(66_u128 << 64) / BigUint::from(100_u64); // 66/100
    let fast_secrecy_thresh_shl64 = Some(BigUint::from(67_u128 << 64) / BigUint::from(100_u64));
    let rounded = main(
        stakes,
        secrecy_thresh_shl64,
        recon_thresh_shl64,
        fast_secrecy_thresh_shl64,
    );
    println!("rounded={:?}", rounded);
}

#[cfg(test)]
const MAINNET_STAKES: [u64; 152] = [
    109085842620913,
    181846169708232,
    116436748125955,
    116430067331922,
    264492524347614,
    1117234735426174,
    1317474743808403,
    780893376087043,
    113754422503733,
    114216365498148,
    113746624056420,
    113755656294904,
    113781952848274,
    113783976062707,
    113755205328003,
    113741613074085,
    113963918602349,
    113782990854844,
    113784943468798,
    116342322109444,
    113843004255938,
    113788537476211,
    881350285519926,
    608909920797466,
    113868631518944,
    113767155717218,
    113779780075063,
    740789396347075,
    113705489737106,
    114055055661859,
    114201560822846,
    796611124841311,
    796908268007787,
    740111566149361,
    802284355409103,
    368278864031829,
    102657284510145,
    113736006330978,
    114239818723942,
    174804196569557,
    174831644972643,
    174809523612237,
    158298197982619,
    1109168779181223,
    113745184543312,
    579409907193940,
    113771256450259,
    796604088661366,
    113752183962838,
    680042418721046,
    752817883778082,
    844189110280186,
    782663436212288,
    113742369789074,
    1001586187521703,
    752655362655174,
    753041794769417,
    812361510819965,
    786195292266166,
    114576362483952,
    1004901523897466,
    587118375166971,
    1151130358354129,
    812247959920627,
    1019896668285008,
    177121771184899,
    1448225866107970,
    185253127119861,
    1003088878597661,
    918239083246732,
    697165115627502,
    692332621322017,
    800430079649388,
    800668349784763,
    799862603620503,
    800535451196497,
    1696666322347755,
    255404578361389,
    676472482614895,
    247900768746466,
    1875474872714470,
    504497673034384,
    253067553127898,
    694666548218942,
    694253061729656,
    694282406501100,
    498426191131203,
    693977043123008,
    3038336720767927,
    1007413364737981,
    1149579651611293,
    1064964254647415,
    1513220802630919,
    1513221020646924,
    396479383974198,
    226817861444564,
    1069619892129257,
    1002639332022909,
    1002639332022909,
    609327620115938,
    672747761018403,
    579890529920216,
    116041358961134,
    321695079236252,
    1056706189505028,
    940312360503218,
    1213747715103075,
    343191042960086,
    1057561132612252,
    285700461676841,
    419142273386077,
    630732044147823,
    1003105222879486,
    663060488813592,
    424390235599690,
    417209226649284,
    505589394660616,
    1006751673769230,
    506292436928968,
    104692703575828,
    313845018170381,
    312459447846793,
    154761534661664,
    1038510051918879,
    101009153432942,
    138924478847793,
    100316943397181,
    1034698650907962,
    1010525137557762,
    206109535363436,
    1002437255001272,
    511526783064789,
    191167871012986,
    230625003449584,
    108033819134242,
    340493300307843,
    203813932428785,
    918993111017769,
    212432200385778,
    1577182739447495,
    202701213893958,
    305403538727752,
    506714700545651,
    604140912065066,
    1777804021776429,
    101000388603664,
    2257894687400754,
    2392552004976224,
    401970761658505,
    137346400526964,
    400686751849318,
    150027911650570,
];
