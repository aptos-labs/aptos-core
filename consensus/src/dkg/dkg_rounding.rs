// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, Formatter, Result};

use aptos_dkg::pvss::WeightedConfig;
use aptos_logger::{debug, error, trace};

pub const WEIGHT_PER_VALIDATOR_MIN : usize = 1;
pub const WEIGHT_PER_VALIDATOR_MAX : usize = 30;
pub const STEPS : usize = 1_000;
pub const HARDCODED_BEST_ROUNDING_THRESHOLD : f64 = 0.5;
pub const STAKE_GAP_THRESHOLD : f64 = 0.1; // dkg todo: decide threshold
pub const RECONSTRUCT_THRESHOLD : f64 = 1.0 / 3.0;  // dkg todo: decide threshold, theoretically > 1/3 is sufficient


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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "stake_gap: {}, ", self.stake_gap)?;
        write!(f, "total_weight: {}, ", self.validator_weights.iter().sum::<usize>())?;
        write!(f, "reconstruct_threshold_in_weights: {}, ", self.reconstruct_threshold_in_weights)?;
        write!(f, "validator_weights: {:?}\n", self.validator_weights)?;

        Ok(())
    }
}

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
        let profile = DKGRoundingProfile::new(validator_stakes.clone(), stake_gap_threshold, weight_per_validator_min, weight_per_validator_max, steps, reconstruct_threshold);

        if profile.stake_gap > stake_gap_threshold {
            // dkg todo: add alert here
            error!("[DKG] Rounding exceeds stake_gap_threshold! stake_gap = {}, stake_gap_threshold = {}", profile.stake_gap, stake_gap_threshold);
        }

        let wconfig =
            WeightedConfig::new(profile.reconstruct_threshold_in_weights, profile.validator_weights.clone()).unwrap();

        Self {
            profile,
            wconfig,
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
        reconstruct_threshold_in_stake_ratio: f64,
    ) -> Self {
        let validator_num = validator_stakes.len();
        let total_weight_min = weight_per_validator_min * validator_num;
        let total_weight_max = weight_per_validator_max * validator_num;
        let mut best_profile = DKGRoundingProfile {
            validator_weights: vec![],
            stake_gap: 1.0,
            reconstruct_threshold_in_weights: 0,
        };

        for step in 0..steps {
            let total_weight = total_weight_min + (total_weight_max - total_weight_min) * step / steps;
        
            let profile = compute_profile(validator_stakes.clone(), total_weight, reconstruct_threshold_in_stake_ratio);

            assert!(profile.stake_gap < 1.0);

            // This check makes sure the randomness is live: 2/3 stakes can reconstruct the randomness.
            if reconstruct_threshold_in_stake_ratio + profile.stake_gap > 2.0 / 3.0 {
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
    reconstruct_threshold_in_stake_ratio: f64,
) -> DKGRoundingProfile {
    let stake_sum = validator_stakes.iter().sum::<u64>();
    let stake_per_weight = stake_sum / weights_sum as u64;
    let fractions = validator_stakes
        .iter()
        .map(|stake| (*stake as f64 / stake_per_weight as f64) - ((stake / stake_per_weight) as f64))
        .collect::<Vec<f64>>();
    let mut delta_down = 0.0;
    let mut delta_up = 0.0;
    for j in 0..fractions.len() {
        if fractions[j] + HARDCODED_BEST_ROUNDING_THRESHOLD >= 1.0 {
            delta_up += 1.0 - fractions[j];
        } else {
            delta_down += fractions[j];
        }
    }
    let delta_total = delta_down + delta_up;

    let validator_weights = validator_stakes
        .iter()
        .map(|stake| (*stake as f64 / stake_per_weight as f64 + HARDCODED_BEST_ROUNDING_THRESHOLD) as usize)
        .collect::<Vec<usize>>();

    let reconstruct_threshold_in_weights = ((stake_sum as f64) / (stake_per_weight as f64) * reconstruct_threshold_in_stake_ratio + delta_up).ceil() as usize;
    //dkg todo - productionize - double check if float number operations are deterministic across platform

    let stake_gap = stake_per_weight as f64 * delta_total / stake_sum as f64;

    let profile = DKGRoundingProfile {
        validator_weights,
        stake_gap,
        reconstruct_threshold_in_weights,
    };

    trace!("[DKG] Rounding in progress! {:?}", profile);

    profile
}

pub const MAINNET_STAKES: [u64; 112] = [
        210500217584363000, 19015034427309200, 190269409955015000, 190372712607660000,
        13695461583653900, 23008441599765600, 190710275073260000, 190710280752007000,
        10610983628971600, 154224802732739000, 175900128414965000, 99375343208846800,
        33975409124588400, 10741696639154700, 190296758443194000, 146931795395201000,
        17136059081003400, 50029051467899600, 10610346785890000, 190293387423510000,
        38649607904320700, 10599959445206200, 10741007619737700, 181012458336443000,
        12476986507395000, 162711519739867000, 210473652405885000, 17652549388174200,
        10602173827686000, 181016968624497000, 10741717083802200, 10601364932429600,
        10626550439528100, 157588554433899000, 190368494070257000, 10602102958015200,
        10659605390935200, 190296749885358000, 10602246540607000, 190691643530347000,
        10741129232477400, 71848511917757900, 10741464265442800, 167168618455916000,
        10626776626668800, 10899006338732500, 154355154034690000, 200386024285735000,
        53519567070710700, 49607201233899200, 10601653390317000, 190575467847849000,
        16797596395552600, 190366710793058000, 10602477251277100, 62443725129072300,
        163816210803988000, 10610954198660500, 201023046191587000, 10601464591446000,
        10609852486777200, 10601487012558200, 180360219576606000, 70316229167094400,
        163090136300726000, 165716856572893000, 64007132243756300, 210458282376492000,
        12244035421744000, 10601711009001400, 156908154902803000, 190688831761348000,
        40078251173380300, 110184163534171000, 38221801093982600, 190373486881563000,
        191035674729349000, 10602120712089200, 76636833488874800, 10602114283230900,
        12257823010913900, 10741509540453600, 10602136737656500, 10602078523390900,
        38222380945714300, 210500003057396000, 10789031621748400, 10741733031173300,
        183655787790140000, 10610791490932400, 10602182576946400, 10741639855953200,
        10602203255280800, 11938813410693300, 10741355256561700, 68993421760499900,
        10610344082022600, 25112384536164900, 22886710016497000, 10602439528909000,
        10602834493124000, 10602101852821800, 16812894183934200, 46140391561066400,
        16579223362042600, 191035150659780000, 169268334324248000, 10600667662818000,
        10625918567828000, 180685941615229000, 38221788594331900, 10516889883063100,
    ];

#[test]
fn compute_mainnet_rounding() {
    for stake_gap in (5..=100).step_by(1) {
        let stake_gap = stake_gap as f64 / 1000.0;
        let mainnet_dkg_rounding = DKGRounding::new(MAINNET_STAKES.to_vec(), stake_gap, WEIGHT_PER_VALIDATOR_MIN, WEIGHT_PER_VALIDATOR_MAX, STEPS, RECONSTRUCT_THRESHOLD);
        println!("{:?}", mainnet_dkg_rounding.profile);
    }
}