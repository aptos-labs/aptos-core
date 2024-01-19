// Copyright © Aptos Foundation

use crate::{on_chain_config::ValidatorSet, validator_verifier::ValidatorVerifier};
use anyhow::Result;
use aptos_crypto::{bls12381, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::{
    constants::SEED_PVSS_PUBLIC_PARAMS,
    pvss::{self, traits::Transcript, WeightedConfig},
};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

pub type WTrx = pvss::das::WeightedTranscript;
pub type DkgPP = <WTrx as Transcript>::PublicParameters;
pub type SSConfig = <WTrx as Transcript>::SecretSharingConfig;
pub type EncPK = <WTrx as Transcript>::EncryptPubKey;

pub const WEIGHT_PER_VALIDATOR_MIN: usize = 1;
pub const WEIGHT_PER_VALIDATOR_MAX: usize = 30;
pub const STEPS: usize = 1_000;
pub const STAKE_GAP_THRESHOLD: f64 = 0.1;
pub const RECONSTRUCT_THRESHOLD: f64 = 0.5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartDKGEvent {
    pub target_epoch: u64,
    pub start_time_us: u64,
    pub target_validator_set: ValidatorSet,
}

impl StartDKGEvent {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}
impl MoveStructType for StartDKGEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("StartDKGEvent");
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGPvssConfig {
    pub epoch: u64,
    // weighted config for randomness generation
    pub wconfig: SSConfig,
    // DKG public parameters
    pub pp: DkgPP,
    // DKG encryption public keys
    pub eks: Vec<EncPK>,
}

impl DKGPvssConfig {
    pub fn new(epoch: u64, wconfig: SSConfig, pp: DkgPP, eks: Vec<EncPK>) -> Self {
        Self {
            epoch,
            wconfig,
            pp,
            eks,
        }
    }

    pub fn num_bytes(&self) -> usize {
        // dkg todo: compute size
        0
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DKGTranscriptWrapper {
    // DKG weighted transcript for randomness generation
    pub trx: WTrx,
}

impl DKGTranscriptWrapper {
    // #[cfg(any(test, feature = "fuzzing"))]
    // pub fn dummy() -> Self {
    //     Self {
    //         trx: WTrx::dummy(),
    //     }
    // }

    pub fn verify(
        &self,
        dkg_pvss_config: &DKGPvssConfig,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        let dealers = self.verify_dealers(verifier.len())?;

        let all_eks = dkg_pvss_config.eks.clone();

        let addresses = verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();

        let spks = dealers_addresses
            .iter()
            .filter_map(|author| verifier.get_public_key(author))
            .collect::<Vec<_>>();

        let aux = dealers_addresses
            .iter()
            .map(|address| (dkg_pvss_config.epoch, address))
            .collect::<Vec<_>>();

        self.trx.verify(
            &dkg_pvss_config.wconfig,
            &dkg_pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        Ok(())
    }

    pub fn verify_dealers(&self, n: usize) -> anyhow::Result<Vec<usize>> {
        let dealers = self
            .trx
            .get_dealers()
            .iter()
            .map(|player| player.id)
            .collect::<Vec<usize>>();
        if dealers.iter().any(|id| *id >= n) {
            anyhow::bail!("[DKG] transcript dealers out of range!");
        }
        Ok(dealers)
    }

    pub fn aggregate_with(&mut self, dkg_pvss_config: &DKGPvssConfig, other: &Self) {
        self.trx
            .aggregate_with(&dkg_pvss_config.wconfig, &other.trx);
    }

    pub fn num_bytes(&self) -> usize {
        self.trx.to_bytes().len()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGAggNodeMetadata {
    pub epoch: u64,
    pub author: AccountAddress,
}

impl DKGAggNodeMetadata {
    pub fn new(epoch: u64, author: AccountAddress) -> Self {
        Self { epoch, author }
    }

    pub fn author(&self) -> &AccountAddress {
        &self.author
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("[DKG] DKGAggNodeMetadata serialization failed!")
    }

    pub fn num_bytes(&self) -> usize {
        self.to_bytes().len()
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq, Eq)]
pub struct DKGAggNode {
    pub metadata: DKGAggNodeMetadata,
    pub agg_trx: DKGTranscriptWrapper,
}

impl DKGAggNode {
    pub fn new(epoch: u64, author: AccountAddress, agg_trx: DKGTranscriptWrapper) -> Self {
        Self {
            metadata: DKGAggNodeMetadata { epoch, author },
            agg_trx,
        }
    }

    pub fn metadata(&self) -> &DKGAggNodeMetadata {
        &self.metadata
    }

    pub fn author(&self) -> &AccountAddress {
        self.metadata.author()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn agg_trx(&self) -> &DKGTranscriptWrapper {
        &self.agg_trx
    }

    pub fn num_bytes(&self) -> usize {
        self.metadata.num_bytes() + self.agg_trx.num_bytes()
    }

    pub fn verify(
        &self,
        pvss_config: &DKGPvssConfig,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        let dealers = self.agg_trx.verify_dealers(verifier.len())?;
        let addresses = verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();
        // Ensure aggregated transcript has enough stakes
        verifier.check_voting_power(dealers_addresses.iter(), false)?;

        self.agg_trx.verify(pvss_config, verifier)
    }
}

pub fn build_dkg_pvss_config(cur_epoch: u64, next_validator_set: &ValidatorSet) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.consensus_voting_power())
        .collect();

    // // For mainnet-like testing
    // let validator_stakes: Vec<u64> = MAINNET_STAKES.to_vec();
    // assert!(validator_stakes.len() == next_validator_set.active_validators.len());

    let dkg_rounding = DKGRounding::new(
        validator_stakes.clone(),
        STAKE_GAP_THRESHOLD,
        WEIGHT_PER_VALIDATOR_MIN,
        WEIGHT_PER_VALIDATOR_MAX,
        STEPS,
        RECONSTRUCT_THRESHOLD,
    );

    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validator_set
        .active_validators
        .iter()
        .map(|vi| vi.consensus_public_key().clone())
        .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();

    let wconfig = dkg_rounding.wconfig.clone();

    let pp = DkgPP::new_from_seed_with_bls_base(SEED_PVSS_PUBLIC_PARAMS);

    DKGPvssConfig::new(cur_epoch, wconfig.clone(), pp, consensus_keys)
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
        assert!(0 < weight_per_validator_min && weight_per_validator_min <= weight_per_validator_max);
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

#[test]
fn compute_mainnet_rounding() {
    for stake_gap in (5..=100).step_by(1) {
        let stake_gap = stake_gap as f64 / 1000.0;
        let mainnet_dkg_rounding = DKGRounding::new(
            MAINNET_STAKES.to_vec(),
            stake_gap,
            WEIGHT_PER_VALIDATOR_MIN,
            WEIGHT_PER_VALIDATOR_MAX,
            STEPS,
            RECONSTRUCT_THRESHOLD,
        );
        println!("{:?}", mainnet_dkg_rounding.profile);
    }
}

#[test]
fn test_rounding_uniform_distribution() {
    let num_runs = 100;
    let mut rng = rand::thread_rng();
    // assuming each validator has a stake between 1_000_000 and 50_000_000, following uniform distribution
    // randomly generate 100~500 validators' stake distribution
    for _ in 0..num_runs {
        let validator_num = rng.gen_range(100, 250);
        let mut validator_stakes = vec![];
        for _ in 0..validator_num {
            validator_stakes.push(rng.gen_range(1_000_000, 50_000_000));
        }
        let dkg_rounding = DKGRounding::new(
            validator_stakes,
            STAKE_GAP_THRESHOLD,
            WEIGHT_PER_VALIDATOR_MIN,
            WEIGHT_PER_VALIDATOR_MAX,
            STEPS,
            RECONSTRUCT_THRESHOLD,
        );
        // println!("{:?}", dkg_rounding.profile);
        assert!(dkg_rounding.profile.stake_gap <= STAKE_GAP_THRESHOLD);
        assert!(dkg_rounding.profile.stake_gap + RECONSTRUCT_THRESHOLD <= 2.0 / 3.0);
    }
}

#[cfg(test)]
fn generate_approximate_zipf(size: usize, a: u64, b: u64, exponent: f64) -> Vec<u64> {
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
        let validator_num = rng.gen_range(100, 250);
        let validator_stakes = generate_approximate_zipf(validator_num, 1_000_000, 50_000_000, 5.0);
        let dkg_rounding = DKGRounding::new(
            validator_stakes,
            STAKE_GAP_THRESHOLD,
            WEIGHT_PER_VALIDATOR_MIN,
            WEIGHT_PER_VALIDATOR_MAX,
            STEPS,
            RECONSTRUCT_THRESHOLD,
        );
        // println!("{:?}", dkg_rounding.profile);
        assert!(dkg_rounding.profile.stake_gap <= STAKE_GAP_THRESHOLD);
        assert!(dkg_rounding.profile.stake_gap + RECONSTRUCT_THRESHOLD <= 2.0 / 3.0);
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