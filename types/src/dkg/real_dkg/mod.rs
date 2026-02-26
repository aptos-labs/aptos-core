// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    dkg::{
        randomness_dkg::{DKGSessionMetadata, DKGTrait, MayHaveRoundingSummary, RoundingSummary},
        real_dkg::rounding::DKGRounding,
    },
    on_chain_config::OnChainRandomnessConfig,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use anyhow::{anyhow, bail, ensure, Context};
#[cfg(any(test, feature = "testing"))]
use aptos_crypto::Uniform;
use aptos_crypto::{arkworks::shamir::Reconstructable, bls12381, bls12381::PrivateKey};
use aptos_dkg::{
    pvss,
    pvss::{
        traits::{
            transcript::{Aggregatable, AggregatableTranscript, Aggregated, TranscriptCore},
            Convert, Transcript,
        },
        Player,
    },
};
use fixed::types::U64F64;
use move_core_types::account_address::AccountAddress;
use num_traits::Zero;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
    time::Instant,
};

pub mod rounding;

pub type WTrx = pvss::das::WeightedTranscript;
pub type DkgPP = <WTrx as TranscriptCore>::PublicParameters;
pub type SSConfig = <WTrx as TranscriptCore>::SecretSharingConfig;
pub type EncPK = <WTrx as TranscriptCore>::EncryptPubKey;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DKGPvssConfig {
    pub epoch: u64,
    // weighted config for randomness generation
    pub wconfig: SSConfig,
    // DKG public parameters
    pub pp: DkgPP,
    // DKG encryption public keys
    pub eks: Vec<EncPK>,
    // Some metrics for caller to consume.
    #[serde(skip)]
    pub rounding_summary: RoundingSummary,
}

impl PartialEq for DKGPvssConfig {
    fn eq(&self, other: &Self) -> bool {
        (self.epoch, &self.wconfig, &self.pp, &self.eks)
            == (other.epoch, &other.wconfig, &other.pp, &other.eks)
    }
}

impl DKGPvssConfig {
    pub fn new(
        epoch: u64,
        wconfig: SSConfig,
        pp: DkgPP,
        eks: Vec<EncPK>,
        rounding_summary: RoundingSummary,
    ) -> Self {
        Self {
            epoch,
            wconfig,
            pp,
            eks,
            rounding_summary,
        }
    }
}

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    secrecy_threshold: U64F64,
    reconstruct_threshold: U64F64,
    next_validators: &[ValidatorConsensusInfo],
) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validators.iter().map(|vi| vi.voting_power).collect();
    let timer = Instant::now();
    let DKGRounding {
        profile,
        wconfig,
        rounding_error,
        rounding_method,
    } = DKGRounding::new(
        &validator_stakes,
        secrecy_threshold,
        reconstruct_threshold,
    );
    let rounding_time = timer.elapsed();
    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validators
        .iter()
        .map(|vi| vi.public_key.clone())
        .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();

    let pp = DkgPP::default_with_bls_base();

    let rounding_summary = RoundingSummary {
        method: rounding_method,
        output: profile,
        exec_time: rounding_time,
        error: rounding_error,
    };

    DKGPvssConfig::new(cur_epoch, wconfig, pp, consensus_keys, rounding_summary)
}

#[derive(Debug)]
pub struct RealDKG {}

#[derive(Clone, Debug)]
pub struct RealDKGPublicParams {
    pub session_metadata: DKGSessionMetadata,
    pub pvss_config: DKGPvssConfig,
    pub verifier: Arc<ValidatorVerifier>,
}

impl MayHaveRoundingSummary for RealDKGPublicParams {
    fn rounding_summary(&self) -> Option<&RoundingSummary> {
        Some(&self.pvss_config.rounding_summary)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Transcripts {
    // transcript for main path
    pub main: WTrx,
    // transcript for fast path (kept for BCS serialization compatibility)
    pub fast: Option<WTrx>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DealtPubKeyShares {
    // dealt public key share for main path
    pub main: <WTrx as TranscriptCore>::DealtPubKeyShare,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DealtSecretKeyShares {
    // dealt secret key share for main path
    pub main: <WTrx as TranscriptCore>::DealtSecretKeyShare,
}

impl DKGTrait for RealDKG {
    type DealerPrivateKey = <WTrx as Transcript>::SigningSecretKey;
    type DealerPublicKey = <WTrx as Transcript>::SigningPubKey;
    type DealtPubKeyShare = DealtPubKeyShares;
    type DealtSecret = <WTrx as TranscriptCore>::DealtSecretKey;
    type DealtSecretShare = DealtSecretKeyShares;
    type InputSecret = <WTrx as Transcript>::InputSecret;
    type NewValidatorDecryptKey = <WTrx as TranscriptCore>::DecryptPrivKey;
    type PublicParams = RealDKGPublicParams;
    type Transcript = Transcripts;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> RealDKGPublicParams {
        let randomness_config = dkg_session_metadata
            .randomness_config_derived()
            .unwrap_or_else(OnChainRandomnessConfig::default_enabled);
        let secrecy_threshold = randomness_config
            .secrecy_threshold()
            .unwrap_or_else(|| *rounding::DEFAULT_SECRECY_THRESHOLD);
        let reconstruct_threshold = randomness_config
            .reconstruct_threshold()
            .unwrap_or_else(|| *rounding::DEFAULT_RECONSTRUCT_THRESHOLD);

        let pvss_config = build_dkg_pvss_config(
            dkg_session_metadata.dealer_epoch,
            secrecy_threshold,
            reconstruct_threshold,
            &dkg_session_metadata.target_validator_consensus_infos_cloned(),
        );
        let verifier = ValidatorVerifier::new(dkg_session_metadata.dealer_consensus_infos_cloned());
        RealDKGPublicParams {
            session_metadata: dkg_session_metadata.clone(),
            pvss_config,
            verifier: verifier.into(),
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
        pk: &Self::DealerPublicKey,
    ) -> Self::Transcript {
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        let wtrx = WTrx::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            pk,
            &pub_params.pvss_config.eks,
            input_secret,
            &aux,
            &Player { id: my_index },
            rng,
        );
        Transcripts {
            main: wtrx,
            fast: None,
        }
    }

    /// Perform extra necessary checks missing in `verify_transcript`.
    ///
    /// Additionally:
    /// - (needed in block proposal validation) if `check_voting_power`,
    ///   also check if the dealer set specified in the transcript has enough voting power;
    /// - (needed in peer transcript verification) if `ensures_single_dealer` is set,
    ///   also check if the dealer set specified in the transcript only contains the peer.
    fn verify_transcript_extra(
        trx: &Self::Transcript,
        verifier: &ValidatorVerifier,
        checks_voting_power: bool,
        ensures_single_dealer: Option<AccountAddress>,
    ) -> anyhow::Result<()> {
        let all_validator_addrs = verifier.get_ordered_account_addresses();
        let main_trx_dealers = trx.main.get_dealers();
        let mut dealer_set = HashSet::with_capacity(main_trx_dealers.len());
        for dealer in main_trx_dealers.iter() {
            if let Some(dealer_addr) = all_validator_addrs.get(dealer.id) {
                dealer_set.insert(*dealer_addr);
            } else {
                bail!("invalid dealer idx");
            }
        }
        ensure!(main_trx_dealers.len() == dealer_set.len());
        if ensures_single_dealer.is_some() {
            let expected_dealer_set: HashSet<AccountAddress> =
                ensures_single_dealer.into_iter().collect();
            ensure!(expected_dealer_set == dealer_set);
        }

        if checks_voting_power {
            verifier
                .check_voting_power(dealer_set.iter(), true)
                .context("not enough power")?;
        }

        Ok(())
    }

    /// NOTE: this is used in VM.
    fn verify_transcript(
        params: &Self::PublicParams,
        trx: &Self::Transcript,
    ) -> anyhow::Result<()> {
        // Verify dealer indices are valid.
        let dealers = trx
            .main
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

        trx.main.verify(
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
        let mut agg = accumulator.main.to_aggregated();
        agg.aggregate_with(&params.pvss_config.wconfig, &element.main)
            .expect("Transcript aggregation failed");
        accumulator.main = agg.normalize(); // TODO: this should be updated
        accumulator.fast = None;
    }

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)> {
        let (sk, pk) = trx.main.decrypt_own_share(
            &pub_params.pvss_config.wconfig,
            &Player {
                id: player_idx as usize,
            },
            dk,
            &pub_params.pvss_config.pp,
        );
        Ok((
            DealtSecretKeyShares { main: sk },
            DealtPubKeyShares { main: pk },
        ))
    }

    // Test-only function
    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        input_player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> anyhow::Result<Self::DealtSecret> {
        let player_share_pairs: Vec<_> = input_player_share_pairs
            .into_iter()
            .map(|(x, y)| (Player { id: x as usize }, y.main))
            .collect();
        let reconstructed_secret = <WTrx as TranscriptCore>::DealtSecretKey::reconstruct(
            &pub_params.pvss_config.wconfig,
            &player_share_pairs,
        )
        .unwrap();
        Ok(reconstructed_secret)
    }

    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64> {
        transcript
            .main
            .get_dealers()
            .into_iter()
            .map(|x| x.id as u64)
            .collect()
    }
}

impl RealDKG {
    #[cfg(any(test, feature = "testing"))]
    pub fn sample_secret_and_generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &<RealDKG as DKGTrait>::PublicParams,
        my_index: u64,
        sk: &<RealDKG as DKGTrait>::DealerPrivateKey,
        pk: &<RealDKG as DKGTrait>::DealerPublicKey,
    ) -> <RealDKG as DKGTrait>::Transcript {
        let secret = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        Self::generate_transcript(rng, pub_params, &secret, my_index, sk, pk)
    }

    /// The same dealer deals twice and aggregates the transcripts.
    #[cfg(any(test, feature = "testing"))]
    pub fn deal_twice_and_aggregate<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &<RealDKG as DKGTrait>::PublicParams,
        my_index: u64,
        sk: &<RealDKG as DKGTrait>::DealerPrivateKey,
        pk: &<RealDKG as DKGTrait>::DealerPublicKey,
    ) -> <RealDKG as DKGTrait>::Transcript {
        let secret_0 = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        let mut trx_0 = Self::generate_transcript(rng, pub_params, &secret_0, my_index, sk, pk);
        let secret_1 = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        let trx_1 = Self::generate_transcript(rng, pub_params, &secret_1, my_index, sk, pk);
        Self::aggregate_transcripts(pub_params, &mut trx_0, trx_1);
        assert_eq!(2, trx_0.main.get_dealers().len());
        trx_0
    }

}
pub fn maybe_dk_from_bls_sk(
    sk: &PrivateKey,
) -> anyhow::Result<<WTrx as TranscriptCore>::DecryptPrivKey> {
    let mut bytes = sk.to_bytes(); // in big-endian
    bytes.reverse();
    <WTrx as TranscriptCore>::DecryptPrivKey::try_from(bytes.as_slice())
        .map_err(|e| anyhow!("dk_from_bls_sk failed with dk deserialization error: {e}"))
}
