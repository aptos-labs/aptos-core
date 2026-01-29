// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::pvss::Player;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    dkg::{
        chunky_dkg::{ChunkyDKGTranscript, ChunkySubtranscript},
        DKGTranscriptMetadata,
    },
};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// Once Chunky DKG starts, a validator should send this message to peers in order to collect Chunky DKG transcripts from peers.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChunkyDKGTranscriptRequest {
    pub dealer_epoch: u64,
}

impl ChunkyDKGTranscriptRequest {
    pub fn new(dealer_epoch: u64) -> Self {
        Self { dealer_epoch }
    }
}

/// Request to validate an aggregated subtranscript. Contains the hash of the subtranscript and the dealers.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct ChunkyDKGSubtranscriptSignatureRequest {
    pub dealer_epoch: u64,
    pub subtranscript_hash: HashValue,
    pub aggregated_subtrx_dealers: Vec<Player>,
}

impl ChunkyDKGSubtranscriptSignatureRequest {
    pub fn new(
        dealer_epoch: u64,
        subtranscript_hash: HashValue,
        aggregated_subtrx_dealers: Vec<Player>,
    ) -> Self {
        Self {
            dealer_epoch,
            subtranscript_hash,
            aggregated_subtrx_dealers,
        }
    }
}

/// Response containing a signature for subtranscript validation.
/// The signature is over the subtranscript itself.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChunkyDKGSubtranscriptSignatureResponse {
    pub dealer_epoch: u64,
    pub subtranscript_hash: HashValue,
    pub signature: aptos_crypto::bls12381::Signature,
}

impl ChunkyDKGSubtranscriptSignatureResponse {
    pub fn new(
        dealer_epoch: u64,
        subtranscript_hash: HashValue,
        signature: aptos_crypto::bls12381::Signature,
    ) -> Self {
        Self {
            dealer_epoch,
            subtranscript_hash,
            signature,
        }
    }
}

/// An aggregated transcript with the list of dealers who contributed to it.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AggregatedSubtranscript {
    pub subtranscript: ChunkySubtranscript,
    pub dealers: Vec<Player>,
}

/// A validated aggregated subtranscript with an aggregate signature that can verify it.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertifiedAggregatedSubtranscript {
    pub aggregated_subtranscript: AggregatedSubtranscript,
    pub aggregate_signature: AggregateSignature,
}

/// A validated aggregated transcript with metadata, similar to DKGTranscript but for Chunky DKG.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatedAggregatedTranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
}

impl std::fmt::Debug for ValidatedAggregatedTranscript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedAggregatedTranscript")
            .field("metadata", &self.metadata)
            .field("transcript_bytes_len", &self.transcript_bytes.len())
            .finish()
    }
}

/// Request to fetch missing transcripts from a peer who has aggregated transcripts.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct MissingTranscriptRequest {
    pub dealer_epoch: u64,
    pub missing_dealer: AccountAddress,
}

impl MissingTranscriptRequest {
    pub fn new(epoch: u64, missing_dealer: AccountAddress) -> Self {
        Self {
            dealer_epoch: epoch,
            missing_dealer,
        }
    }
}

/// Response containing the requested transcript.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MissingTranscriptResponse {
    pub transcript: ChunkyDKGTranscript,
}

impl MissingTranscriptResponse {
    pub fn new(transcript: ChunkyDKGTranscript) -> Self {
        Self { transcript }
    }
}
