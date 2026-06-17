// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aggregate_signature::AggregateSignature,
    chain_id::ChainId,
    dkg::{
        real_dkg::rounding::{
            DKGRoundingProfile, DEFAULT_RECONSTRUCT_THRESHOLD, DEFAULT_SECRECY_THRESHOLD,
        },
        DKGTranscriptMetadata,
    },
    on_chain_config::{ChunkyDKGConfigMoveStruct, OnChainChunkyDKGConfig, OnChainConfig},
    validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
};
use anyhow::Result;
use aptos_batch_encryption::{
    group::{Fr, G2Affine, Pairing},
    shared::{
        digest::{DigestKey, DigestKeyHeader, DigestKeyView, RoundData},
        digest_key_store::{DigestKeyStore, DigestKeyStoreConfig},
        encryption_key::EncryptionKey,
    },
};
use aptos_bitvec::BitVec;
use aptos_crypto::{bls12381, weighted_config::WeightedConfigArkworks};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::pvss::{
    chunky::{
        DecryptPrivKey, EncryptPubKey, InputSecret, PublicParameters, SignedWeightedTranscript,
        WeightedSubtranscript,
    },
    traits::TranscriptCore,
};
use ark_ec::AffineRepr;
use fixed::types::U64F64;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag,
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    fmt::{Debug, Formatter},
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Instant,
};

/// MaxBcsSize: 818 + 32·n + 120·W + 24·max_w + 128·(W + max_w)·c + 80·ell.
pub type ChunkyTranscript = SignedWeightedTranscript<Pairing>;
/// ArkSize: 120 + 16·n + 104·W + 8·max_w + 48·(W + max_w)·c.
pub type ChunkySubtranscript = WeightedSubtranscript<Pairing>;
pub type DealerPrivateKey = bls12381::PrivateKey;
pub type DealerPublicKey = bls12381::PublicKey;
pub type ChunkyDKGThresholdConfig = WeightedConfigArkworks<Fr>;
pub type ChunkyEncryptPubKey = EncryptPubKey<Pairing>;
pub type ChunkyDecryptPrivKey = DecryptPrivKey<Pairing>;
pub type ChunkyDKGPublicParameters = PublicParameters<Pairing>;
pub type ChunkyInputSecret = InputSecret<Fr>;
/// Shared test DigestKey for encryption key derivation (unit tests only).
pub static TEST_DIGEST_KEY: Lazy<Arc<DigestKey>> = Lazy::new(|| {
    use ark_std::rand::SeedableRng;
    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(100u64);
    Arc::new(DigestKey::new(&mut rng, 32, 200).expect("DigestKey creation should not fail"))
});
/// Shared test PublicParameters for chunky DKG (unit tests only).
/// Sized for n=20 (forge): `max_num_shares=72` (rounding upper bound),
/// `max_aggregation=20` (per-chunk dealer-sum ceiling).
pub static TEST_PUBLIC_PARAMETERS: Lazy<Arc<ChunkyDKGPublicParameters>> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(200u64);
    Arc::new(PublicParameters::new_for_testing(
        72,
        aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_DEPLOYMENT,
        20,
        G2Affine::generator(),
        &mut rng,
    ))
});

/// Path to the BCS-serialized PublicParameters blob file.
static PUBLIC_PARAMETERS_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Deferred PublicParameters source — no expensive work at init time.
#[derive(Debug)]
enum PublicParametersOverride {
    /// Resolve to TEST_PUBLIC_PARAMETERS on first access (deferred).
    TestFallback,
    /// An explicit Arc (e.g. set directly in tests).
    Explicit(Arc<ChunkyDKGPublicParameters>),
}

/// Deferred PublicParameters override (for test chains).
static PUBLIC_PARAMETERS_OVERRIDE: OnceLock<PublicParametersOverride> = OnceLock::new();

/// Production PublicParameters: checks override first, then reads from file path.
/// Returns `None` if neither was configured or if reading/deserializing fails.
/// TEST_PUBLIC_PARAMETERS is only evaluated here (on first access), not at boot.
pub static PUBLIC_PARAMETERS: Lazy<Option<Arc<ChunkyDKGPublicParameters>>> = Lazy::new(|| {
    match PUBLIC_PARAMETERS_OVERRIDE.get() {
        Some(PublicParametersOverride::TestFallback) => {
            return Some(Arc::clone(&TEST_PUBLIC_PARAMETERS));
        },
        Some(PublicParametersOverride::Explicit(pp)) => {
            return Some(Arc::clone(pp));
        },
        None => {},
    }
    let path = PUBLIC_PARAMETERS_PATH.get()?;
    let start = Instant::now();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(
                "[PublicParameters] failed to read blob file {}: {}",
                path.display(),
                e
            );
            return None;
        },
    };
    let pp: ChunkyDKGPublicParameters = match bcs::from_bytes(&bytes) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!(
                "[PublicParameters] failed to deserialize blob ({} bytes): {}",
                bytes.len(),
                e
            );
            return None;
        },
    };
    let elapsed = start.elapsed();
    tracing::info!(
        "[PublicParameters] loaded from {} ({} bytes) in {:?}",
        path.display(),
        bytes.len(),
        elapsed,
    );
    Some(Arc::new(pp))
});

/// Store the path to the PublicParameters blob file. No I/O is performed.
pub fn set_public_parameters_path(path: PathBuf) {
    PUBLIC_PARAMETERS_PATH
        .set(path)
        .expect("PublicParameters path already set");
}

/// Directly set the PublicParameters.
pub fn set_public_parameters(pp: Arc<ChunkyDKGPublicParameters>) {
    PUBLIC_PARAMETERS_OVERRIDE
        .set(PublicParametersOverride::Explicit(pp))
        .expect("PublicParameters already set");
}

/// Result of early PublicParameters initialization (metadata only, no file read).
#[derive(Debug)]
pub enum PublicParametersSource {
    /// A blob file exists and will be lazily read on first access.
    WillLoadFromFile { path: PathBuf, file_size: u64 },
    /// Will fall back to the built-in test parameters on first access.
    TestKeyFallback,
    /// No PublicParameters available (no path configured, not a test chain).
    NotAvailable,
}

/// Initialize the PublicParameters source. No expensive work is performed;
/// TEST_PUBLIC_PARAMETERS construction is deferred to first access.
/// Unlike DigestKey, PublicParameters are needed by all nodes (including fullnodes)
/// to construct ChunkyDKGSession during state sync.
pub fn initialize_public_parameters(chain_id: ChainId) -> PublicParametersSource {
    if let Some(path) = PUBLIC_PARAMETERS_PATH.get() {
        match std::fs::metadata(path) {
            Ok(meta) => PublicParametersSource::WillLoadFromFile {
                path: path.clone(),
                file_size: meta.len(),
            },
            Err(_) => PublicParametersSource::NotAvailable,
        }
    } else if chain_id == ChainId::test() {
        let _ = PUBLIC_PARAMETERS_OVERRIDE.set(PublicParametersOverride::TestFallback);
        PublicParametersSource::TestKeyFallback
    } else {
        PublicParametersSource::NotAvailable
    }
}

/// Path to the BCS-serialized DigestKey blob file.
static DIGEST_KEY_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Streaming-store config. Set before `initialize_digest_key` if non-default values are needed.
static DIGEST_KEY_STORE_CONFIG: OnceLock<DigestKeyStoreConfig> = OnceLock::new();

/// Backing storage for the production `DigestKey`.
///
/// Either a streaming, bounded-residency `DigestKeyStore` (production, file-backed) or an
/// in-memory `DigestKey` (test chains and explicit-set tests). Both implement `DigestKeyView`,
/// so consumers see one uniform surface.
pub enum DigestKeyHandle {
    Store(Arc<DigestKeyStore>),
    InMemory(Arc<DigestKey>),
}

impl DigestKeyHandle {
    pub fn header(&self) -> &DigestKeyHeader {
        match self {
            DigestKeyHandle::Store(s) => DigestKeyView::header(s.as_ref()),
            DigestKeyHandle::InMemory(k) => DigestKeyView::header(k.as_ref()),
        }
    }

    pub fn round(&self, r: usize) -> Arc<RoundData> {
        match self {
            DigestKeyHandle::Store(s) => DigestKeyView::round(s.as_ref(), r),
            DigestKeyHandle::InMemory(k) => DigestKeyView::round(k.as_ref(), r),
        }
    }

    pub fn tau_g2(&self) -> G2Affine {
        self.header().tau_g2
    }

    pub fn num_rounds(&self) -> usize {
        self.header().num_rounds
    }
}

impl DigestKeyView for DigestKeyHandle {
    fn header(&self) -> &DigestKeyHeader {
        DigestKeyHandle::header(self)
    }

    fn round(&self, r: usize) -> Arc<RoundData> {
        DigestKeyHandle::round(self, r)
    }
}

/// Deferred resolution of which backing source to use. Set by `initialize_digest_key`.
enum DigestKeyResolution {
    /// File-backed; lazy `DigestKeyStore::open` on first access.
    FromFile {
        path: PathBuf,
        cfg: DigestKeyStoreConfig,
    },
    /// Test-chain validator fallback — `TEST_DIGEST_KEY` evaluated on first access.
    TestFallback,
    /// Explicit `set_digest_key` from tests.
    Explicit(Arc<DigestKey>),
}

static DIGEST_KEY_RESOLUTION: OnceLock<DigestKeyResolution> = OnceLock::new();

/// Production `DigestKey` handle. Lazily resolves on first access, opening the streaming store
/// or installing the in-memory test key as needed.
pub static DIGEST_KEY: Lazy<Option<Arc<DigestKeyHandle>>> = Lazy::new(|| {
    let resolution = DIGEST_KEY_RESOLUTION.get()?;
    match resolution {
        DigestKeyResolution::TestFallback => Some(Arc::new(DigestKeyHandle::InMemory(Arc::clone(
            &TEST_DIGEST_KEY,
        )))),
        DigestKeyResolution::Explicit(key) => {
            Some(Arc::new(DigestKeyHandle::InMemory(Arc::clone(key))))
        },
        DigestKeyResolution::FromFile { path, cfg } => {
            let start = Instant::now();
            match DigestKeyStore::open(path, cfg.clone()) {
                Ok(store) => {
                    tracing::info!(
                        "[DigestKey] streaming store opened for {} in {:?}",
                        path.display(),
                        start.elapsed(),
                    );
                    Some(Arc::new(DigestKeyHandle::Store(store)))
                },
                Err(e) => {
                    tracing::error!(
                        "[DigestKey] failed to open streaming store for {}: {}",
                        path.display(),
                        e
                    );
                    None
                },
            }
        },
    }
});

/// Store the path to the DigestKey blob file. No I/O is performed.
pub fn set_digest_key_path(path: PathBuf) {
    DIGEST_KEY_PATH
        .set(path)
        .expect("DigestKey path already set");
}

/// Override the default streaming-store tuning. Must be called before `initialize_digest_key`.
pub fn set_digest_key_store_config(cfg: DigestKeyStoreConfig) {
    DIGEST_KEY_STORE_CONFIG
        .set(cfg)
        .expect("DigestKeyStore config already set");
}

/// Mirror of [`DigestKeyStoreConfig`] suitable for `aptos-node` to pass in without a direct
/// dependency on `aptos-batch-encryption`. The fields match `DigestKeyStoreConfig`.
#[derive(Clone, Debug)]
pub struct DigestKeyStoreConfigOverride {
    pub pinned_prefix_rounds: usize,
    pub sliding_lookback_rounds: usize,
    pub sliding_lookahead_rounds: usize,
    pub read_batch_rounds: usize,
}

impl From<DigestKeyStoreConfigOverride> for DigestKeyStoreConfig {
    fn from(o: DigestKeyStoreConfigOverride) -> Self {
        DigestKeyStoreConfig {
            pinned_prefix_rounds: o.pinned_prefix_rounds,
            sliding_lookback_rounds: o.sliding_lookback_rounds,
            sliding_lookahead_rounds: o.sliding_lookahead_rounds,
            read_batch_rounds: o.read_batch_rounds,
        }
    }
}

/// Directly set an in-memory DigestKey. Used by tests that need a specific test setup
/// without going through the file path.
pub fn set_digest_key(key: Arc<DigestKey>) {
    DIGEST_KEY_RESOLUTION
        .set(DigestKeyResolution::Explicit(key))
        .map_err(|_| ())
        .expect("DigestKey already set");
}

/// Result of early DigestKey initialization (metadata only, no file read).
#[derive(Debug)]
pub enum DigestKeySource {
    /// A blob file exists and will be lazily read on first access.
    WillLoadFromFile { path: PathBuf, file_size: u64 },
    /// Fell back to the built-in test key.
    TestKeyFallback,
    /// No DigestKey is available (no path configured, not a test chain).
    NotAvailable,
}

/// Initialize the DigestKey source. Checks metadata only (no file read).
/// On test chains without an explicit path, sets the test key override
/// for validator nodes only. Non-validator nodes (fullnodes) should not
/// have a digest key since they don't participate in decryption.
pub fn initialize_digest_key(chain_id: ChainId, is_validator: bool) -> DigestKeySource {
    if let Some(path) = DIGEST_KEY_PATH.get() {
        match std::fs::metadata(path) {
            Ok(meta) => {
                let cfg = DIGEST_KEY_STORE_CONFIG.get().cloned().unwrap_or_default();
                let _ = DIGEST_KEY_RESOLUTION.set(DigestKeyResolution::FromFile {
                    path: path.clone(),
                    cfg,
                });
                DigestKeySource::WillLoadFromFile {
                    path: path.clone(),
                    file_size: meta.len(),
                }
            },
            Err(_) => DigestKeySource::NotAvailable,
        }
    } else if chain_id == ChainId::test() && is_validator {
        let _ = DIGEST_KEY_RESOLUTION.set(DigestKeyResolution::TestFallback);
        DigestKeySource::TestKeyFallback
    } else {
        DigestKeySource::NotAvailable
    }
}

/// An aggregated transcript with the set of dealers who contributed to it.
///
/// Dealers are represented as a `BitVec` bitmask over validator indices,
/// which inherently prevents duplicates and ensures canonical ordering.
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AggregatedSubtranscript {
    pub dealer_epoch: u64,
    pub subtranscript: ChunkySubtranscript,
    pub dealer_bitmask: BitVec,
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
pub struct ChunkyDKGSession {
    pub threshold_config: ChunkyDKGThresholdConfig,
    pub public_parameters: Arc<ChunkyDKGPublicParameters>,
    pub session_metadata: ChunkyDKGSessionMetadata,
    pub eks: Vec<ChunkyEncryptPubKey>,
}

impl ChunkyDKGSession {
    /// Create a new DKG session from on-chain session metadata.
    pub fn new(dkg_session_metadata: &ChunkyDKGSessionMetadata) -> Arc<ChunkyDKGSession> {
        let onchain_config = dkg_session_metadata
            .into_on_chain_chunky_dkg_config()
            .unwrap_or_else(OnChainChunkyDKGConfig::default_disabled);
        let secrecy_threshold = onchain_config
            .secrecy_threshold()
            .unwrap_or_else(|| *DEFAULT_SECRECY_THRESHOLD);
        let reconstruct_threshold = onchain_config
            .reconstruct_threshold()
            .unwrap_or_else(|| *DEFAULT_RECONSTRUCT_THRESHOLD);
        let reconstruct_threshold = max(reconstruct_threshold, secrecy_threshold + U64F64::DELTA);

        let target_validators = dkg_session_metadata.target_validator_consensus_infos_cloned();
        let validator_stakes: Vec<u64> =
            target_validators.iter().map(|vi| vi.voting_power).collect();

        let eks: Vec<ChunkyEncryptPubKey> = target_validators
            .iter()
            .map(|vi| (&vi.public_key).into())
            .collect();

        let profile = DKGRoundingProfile::new(
            &validator_stakes,
            secrecy_threshold,
            reconstruct_threshold,
            None,
        )
        .unwrap_or_else(|_| {
            DKGRoundingProfile::infallible(
                &validator_stakes,
                secrecy_threshold,
                reconstruct_threshold,
                None,
            )
        });

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

        let public_parameters = PUBLIC_PARAMETERS
            .as_ref()
            .expect("PublicParameters not initialized; call initialize_public_parameters first")
            .clone();

        Arc::new(ChunkyDKGSession {
            threshold_config,
            public_parameters,
            session_metadata: dkg_session_metadata.clone(),
            eks,
        })
    }

    /// BCS wire-size upper bound for an inbound `ChunkyTranscript`. Used as a structural
    /// validation gate before deserialization.
    ///
    /// Formula taken from `aptos_dkg::pvss::signed::generic_signing::GenericSigning`'s
    /// `MaxBcsSize(T=UnsignedWeightedTranscript<Bls12_381>)` annotation.
    pub fn expected_max_transcript_size(&self) -> usize {
        const FR_BITS: usize = 255;
        let n = self.eks.len();
        let w = self.threshold_config.get_total_weight();
        let max_w = self.threshold_config.get_max_weight();
        let ell = self.public_parameters.ell.max(1);
        let c = FR_BITS.div_ceil(ell);
        let max_bcs_size = 818 + 32 * n + 120 * w + 24 * max_w + 128 * (w + max_w) * c + 80 * ell;
        const FIXED_SLACK: usize = 4 * 1024;
        max_bcs_size + FIXED_SLACK
    }
}

/// Reflection of `0x1::chunky_dkg::DKGSessionMetadata` in rust.
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

/// Reflection of Move type `0x1::chunky_dkg::ChunkyDKGSessionState`.
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

/// Reflection of Move type `0x1::chunky_dkg::ChunkyDKGState`.
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

#[cfg(test)]
mod transcript_size_bound_tests {
    use super::*;
    use aptos_crypto::{SigningKey, Uniform};
    use aptos_dkg::pvss::{
        chunky::DEFAULT_ELL_FOR_DEPLOYMENT,
        traits::{transcript::Transcript, Convert, HasEncryptionPublicParams},
        Player,
    };
    use rand::thread_rng;

    fn deal_and_check(weights: Vec<usize>, max_num_shares: usize) {
        let mut rng_pp = StdRng::seed_from_u64(0xC0FFEE);
        let pp = Arc::new(ChunkyDKGPublicParameters::new_for_testing(
            max_num_shares,
            DEFAULT_ELL_FOR_DEPLOYMENT,
            4,
            G2Affine::generator(),
            &mut rng_pp,
        ));
        let total_w: usize = weights.iter().sum();
        let threshold_config =
            ChunkyDKGThresholdConfig::new(total_w.div_ceil(2).max(1), weights.clone()).unwrap();
        let n = weights.len();
        let dks: Vec<ChunkyDecryptPrivKey> = (0..n)
            .map(|_| Uniform::generate(&mut thread_rng()))
            .collect();
        let eks: Vec<ChunkyEncryptPubKey> = dks
            .iter()
            .map(|dk| dk.to(pp.get_encryption_public_params()))
            .collect();
        let ssk: DealerPrivateKey = Uniform::generate(&mut thread_rng());
        let spk: DealerPublicKey = ssk.verifying_key();
        let session = ChunkyDKGSession {
            threshold_config,
            public_parameters: pp,
            session_metadata: ChunkyDKGSessionMetadata {
                dealer_epoch: 0,
                chunky_dkg_config: OnChainChunkyDKGConfig::default_enabled().into(),
                dealer_validator_set: vec![],
                target_validator_set: vec![],
            },
            eks,
        };
        let bound = session.expected_max_transcript_size();
        let mut rng = thread_rng();
        let secret = ChunkyInputSecret::generate(&mut rng);
        let trx = ChunkyTranscript::deal(
            &session.threshold_config,
            &session.public_parameters,
            &ssk,
            &spk,
            &session.eks,
            &secret,
            &session.session_metadata,
            &Player { id: 0 },
            &mut rng,
        );
        let actual = bcs::to_bytes(&trx).unwrap().len();
        let w = session.threshold_config.get_total_weight();
        let max_w = session.threshold_config.get_max_weight();
        println!(
            "[size-bound] W={w} max_w={max_w} n={n} actual={actual} bound={bound} headroom={}",
            bound as isize - actual as isize
        );
        assert!(actual <= bound, "actual {actual} > bound {bound}");
    }

    #[test]
    fn within_bound_w16() {
        deal_and_check(vec![4, 4, 4, 4], 24);
    }

    #[test]
    fn within_bound_w100() {
        deal_and_check(vec![20; 5], 128);
    }

    #[test]
    fn within_bound_w200() {
        deal_and_check(vec![40; 5], 256);
    }

    #[test]
    fn within_bound_w400() {
        deal_and_check(vec![40; 10], 512);
    }
}
