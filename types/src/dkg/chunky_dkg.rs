// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aggregate_signature::AggregateSignature,
    dkg::{
        real_dkg::rounding::{
            DKGRounding, DEFAULT_RECONSTRUCT_THRESHOLD, DEFAULT_SECRECY_THRESHOLD,
        },
        DKGTranscriptMetadata,
    },
    on_chain_config::{ChunkyDKGConfigMoveStruct, OnChainChunkyDKGConfig, OnChainConfig},
    validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
};
use anyhow::Result;
use aptos_batch_encryption::{
    group::{Fr, G2Affine, Pairing},
    shared::{digest::DigestKey, encryption_key::EncryptionKey},
};
use aptos_crypto::{bls12381, weighted_config::WeightedConfigArkworks, TSecretSharingConfig};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::pvss::{
    chunky::{
        DecryptPrivKey, EncryptPubKey, InputSecret, PublicParameters, SignedWeightedTranscript,
        WeightedSubtranscript,
    },
    traits::{
        transcript::{Aggregatable, HasAggregatableSubtranscript, Transcript},
        TranscriptCore,
    },
    Player,
};
use ark_ec::AffineRepr;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag,
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use rand::{rngs::StdRng, CryptoRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

pub type ChunkyTranscript = SignedWeightedTranscript<Pairing>;
pub type ChunkySubtranscript = WeightedSubtranscript<Pairing>;
pub type DealerPrivateKey = bls12381::PrivateKey;
pub type DealerPublicKey = bls12381::PublicKey;
pub type ChunkyDKGThresholdConfig = WeightedConfigArkworks<Fr>;
pub type ChunkyEncryptPubKey = EncryptPubKey<Pairing>;
pub type ChunkyDecryptPrivKey = DecryptPrivKey<Pairing>;
pub type ChunkyDKGPublicParameters = PublicParameters<Pairing>;
pub type ChunkyInputSecret = InputSecret<Fr>;
/// Shared test DigestKey for encryption key derivation.
/// TODO(ibalajiarun): Replace with proper trusted setup for production.
pub static TEST_DIGEST_KEY: Lazy<DigestKey> = Lazy::new(|| {
    use ark_std::rand::SeedableRng;
    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(100u64);
    DigestKey::new(&mut rng, 32, 200).expect("DigestKey creation should not fail")
});

/// An aggregated transcript with the list of dealers who contributed to it.
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AggregatedSubtranscript {
    pub subtranscript: ChunkySubtranscript,
    pub dealers: Vec<Player>,
}

impl AggregatedSubtranscript {
    /// Derive the encryption key bytes from this transcript using the given tau_g2.
    pub fn derive_encryption_key_bytes(&self, tau_g2: G2Affine) -> Result<Vec<u8>> {
        let mpk_g2 = self.subtranscript.get_dealt_public_key().as_g2();
        let encryption_key = EncryptionKey::new(mpk_g2, tau_g2);
        bcs::to_bytes(&encryption_key)
            .map_err(|e| anyhow::anyhow!("encryption key serialization error: {e}"))
    }
}

/// Chunky DKG transcript and its metadata.
/// Similar to DKGTranscript but for Chunky DKG with ChunkyTranscript.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkyDKGTranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
}

impl Debug for ChunkyDKGTranscript {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkyDKGTranscript")
            .field("metadata", &self.metadata)
            .field("transcript_bytes_len", &self.transcript_bytes.len())
            .finish()
    }
}

impl ChunkyDKGTranscript {
    pub fn new(epoch: u64, author: AccountAddress, transcript_bytes: Vec<u8>) -> Self {
        Self {
            metadata: DKGTranscriptMetadata { epoch, author },
            transcript_bytes,
        }
    }

    pub fn dummy() -> Self {
        Self {
            metadata: DKGTranscriptMetadata {
                epoch: 0,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChunkyDKGConfig {
    pub threshold_config: ChunkyDKGThresholdConfig,
    pub public_parameters: ChunkyDKGPublicParameters,
    pub session_metadata: ChunkyDKGSessionMetadata,
    pub eks: Vec<ChunkyEncryptPubKey>,
}

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
        OnChainChunkyDKGConfig::try_from(self.chunky_dkg_config.clone()).ok()
    }
}

/// Reflection of `0x1::chunky_dkg::ChunkyDKGStartEvent` in rust for Chunky DKG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkyDKGStartEvent {
    pub session_metadata: ChunkyDKGSessionMetadata,
    pub start_time_us: u64,
}

impl MoveStructType for ChunkyDKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("chunky_dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ChunkyDKGStartEvent");
}

pub static CHUNKY_DKG_START_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(ChunkyDKGStartEvent::struct_tag())));

pub struct ChunkyDKG {
    threshold_config: ChunkyDKGThresholdConfig,
    public_parameters: PublicParameters<Pairing>,
}

// TODO(ibalajiarun): make the APIs consistent. Is this struct event necessary?
impl ChunkyDKG {
    pub fn new(
        secret_sharing_config: ChunkyDKGThresholdConfig,
        public_parameters: PublicParameters<Pairing>,
    ) -> Self {
        Self {
            threshold_config: secret_sharing_config,
            public_parameters,
        }
    }

    pub fn deal<A: Serialize + Clone, R: RngCore + CryptoRng>(
        dkg_config: &ChunkyDKGConfig,
        ssk: &DealerPrivateKey,
        spk: &DealerPublicKey,
        s: &ChunkyInputSecret,
        sid: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> ChunkyTranscript {
        ChunkyTranscript::deal(
            &dkg_config.threshold_config,
            &dkg_config.public_parameters,
            ssk,
            spk,
            &dkg_config.eks,
            s,
            sid,
            dealer,
            rng,
        )
    }

    pub fn verify<A: Serialize + Clone, R: RngCore + CryptoRng>(
        &self,
        transcript: &ChunkyTranscript,
        spks: &[DealerPublicKey],
        eks: &[ChunkyEncryptPubKey],
        sid: &A,
        rng: &mut R,
    ) -> Result<()> {
        transcript.verify(
            &self.threshold_config,
            &self.public_parameters,
            spks,
            eks,
            sid,
            rng,
        )
    }

    pub fn sub_aggregate(
        &self,
        sub_transcripts: &[ChunkySubtranscript],
    ) -> Result<ChunkySubtranscript> {
        // Do all aggregations in projective form, then normalize to affine
        ChunkySubtranscript::aggregate(&self.threshold_config, sub_transcripts.to_vec())
    }

    /// Generate secret sharing config and public parameters from DKG session metadata.
    /// Similar to `RealDKG::new_public_params` but returns the config components directly.
    pub fn generate_config(dkg_session_metadata: &ChunkyDKGSessionMetadata) -> ChunkyDKGConfig {
        let onchain_config = dkg_session_metadata
            .into_on_chain_chunky_dkg_config()
            .unwrap_or_else(OnChainChunkyDKGConfig::default_disabled);
        let secrecy_threshold = onchain_config
            .secrecy_threshold()
            .unwrap_or_else(|| *DEFAULT_SECRECY_THRESHOLD);
        let reconstruct_threshold = onchain_config
            .reconstruct_threshold()
            .unwrap_or_else(|| *DEFAULT_RECONSTRUCT_THRESHOLD);

        let target_validators = dkg_session_metadata.target_validator_consensus_infos_cloned();
        let validator_stakes: Vec<u64> =
            target_validators.iter().map(|vi| vi.voting_power).collect();

        let eks: Vec<ChunkyEncryptPubKey> = target_validators
            .iter()
            .map(|vi| (&vi.public_key).into())
            .collect();

        // Use the same rounding logic as RealDKG to compute weights
        // TODO(ibalajiarun): Just compute profile instead of doing Blss things with DKGRounding
        let DKGRounding { profile, .. } = DKGRounding::new(
            &validator_stakes,
            secrecy_threshold,
            reconstruct_threshold,
        );

        // Create WeightedConfigArkworks<Fr> from the computed weights
        let threshold_config = ChunkyDKGThresholdConfig::new(
            profile.reconstruct_threshold_in_weights as usize,
            profile
                .validator_weights
                .iter()
                .map(|w| *w as usize)
                .collect(),
        )
        .expect("Failed to create WeightedConfigArkworks");

        // Create PublicParameters<Pairing> with max_num_shares based on total weight
        // TODO(ibalajiarun): Modify PublicParameters to take in u64 weights.
        let total_weight: u32 = profile.validator_weights.iter().sum::<u64>() as u32;

        // TODO(ibalajiarun): Replace seed for public parameters with a trusted setup
        let seed = dkg_session_metadata.dealer_epoch;
        let mut rng_aptos = StdRng::seed_from_u64(seed);
        let public_parameters = PublicParameters::new_with_commitment_base(
            total_weight as usize,
            aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_TESTING,
            threshold_config.get_total_num_players(),
            G2Affine::generator(),
            &mut rng_aptos,
        );

        ChunkyDKGConfig {
            threshold_config,
            public_parameters,
            session_metadata: dkg_session_metadata.clone(),
            eks,
        }
    }
}

/// Wrapper so that transcript bytes can be used with verify_multi_signatures (requires CryptoHash).
/// BCS(TranscriptBytesForSigning(bytes)) equals BCS(bytes), so the hash matches what was signed.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct TranscriptBytesForSigning(#[serde(with = "serde_bytes")] pub Vec<u8>);

/// A validated aggregated transcript with metadata, similar to DKGTranscript but for Chunky DKG.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CertifiedAggregatedChunkySubtranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
    pub signature: AggregateSignature,
}

impl std::fmt::Debug for CertifiedAggregatedChunkySubtranscript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertifiedAggregatedChunkySubtranscript")
            .field("metadata", &self.metadata)
            .field("transcript_bytes_len", &self.transcript_bytes.len())
            .finish()
    }
}

/// Output of Chunky DKG: the certified transcript + derived encryption key.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CertifiedChunkyDKGOutput {
    pub certified_transcript: CertifiedAggregatedChunkySubtranscript,
    #[serde(with = "serde_bytes")]
    pub encryption_key: Vec<u8>,
}

/// Reflection of Move type `0x1::dkg::DKGSessionState`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkyDKGSessionState {
    pub metadata: ChunkyDKGSessionMetadata,
    pub start_time_us: u64,
    pub transcript: Vec<u8>,
}

impl ChunkyDKGSessionState {
    pub fn target_epoch(&self) -> u64 {
        self.metadata.dealer_epoch + 1
    }
}

/// Reflection of Move type `0x1::dkg::DKGState`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkyDKGState {
    pub last_completed: Option<ChunkyDKGSessionState>,
    pub in_progress: Option<ChunkyDKGSessionState>,
}

impl ChunkyDKGState {
    pub fn maybe_last_complete(&self, epoch: u64) -> Option<&ChunkyDKGSessionState> {
        match &self.last_completed {
            Some(session) if session.target_epoch() == epoch => Some(session),
            _ => None,
        }
    }

    pub fn last_complete(&self) -> &ChunkyDKGSessionState {
        self.last_completed.as_ref().unwrap()
    }
}

impl OnChainConfig for ChunkyDKGState {
    const MODULE_IDENTIFIER: &'static str = "chunky_dkg";
    const TYPE_IDENTIFIER: &'static str = "ChunkyDKGState";
}
