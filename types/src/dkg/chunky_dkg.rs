use crate::{
    dkg::real_dkg::rounding::{
        DKGRounding, DEFAULT_RECONSTRUCT_THRESHOLD, DEFAULT_SECRECY_THRESHOLD,
    },
    on_chain_config::{
        ChunkyDKGConfigMoveStruct, OnChainChunkyDKGConfig, OnChainRandomnessConfig,
        RandomnessConfigMoveStruct,
    },
    validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
};
use anyhow::Result;
use aptos_batch_encryption::group::Pairing;
use aptos_crypto::{
    bls12381::{self, PrivateKey, PublicKey},
    weighted_config::WeightedConfigArkworks,
};
use aptos_dkg::pvss::{
    chunky::{
        input_secret::InputSecret, keys, PublicParameters, SignedWeightedTranscript,
        WeightedSubtranscript,
    },
    traits::transcript::{Aggregatable, HasAggregatableSubtranscript, Subtranscript, Transcript},
    Player,
};
use ark_bn254::Fr;
use fixed::types::U64F64;
use rand::{thread_rng, CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::cmp::max;

pub type ChunkyTranscript = SignedWeightedTranscript<Pairing>;
pub type ChunkySubtranscript = WeightedSubtranscript<Pairing>;
pub type DealerPrivateKey = bls12381::PrivateKey;
pub type DealerPublicKey = bls12381::PublicKey;
pub type SecretSharingConfig = WeightedConfigArkworks<Fr>;
pub type EncryptPubKey = keys::EncryptPubKey<Pairing>;

/// Reflection of `0x1::dkg::DKGSessionMetadata` in rust for Chunky DKG.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkyDKGSessionMetadata {
    pub dealer_epoch: u64,
    pub chunky_dkg_config: ChunkyDKGConfigMoveStruct,
    pub dealer_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
    pub target_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
}

impl ChunkyDKGSessionMetadata {
    pub fn target_validator_consensus_infos_cloned(&self) -> Vec<ValidatorConsensusInfo> {
        self.target_validator_set
            .clone()
            .into_iter()
            .map(|obj| obj.try_into().unwrap())
            .collect()
    }

    pub fn dealer_consensus_infos_cloned(&self) -> Vec<ValidatorConsensusInfo> {
        self.dealer_validator_set
            .clone()
            .into_iter()
            .map(|obj| obj.try_into().unwrap())
            .collect()
    }

    pub fn into_on_chain_chunky_dkg_config(&self) -> Option<OnChainChunkyDKGConfig> {
        OnChainRandomnessConfig::try_from(self.chunky_dkg_config.clone()).ok()
    }
}

pub struct ChunkyDKG {
    secret_sharing_config: SecretSharingConfig,
    public_parameters: PublicParameters<Pairing>,
    ssk: DealerPrivateKey,
    spk: DealerPublicKey,
}

impl ChunkyDKG {
    pub fn new(
        secret_sharing_config: SecretSharingConfig,
        public_parameters: PublicParameters<Pairing>,
        ssk: DealerPrivateKey,
        spk: DealerPublicKey,
    ) -> Self {
        Self {
            secret_sharing_config,
            public_parameters,
            ssk,
            spk,
        }
    }

    pub fn deal<A: Serialize + Clone, R: RngCore + CryptoRng>(
        secret_sharing_config: &SecretSharingConfig,
        public_parameters: &PublicParameters<Pairing>,
        ssk: &DealerPrivateKey,
        spk: &DealerPublicKey,
        s: &InputSecret<Fr>,
        sid: &A,
        dealer: &Player,
        eks: &[EncryptPubKey],
        rng: &mut R,
    ) -> ChunkyTranscript {
        ChunkyTranscript::deal(
            secret_sharing_config,
            public_parameters,
            ssk,
            spk,
            eks,
            s,
            sid,
            dealer,
            rng,
        )
    }

    pub fn verify<A: Serialize + Clone>(
        &self,
        transcript: &ChunkyTranscript,
        spks: &[DealerPublicKey],
        eks: &[EncryptPubKey],
        sid: &A,
    ) -> Result<()> {
        transcript.verify(
            &self.secret_sharing_config,
            &self.public_parameters,
            spks,
            eks,
            sid,
        )
    }

    pub fn sub_aggregate(
        &self,
        sub_transcripts: &[ChunkySubtranscript],
    ) -> Result<ChunkySubtranscript> {
        if sub_transcripts.is_empty() {
            anyhow::bail!("Cannot aggregate empty vector of subtranscripts");
        }

        let mut accumulator = sub_transcripts[0].clone();
        for other in sub_transcripts.iter().skip(1) {
            accumulator.aggregate_with(&self.secret_sharing_config, other)?;
        }

        Ok(accumulator)
    }

    /// Generate secret sharing config and public parameters from DKG session metadata.
    /// Similar to `RealDKG::new_public_params` but returns the config components directly.
    pub fn generate_config(
        dkg_session_metadata: &ChunkyDKGSessionMetadata,
    ) -> (
        SecretSharingConfig,
        PublicParameters<Pairing>,
        Vec<EncryptPubKey>,
    ) {
        let randomness_config = dkg_session_metadata
            .chunky_dkg_config_derived()
            .unwrap_or_else(OnChainRandomnessConfig::default_enabled);
        let secrecy_threshold = randomness_config
            .secrecy_threshold()
            .unwrap_or_else(|| *DEFAULT_SECRECY_THRESHOLD);
        let reconstruct_threshold = randomness_config
            .reconstruct_threshold()
            .unwrap_or_else(|| *DEFAULT_RECONSTRUCT_THRESHOLD);
        let maybe_fast_path_secrecy_threshold = randomness_config.fast_path_secrecy_threshold();

        let target_validators = dkg_session_metadata.target_validator_consensus_infos_cloned();
        let validator_stakes: Vec<u64> =
            target_validators.iter().map(|vi| vi.voting_power).collect();

        let validator_consensus_keys: Vec<bls12381::PublicKey> = target_validators
            .iter()
            .map(|vi| vi.public_key.clone())
            .collect();

        let eks: Vec<EncryptPubKey> = validator_consensus_keys
            .iter()
            .map(|k| k.to_bytes().as_slice().try_into().unwrap())
            .collect::<Vec<_>>();

        // Use the same rounding logic as RealDKG to compute weights
        let DKGRounding {
            profile,
            wconfig: _,
            fast_wconfig: _,
            rounding_error: _,
            rounding_method: _,
        } = DKGRounding::new(
            &validator_stakes,
            secrecy_threshold,
            reconstruct_threshold,
            maybe_fast_path_secrecy_threshold,
        );

        // Create WeightedConfigArkworks<Fr> from the computed weights
        let secret_sharing_config = WeightedConfigArkworks::new(
            profile.reconstruct_threshold_in_weights as usize,
            profile
                .validator_weights
                .iter()
                .map(|w| *w as usize)
                .collect(),
        )
        .expect("Failed to create WeightedConfigArkworks");

        // Create PublicParameters<Pairing> with max_num_shares based on total weight
        let total_weight: usize = profile.validator_weights.iter().sum::<u64>() as usize;
        let mut rng = thread_rng();
        let public_parameters = PublicParameters::with_max_num_shares(total_weight);

        (secret_sharing_config, public_parameters, eks)
    }
}
