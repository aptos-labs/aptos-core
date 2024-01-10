// Copyright © Aptos Foundation

use crate::on_chain_config::{OnChainConfig, ValidatorSet};
use anyhow::Result;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};
use rand::CryptoRng;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGTranscriptMetadata {
    pub epoch: u64,
    pub author: AccountAddress,
}

/// Reflection of Move type `0x1::dkg::DKGStartEvent`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DKGStartEvent {
    pub target_epoch: u64,
    pub start_time_us: u64,
    pub target_validator_set: ValidatorSet,
}

impl MoveStructType for DKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DKGStartEvent");
}

/// DKG parameters.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGConfig {
    //TODO
}

/// DKG transcript and its metadata.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DKGNode {
    pub metadata: DKGTranscriptMetadata,
    pub transcript_bytes: Vec<u8>,
}

impl DKGNode {
    pub fn new(epoch: u64, author: AccountAddress, transcript_bytes: Vec<u8>) -> Self {
        Self {
            metadata: DKGTranscriptMetadata { epoch, author },
            transcript_bytes,
        }
    }
}

/// Reflection of Move type `0x1::dkg::DKGSessionState`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionState {
    pub start_time_us: u64,
    pub dealer_epoch: u64,
    pub dealer_validator_set: ValidatorSet,
    pub target_epoch: u64,
    pub target_validator_set: ValidatorSet,
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

pub trait DKGTrait {
    type PrivateParams;
    type PublicParams: Send + Sync;
    type Transcript: Clone + Default + Send + Sync + for<'a> Deserialize<'a>;

    fn generate_transcript<R: CryptoRng>(
        rng: &mut R,
        sk: &Self::PrivateParams,
        params: &Self::PublicParams,
    ) -> Self::Transcript;

    fn verify_transcript(params: &Self::PublicParams, trx: &Self::Transcript) -> Result<()>;

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        base: &mut Self::Transcript,
        extra: &Self::Transcript,
    );
}

pub trait DKGPrivateParamsProvider<DKG: DKGTrait> {
    fn dkg_private_params(&self) -> &DKG::PrivateParams;
}
