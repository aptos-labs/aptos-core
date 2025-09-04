// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto_derive::CryptoHasher;
use velor_enum_conversion_derive::EnumConversion;
use velor_reliable_broadcast::RBMessage;
pub use velor_types::dkg::DKGTranscript;
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

/// The DKG network message.
#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    TranscriptRequest(DKGTranscriptRequest),
    TranscriptResponse(DKGTranscript),
}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::TranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::TranscriptResponse(response) => response.metadata.epoch,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DKGMessage::TranscriptRequest(_) => "DKGTranscriptRequest",
            DKGMessage::TranscriptResponse(_) => "DKGTranscriptResponse",
        }
    }
}

impl RBMessage for DKGMessage {}
