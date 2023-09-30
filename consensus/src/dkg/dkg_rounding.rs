// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Formatter, Result};

use aptos_dkg::pvss::WeightedConfig;
use aptos_logger::{debug, error, trace};

pub const WEIGHT_PER_VALIDATOR_MIN : usize = 1;
pub const WEIGHT_PER_VALIDATOR_MAX : usize = 30;
pub const STEPS : usize = 1_000;
pub const STAKE_GAP_THRESHOLD : f64 = 0.02; // dkg todo: decide threshold

#[derive(Clone)]
pub struct DKGRoundingProfile {
    pub validator_weights: Vec<usize>,
    pub stake_gap: f64,
    pub threshold_fallback: usize,
    pub threshold_optimistic: usize,
}

impl Debug for DKGRoundingProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "stake_gap: {}, ", self.stake_gap)?;
        write!(f, "total_weight: {}, ", self.validator_weights.iter().sum::<usize>())?;
        write!(f, "threshold_fallback: {}, ", self.threshold_fallback)?;
        write!(f, "threshold_optimistic: {}, ", self.threshold_optimistic)?;
        write!(f, "validator_weights: {:?}\n", self.validator_weights)?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DKGRounding {
    pub profile: DKGRoundingProfile,
    pub config_fallback: WeightedConfig,
    pub config_optimistic: WeightedConfig,
}

impl DKGRounding {
    pub fn new(
        validator_stakes: Vec<u64>,
        stake_gap_threshold: f64,
        weight_per_validator_min: usize,
        weight_per_validator_max: usize,
        steps: usize,
    ) -> Self {
        let profile = DKGRoundingProfile::new(validator_stakes.clone(), stake_gap_threshold, weight_per_validator_min, weight_per_validator_max, steps);

        if profile.stake_gap > stake_gap_threshold {
            // dkg todo: add alert here
            error!("[DKG] Rounding exceeds stake_gap_threshold! stake_gap = {}, stake_gap_threshold = {}", profile.stake_gap, stake_gap_threshold);
        }

        let config_fallback =
            WeightedConfig::new(profile.threshold_fallback, profile.validator_weights.clone()).unwrap();
        let config_optimistic =
            WeightedConfig::new(profile.threshold_optimistic, profile.validator_weights.clone()).unwrap();

        Self {
            profile,
            config_fallback,
            config_optimistic,
        }
    }
}

impl DKGRoundingProfile {
    pub fn new(
        validator_stakes: Vec<u64>,
        stake_gap_threshold: f64,
        weight_per_validator_min: usize,
        weight_per_validator_max: usize,
        steps: usize,
    ) -> Self {
        let validator_num = validator_stakes.len();
        let total_weight_min = weight_per_validator_min * validator_num;
        let total_weight_max = weight_per_validator_max * validator_num;
        let mut best_profile = DKGRoundingProfile {
            validator_weights: vec![],
            stake_gap: 1.0,
            threshold_fallback: 0,
            threshold_optimistic: 0,
        };

        for step in 0..steps {
            let total_weight = total_weight_min + (total_weight_max - total_weight_min) * step / steps;
        
            let profile = compute_profile(validator_stakes.clone(), total_weight);

            assert!(profile.stake_gap < 1.0);

            // This check makes sure the fallback path is live: 2/3 stakes can reconstruct the randomness.
            if profile.stake_gap > 1.0 / 3.0 {
                continue;
            }

            // Make sure each validator has at least 1 weight.
            if profile.validator_weights.iter().any(|w| *w == 0) {
                continue;
            }

            if profile.stake_gap < best_profile.stake_gap {
                best_profile = profile.clone();
            }

            if profile.stake_gap <= stake_gap_threshold {
                debug!(
                    "[DKG] Rounding finished! {:?}\n",
                    profile
                );
                break;
            }
        }
        best_profile
    }
}

pub fn compute_profile(
    validator_stakes: Vec<u64>,
    weights_sum: usize,
) -> DKGRoundingProfile {
    let stake_sum = validator_stakes.iter().sum::<u64>();
    let stake_per_weight = stake_sum / weights_sum as u64;
    let fractions = validator_stakes
        .iter()
        .map(|stake| (*stake as f64 / stake_per_weight as f64) - ((stake / stake_per_weight) as f64))
        .collect::<Vec<f64>>();
    let c = 0.5;
    let mut delta_d = 0.0;
    let mut delta_u = 0.0;
    for j in 0..fractions.len() {
        if fractions[j] + c >= 1.0 {
            delta_u += 1.0 - fractions[j];
        } else {
            delta_d += fractions[j];
        }
    }
    let delta = delta_d + delta_u;

    let validator_weights = validator_stakes
        .iter()
        .map(|stake| (*stake as f64 / stake_per_weight as f64 + c) as usize)
        .collect::<Vec<usize>>();

    let threshold_fallback = ((stake_sum as f64) / (3.0 * stake_per_weight as f64) + delta_u).ceil() as usize;
    let threshold_optimistic = ((2.0 * stake_sum as f64) / (3.0 * stake_per_weight as f64) + delta_u).ceil() as usize; 
    //dkg todo - productionize - double check if float number operations are deterministic across platform

    let stake_gap = stake_per_weight as f64 * delta / stake_sum as f64;

    let profile = DKGRoundingProfile {
        validator_weights,
        stake_gap,
        threshold_fallback,
        threshold_optimistic,
    };

    trace!("[DKG] Rounding in progress! {:?}", profile);

    profile
}

#[test]
fn compute_mainnet_rounding() {
    const MAINNET_STAKES: [u64; 105] = [
        165145270249182, 167472875944241, 106998019789998, 106997817736231,
        106180110197899, 715670034108607, 384987808538022, 106995944603167,
        106996443343406, 106995660962202, 106997667334856, 106996226114454,
        2097152360830777, 2103881557850920, 2103835069798367, 2103827879313634,
        2103816799694689, 2097841475140157, 2097840449641530, 1637810321235780,
        105697215834675, 105742598030792, 105698811528848, 105700051110470,
        105698088171377, 105697166913847, 105699691526728, 105698491267709,
        105698438903025, 105699494646477, 105698231203401, 105789923724341,
        106992186404498, 170691845612231, 106990975022283, 105850796757937,
        105696045824968, 1848599938561894, 2097046518508971, 105698831776364,
        105702350232896, 496873377475271, 105695013750974, 1553672331034765,
        229248988804101, 105698430748858, 105695696366376, 105694860673746,
        105694598499196, 2098130855809659, 2097045160038929, 1552385315374027,
        1682647161200787, 768248526840643, 122101842895227, 990146021892019,
        109182231249042, 105683300381625, 105782803670666, 105853149308755,
        384855202097242, 384855019986354, 384855038233680, 137899492111608,
        2103668659908977, 2097639023899308, 105692568938377, 2104573902627526,
        105693952443274, 105844502700115, 2096960885670504, 105693249229351,
        2097003298668704, 687430084400130, 533253043331656, 2103654954690872,
        1769909958939197, 1641038289198788, 105698265505541, 2098643081809643,
        2103654391648782, 1578988244474554, 1703809423052874, 1648903295001773,
        175938216389803, 2013488582724366, 104736444788232, 188621842683820,
        114000122599956, 1415402623129527, 170308197224643, 1625371326821225,
        121961473780216, 340565070196551, 2008230454671257, 124065143384481,
        2099068273877142, 2100372478327218, 2100374888852838, 2100374910083403,
        2100206976949128, 2100139906897352, 1568974732585563, 700423014587286,
        250143869147619,
    ];

    for stake_gap in (5..=100).step_by(1) {
        let stake_gap = stake_gap as f64 / 1000.0;
        let mainnet_dkg_rounding = DKGRounding::new(MAINNET_STAKES.to_vec(), stake_gap, WEIGHT_PER_VALIDATOR_MIN, WEIGHT_PER_VALIDATOR_MAX, STEPS);
        println!("{:?}", mainnet_dkg_rounding.profile);
    }
}