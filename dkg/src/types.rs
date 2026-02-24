// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use crate::chunky::types::{
    ChunkyDKGSubtranscriptSignatureRequest, ChunkyDKGSubtranscriptSignatureResponse,
    ChunkyDKGTranscriptRequest, MissingTranscriptRequest, MissingTranscriptResponse,
    ValidatedAggregatedTranscript,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
pub use aptos_types::dkg::{chunky_dkg::ChunkyDKGTranscript, DKGTranscript, DKGTranscriptMetadata};
use serde::{Deserialize, Serialize};

/// Once DKG starts, a validator should send this message to peers in order to collect DKG transcripts from peers.
/// This is the request for the Das DKG implementation
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

/// The DKG network message.
#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    TranscriptRequest(DKGTranscriptRequest),
    TranscriptResponse(DKGTranscript),
    ChunkyTranscriptRequest(ChunkyDKGTranscriptRequest),
    ChunkyTranscriptResponse(ChunkyDKGTranscript),
    SubtranscriptSignatureRequest(ChunkyDKGSubtranscriptSignatureRequest),
    SubtranscriptSignatureResponse(ChunkyDKGSubtranscriptSignatureResponse),
    MissingTranscriptRequest(MissingTranscriptRequest),
    MissingTranscriptResponse(MissingTranscriptResponse),
}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::TranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::TranscriptResponse(response) => response.metadata.epoch,
            DKGMessage::ChunkyTranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::ChunkyTranscriptResponse(response) => response.metadata.epoch,
            DKGMessage::SubtranscriptSignatureRequest(request) => request.dealer_epoch,
            DKGMessage::SubtranscriptSignatureResponse(response) => response.dealer_epoch,
            DKGMessage::MissingTranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::MissingTranscriptResponse(response) => response.transcript.metadata.epoch,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DKGMessage::TranscriptRequest(_) => "DKGTranscriptRequest",
            DKGMessage::TranscriptResponse(_) => "DKGTranscriptResponse",
            DKGMessage::ChunkyTranscriptRequest(_) => "ChunkyDKGTranscriptRequest",
            DKGMessage::ChunkyTranscriptResponse(_) => "ChunkyDKGTranscriptResponse",
            DKGMessage::SubtranscriptSignatureRequest(_) => {
                "ChunkyDKGSubtranscriptSignatureRequest"
            },
            DKGMessage::SubtranscriptSignatureResponse(_) => {
                "ChunkyDKGSubtranscriptSignatureResponse"
            },
            DKGMessage::MissingTranscriptRequest(_) => "MissingTranscriptRequest",
            DKGMessage::MissingTranscriptResponse(_) => "MissingTranscriptResponse",
        }
    }
}

impl RBMessage for DKGMessage {}
