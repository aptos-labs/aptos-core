// Copyright © Aptos Foundation

use crate::{
    dkg::dummy_dkg::DummyDKG, on_chain_config::OnChainConfig,
    validator_verifier::ValidatorConsensusInfoMoveStruct,
};
use anyhow::Result;
use aptos_crypto::Uniform;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};
use rand::CryptoRng;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt::Debug};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGTranscriptMetadata {
    pub epoch: u64,
    pub author: AccountAddress,
}

/// Reflection of Move type `0x1::dkg::DKGStartEvent`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DKGStartEvent {
    pub session_metadata: DKGSessionMetadata,
    pub start_time_us: u64,
}

impl MoveStructType for DKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DKGStartEvent");
}

/// DKG transcript and its metadata.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DKGTranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
}

impl DKGTranscript {
    pub fn new(epoch: u64, author: AccountAddress, transcript_bytes: Vec<u8>) -> Self {
        Self {
            metadata: DKGTranscriptMetadata { epoch, author },
            transcript_bytes,
        }
    }
}

// The input of DKG.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionMetadata {
    pub dealer_epoch: u64,
    pub dealer_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
    pub target_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
}

// The input and the run state of DKG.
/// Reflection of Move type `0x1::dkg::DKGSessionState`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionState {
    pub metadata: DKGSessionMetadata,
    pub start_time_us: u64,
    pub result: Vec<u8>,
    pub deadline_microseconds: u64,
}

/// Reflection of Move type `0x1::dkg::DKGState`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub last_complete: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
}

impl OnChainConfig for DKGState {
    const MODULE_IDENTIFIER: &'static str = "dkg";
    const TYPE_IDENTIFIER: &'static str = "DKGState";
}

pub trait DKGTrait: Debug {
    type DealerPrivateKey;
    type PublicParams: Clone + Debug + Send + Sync;
    type Transcript: Clone + Default + Send + Sync + Serialize + for<'a> Deserialize<'a>;
    type InputSecret: Uniform;
    type DealtSecret;
    type DealtSecretShare;
    type NewValidatorDecryptKey;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> Self::PublicParams;
    fn generate_predictable_input_secret_for_testing(
        dealer_sk: &Self::DealerPrivateKey,
    ) -> Self::InputSecret;
    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret;
    fn dealt_secret_from_input(input: &Self::InputSecret) -> Self::DealtSecret;
    fn generate_transcript<R: CryptoRng>(
        rng: &mut R,
        params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript;

    fn verify_transcript(params: &Self::PublicParams, trx: &Self::Transcript) -> Result<()>;

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        transcripts: Vec<Self::Transcript>,
    ) -> Self::Transcript;
    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> Result<Self::DealtSecretShare>;
    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> Result<Self::DealtSecret>;
    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64>;
}

pub mod dummy_dkg;

// TODO: replace with RealDKG.
pub type DefaultDKG = DummyDKG;
