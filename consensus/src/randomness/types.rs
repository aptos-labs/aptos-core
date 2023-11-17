// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
pub use aptos_consensus_types::common::Author;
use aptos_consensus_types::common::Round;
use aptos_crypto::{HashValue, bls12381::Signature, CryptoMaterialError, hash::{CryptoHash, CryptoHasher}};
use aptos_dkg::weighted_vuf::traits::WeightedVUF;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage};
use aptos_types::{randomness::{RandConfig, RandDecision, Mode, ProofShare, Delta, RandMetadata, WVUF}, validator_verifier::ValidatorVerifier, validator_signer::ValidatorSigner, aggregate_signature::{PartialSignatures, AggregateSignature}};
use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};


#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RandShare {
    author: Author,
    mode: Mode,
    metadata: RandMetadata,
    share: ProofShare,
}

impl RandShare {
    pub fn new(author: Author, mode: Mode, metadata: RandMetadata, share: ProofShare) -> Self {
        Self { author, mode, metadata, share }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn id(&self) -> HashValue {
        self.metadata.block_id
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch()
    }

    pub fn timestamp(&self) -> u64 {
        self.metadata.timestamp
    }

    pub fn share(&self) -> &ProofShare {
        &self.share
    }
}

impl RandShare {
    pub fn verify(&self, mode: Mode, rand_config: &RandConfig) -> anyhow::Result<()> {
        assert_eq!(self.mode, mode, "[RandShare] Invalid mode");
        let index = *rand_config.validator.address_to_validator_index().get(&self.author).unwrap();
        let maybe_apk = match self.mode {
            Mode::Optimistic => &rand_config.keys_o.certified_apks[index],
            Mode::Fallback => &rand_config.keys_f.certified_apks[index],
        };
        if let Some(apk) = maybe_apk {
            <WVUF as WeightedVUF>::verify_share(&rand_config.vuf_pp, apk, self.metadata.to_bytes().as_slice(), &self.share)?;
        } else {
            bail!("[RandShare] No augmented public key for validator id {}, {}", index, self.author);
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ShareAck {
    pub maybe_decision: Option<RandDecision>,
}

impl ShareAck {
    pub fn new(maybe_decision: Option<RandDecision>) -> Self {
        Self { maybe_decision }
    }
}

#[derive(Serialize)]
struct DeltaMsgWithoutDigest<'a> {
    epoch: u64,
    author: Author,
    delta: &'a Delta,
}

impl<'a> CryptoHash for DeltaMsgWithoutDigest<'a> {
    type Hasher = DeltaMsgHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::new();
        let bytes = bcs::to_bytes(&self).expect("Unable to serialize delta msg");
        state.update(&bytes);
        state.finish()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher, BCSCryptoHash)]
pub struct DeltaMetadata {
    epoch: u64,
    author: Author,
    digest: HashValue,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher, BCSCryptoHash)]
pub struct DeltaMsg {
    metadata: DeltaMetadata,
    delta: Delta,
}

impl DeltaMsg {
    pub fn new(epoch: u64, author: Author, delta: Delta) -> Self {
        let digest = Self::calculate_digest_internal(epoch, author, &delta);
        let metadata = DeltaMetadata { epoch, author, digest };
        Self { metadata, delta }
    }

    fn calculate_digest(&self) -> HashValue {
        Self::calculate_digest_internal(self.metadata.epoch, self.metadata.author, &self.delta)
    }

    fn calculate_digest_internal(
        epoch: u64,
        author: Author,
        delta: &Delta,
    ) -> HashValue {
        let delta_msg_without_digest = DeltaMsgWithoutDigest {
            epoch,
            author,
            delta,
        };
        delta_msg_without_digest.hash()
    }

    pub fn metadata(&self) -> &DeltaMetadata {
        &self.metadata
    }

    pub fn author(&self) -> &Author {
       & self.metadata.author
    }

    pub fn digest(&self) -> HashValue {
        self.metadata.digest
    }

    pub fn delta(&self) -> &Delta {
        &self.delta
    }

    pub fn sign_vote(&self, signer: &ValidatorSigner) -> Result<Signature, CryptoMaterialError> {
        signer.sign(self.metadata())
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        ensure!(self.digest() == self.calculate_digest(), "DeltaMsg: invalid digest");
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeltaAck {
    metadata: DeltaMetadata,
    signature: Signature,
}

impl DeltaAck {
    pub fn new(metadata: DeltaMetadata, signature: Signature) -> Self {
        Self { metadata, signature }
    }

    pub fn metadata(&self) -> &DeltaMetadata {
        &self.metadata
    }

    pub fn verify(&self, author: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(verifier.verify(author, &self.metadata, &self.signature)?)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DeltaCertificate {
    metadata: DeltaMetadata,
    signatures: AggregateSignature,
}

impl DeltaCertificate {
    pub fn new(metadata: DeltaMetadata, signatures: AggregateSignature) -> Self {
        Self { metadata, signatures }
    }

    pub fn signatures(&self) -> &AggregateSignature {
        &self.signatures
    }
    
    // pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
    //     Ok(verifier.verify_multi_signatures(self.metadata(), &self.signatures)?)
    // }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CertifiedDelta {
    delta_msg: DeltaMsg,
    signatures: AggregateSignature,
}

impl CertifiedDelta {
    pub fn new(delta_msg: DeltaMsg, signatures: AggregateSignature) -> Self {
        Self { delta_msg, signatures }
    }

    pub fn metadata(&self) -> &DeltaMetadata {
        &self.delta_msg.metadata
    }

    pub fn author(&self) -> &Author {
        &self.delta_msg.metadata.author
    }

    pub fn delta(&self) -> &Delta {
        &self.delta_msg.delta
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        self.delta_msg.verify()?;

        Ok(verifier.verify_multi_signatures(self.metadata(), &self.signatures)?)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion)]
pub enum RandMessage {
    Share(RandShare),
    ShareAck(ShareAck),
    Delta(DeltaMsg),
    DeltaAck(DeltaAck),
    CertifiedDelta(CertifiedDelta),
    CertifiedDeltaAck(()),
}

impl RandMessage {
    pub fn name(&self) -> &'static str {
        match self {
            RandMessage::Share(_) => "RandMessage::Share",
            RandMessage::ShareAck(_) => "RandMessage::ShareAck",
            RandMessage::Delta(_) => "RandMessage::Delta",
            RandMessage::DeltaAck(_) => "RandMessage::DeltaAck",
            RandMessage::CertifiedDelta(_) => "RandMessage::CertifiedDelta",
            RandMessage::CertifiedDeltaAck(_) => "RandMessage::CertifiedDeltaAck",
        }
    }
}

impl RBMessage for RandMessage {}

pub struct ShareAckState {
    validators: HashSet<Author>,
    rand_config: RandConfig,
    rand_decision_tx: UnboundedSender<RandDecision>,
}

impl ShareAckState {
    pub fn new(validators: impl Iterator<Item = Author>, rand_config: RandConfig, rand_decision_tx: UnboundedSender<RandDecision>) -> Self {
        Self {
            validators: validators.collect(),
            rand_config,
            rand_decision_tx,
        }
    }
}

impl BroadcastStatus<RandMessage> for ShareAckState {
    type Ack = ShareAck;
    type Aggregated = ();
    type Message = RandShare;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        if self.validators.remove(&peer) {
            // If receive a decision, verify it and send it to the randomness manager and stop the reliable broadcast
            if let Some(decision) = ack.maybe_decision {
                match decision.verify(&self.rand_config) {
                    Ok(()) => {
                        let _ = self.rand_decision_tx.unbounded_send(decision);
                        return Ok(Some(()));
                    },
                    Err(e) => {
                        bail!("[RandMessage] Invalid decision from {}: {}", peer, e);
                    },
                }
            }
            // If receive from all validators, stop the reliable broadcast
            if self.validators.is_empty() {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        } else {
            bail!("[RandMessage] Unknown author: {}", peer);
        }
    }
}

pub struct SignatureBuilder {
    metadata: DeltaMetadata,
    partial_signatures: PartialSignatures,
    verifier: Arc<ValidatorVerifier>,
}

impl SignatureBuilder {
    pub fn new(metadata: DeltaMetadata, verifier: Arc<ValidatorVerifier>) -> Self {
        Self {
            metadata,
            partial_signatures: PartialSignatures::empty(),
            verifier,
        }
    }
}

impl BroadcastStatus<RandMessage> for SignatureBuilder {
    type Ack = DeltaAck;
    type Aggregated = DeltaCertificate;
    type Message = DeltaMsg;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(self.metadata == ack.metadata, "Digest mismatch");
        ack.verify(peer, &self.verifier)?;

        self.partial_signatures.add_signature(peer, ack.signature);
        Ok(self
            .verifier
            .check_voting_power(self.partial_signatures.signatures().keys(), true)
            .ok()
            .map(|_| {
                let aggregated_signature = self
                    .verifier
                    .aggregate_signatures(&self.partial_signatures)
                    .expect("Signature aggregation should succeed");
                DeltaCertificate::new(self.metadata.clone(), aggregated_signature)
            }))
    }
}


pub struct CertifiedDeltaAckState {
    validators: HashSet<Author>,
}

impl CertifiedDeltaAckState {
    pub fn new(validators: impl Iterator<Item = Author>) -> Self {
        Self {
            validators: validators.collect()
        }
    }
}

impl BroadcastStatus<RandMessage> for CertifiedDeltaAckState {
    type Ack = ();
    type Aggregated = ();
    type Message = CertifiedDelta;

    fn add(&mut self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        if self.validators.remove(&peer) {
            // If receive from all validators, stop the reliable broadcast
            if self.validators.is_empty() {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        } else {
            bail!("[RandMessage] Unknown author: {}", peer);
        }
    }
}