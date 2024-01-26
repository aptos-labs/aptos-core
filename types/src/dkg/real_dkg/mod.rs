// Copyright © Aptos Foundation

use crate::{
    dkg::{real_dkg::rounding::DKGRounding, DKGSessionMetadata, DKGTrait},
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use anyhow::ensure;
use aptos_crypto::bls12381;
use aptos_dkg::{
    pvss,
    pvss::{
        traits::{Convert, Reconstructable, Transcript},
        Player,
    },
};
use num_traits::Zero;
use rand::{CryptoRng, RngCore};
use rounding::{
    RECONSTRUCT_THRESHOLD, STAKE_GAP_THRESHOLD, STEPS, WEIGHT_PER_VALIDATOR_MAX,
    WEIGHT_PER_VALIDATOR_MIN,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod rounding;

pub type WTrx = pvss::das::WeightedTranscript;
pub type DkgPP = <WTrx as Transcript>::PublicParameters;
pub type SSConfig = <WTrx as Transcript>::SecretSharingConfig;
pub type EncPK = <WTrx as Transcript>::EncryptPubKey;

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
}

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    next_validators: &[ValidatorConsensusInfo],
) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validators.iter().map(|vi| vi.voting_power).collect();

    // For mainnet-like testing
    let validator_stakes: Vec<u64> = MAINNET_STAKES.to_vec();
    assert!(validator_stakes.len() == next_validators.len());

    let dkg_rounding = DKGRounding::new(
        validator_stakes.clone(),
        STAKE_GAP_THRESHOLD,
        WEIGHT_PER_VALIDATOR_MIN,
        WEIGHT_PER_VALIDATOR_MAX,
        STEPS,
        RECONSTRUCT_THRESHOLD,
    );

    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validators
        .iter()
        .map(|vi| vi.public_key.clone())
        .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();

    let wconfig = dkg_rounding.wconfig.clone();

    let pp = DkgPP::default_with_bls_base();

    DKGPvssConfig::new(cur_epoch, wconfig.clone(), pp, consensus_keys)
}

#[derive(Debug)]
pub struct RealDKG {}

#[derive(Clone, Debug)]
pub struct RealDKGPublicParams {
    pub session_metadata: DKGSessionMetadata,
    pub pvss_config: DKGPvssConfig,
    pub verifier: ValidatorVerifier,
}

impl DKGTrait for RealDKG {
    type DealerPrivateKey = <WTrx as Transcript>::SigningSecretKey;
    type DealtPubKeyShare = <WTrx as Transcript>::DealtPubKeyShare;
    type DealtSecret = <WTrx as Transcript>::DealtSecretKey;
    type DealtSecretShare = <WTrx as Transcript>::DealtSecretKeyShare;
    type InputSecret = <WTrx as Transcript>::InputSecret;
    type NewValidatorDecryptKey = <WTrx as Transcript>::DecryptPrivKey;
    type PublicParams = RealDKGPublicParams;
    type Transcript = WTrx;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> RealDKGPublicParams {
        let pvss_config = build_dkg_pvss_config(
            dkg_session_metadata.dealer_epoch,
            &dkg_session_metadata.target_validator_consensus_infos_cloned(),
        );
        let verifier = ValidatorVerifier::new(dkg_session_metadata.dealer_consensus_infos_cloned());
        RealDKGPublicParams {
            session_metadata: dkg_session_metadata.clone(),
            pvss_config,
            verifier,
        }
    }

    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret {
        secrets
            .into_iter()
            .fold(<WTrx as Transcript>::InputSecret::zero(), |acc, item| {
                acc + item
            })
    }

    fn dealt_secret_from_input(
        pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret {
        input.to(&pub_params.pvss_config.pp)
    }

    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript {
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        WTrx::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            &pub_params.pvss_config.eks,
            input_secret,
            &aux,
            &Player { id: my_index },
            rng,
        )
    }

    fn verify_transcript(
        params: &Self::PublicParams,
        trx: &Self::Transcript,
    ) -> anyhow::Result<()> {
        // Verify dealer indices are valid.
        let dealers = trx
            .get_dealers()
            .iter()
            .map(|player| player.id)
            .collect::<Vec<usize>>();
        let num_validators = params.session_metadata.dealer_validator_set.len();
        ensure!(
            dealers.iter().all(|id| *id < num_validators),
            "real_dkg::verify_transcript failed with invalid dealer index."
        );

        let all_eks = params.pvss_config.eks.clone();

        let addresses = params.verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();

        let spks = dealers_addresses
            .iter()
            .filter_map(|author| params.verifier.get_public_key(author))
            .collect::<Vec<_>>();

        let aux = dealers_addresses
            .iter()
            .map(|address| (params.pvss_config.epoch, address))
            .collect::<Vec<_>>();

        trx.verify(
            &params.pvss_config.wconfig,
            &params.pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        Ok(())
    }

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    ) {
        accumulator.aggregate_with(&params.pvss_config.wconfig, &element);
    }

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)> {
        let (sk, pk) = trx.decrypt_own_share(
            &pub_params.pvss_config.wconfig,
            &Player {
                id: player_idx as usize,
            },
            dk,
        );
        Ok((sk, pk))
    }

    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> anyhow::Result<Self::DealtSecret> {
        let player_share_pairs = player_share_pairs
            .into_iter()
            .map(|(x, y)| (Player { id: x as usize }, y))
            .collect();
        let reconstructed_secret = <WTrx as Transcript>::DealtSecretKey::reconstruct(
            &pub_params.pvss_config.wconfig,
            &player_share_pairs,
        );
        Ok(reconstructed_secret)
    }

    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64> {
        transcript
            .get_dealers()
            .into_iter()
            .map(|x| x.id as u64)
            .collect()
    }
}

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
