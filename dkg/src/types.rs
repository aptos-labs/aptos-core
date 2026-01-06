// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto_derive::CryptoHasher;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
pub use aptos_types::dkg::{DKGTranscript, DKGTranscriptMetadata};
use serde::{Deserialize, Serialize};

/// Once DKG starts, a validator should send this message to peers in order to collect DKG transcripts from peers.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct DKGTranscriptRequest {
    dealer_epoch: u64,
}

impl DKGTranscriptRequest {
    pub fn new(epoch: u64) -> Self {
        Self {
            dealer_epoch: epoch,
        }
    }
}

/// Request to validate an aggregated subtranscript. Contains the hash of the subtranscript and the dealers.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct DKGSubtranscriptValidationRequest {
    pub dealer_epoch: u64,
    pub subtranscript_hash: Vec<u8>,
    pub dealers: Vec<aptos_dkg::pvss::Player>,
}

impl DKGSubtranscriptValidationRequest {
    pub fn new(
        epoch: u64,
        subtranscript_hash: Vec<u8>,
        dealers: Vec<aptos_dkg::pvss::Player>,
    ) -> Self {
        Self {
            dealer_epoch: epoch,
            subtranscript_hash,
            dealers,
        }
    }
}

/// Response containing a signature for subtranscript validation.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DKGSubtranscriptValidationResponse {
    pub metadata: DKGTranscriptMetadata,
    pub signature: aptos_crypto::bls12381::Signature,
}

impl DKGSubtranscriptValidationResponse {
    pub fn new(
        metadata: DKGTranscriptMetadata,
        signature: aptos_crypto::bls12381::Signature,
    ) -> Self {
        Self {
            metadata,
            signature,
        }
    }
}

/// The DKG network message.
#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    TranscriptRequest(DKGTranscriptRequest),
    TranscriptResponse(DKGTranscript),
    SubtranscriptValidationRequest(DKGSubtranscriptValidationRequest),
    SubtranscriptValidationResponse(DKGSubtranscriptValidationResponse),
}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::TranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::TranscriptResponse(response) => response.metadata.epoch,
            DKGMessage::SubtranscriptValidationRequest(request) => request.dealer_epoch,
            DKGMessage::SubtranscriptValidationResponse(response) => response.metadata.epoch,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DKGMessage::TranscriptRequest(_) => "DKGTranscriptRequest",
            DKGMessage::TranscriptResponse(_) => "DKGTranscriptResponse",
            DKGMessage::SubtranscriptValidationRequest(_) => "DKGSubtranscriptValidationRequest",
            DKGMessage::SubtranscriptValidationResponse(_) => "DKGSubtranscriptValidationResponse",
        }
    }
}

impl RBMessage for DKGMessage {}
