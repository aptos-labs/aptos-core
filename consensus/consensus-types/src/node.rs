// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, Round, Author};
use anyhow::Context;
use aptos_crypto::{bls12381, hash::DefaultHasher, CryptoMaterialError, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug)]
pub enum SignedNodeDigestError {
    WrongDigest,
    DuplicatedSignature,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash, PartialEq, Eq, Hash,
)]
pub struct SignedNodeDigestInfo {
    digest: HashValue,
}

impl SignedNodeDigestInfo {
    pub fn new(digest: HashValue) -> Self {
        Self { digest }
    }

    pub fn digest(&self) -> HashValue {
        self.digest
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedNodeDigest {
    epoch: u64,
    signed_node_digest_info: SignedNodeDigestInfo,
    peer_id: PeerId,
    signature: bls12381::Signature,
}

impl SignedNodeDigest {
    pub fn new(
        epoch: u64,
        digest: HashValue,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Result<Self, CryptoMaterialError> {
        let info = SignedNodeDigestInfo::new(digest);
        let signature = validator_signer.sign(&info)?;

        Ok(Self {
            epoch,
            signed_node_digest_info: SignedNodeDigestInfo::new(digest),
            peer_id: validator_signer.author(),
            signature,
        })
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(validator.verify(self.peer_id, &self.signed_node_digest_info, &self.signature)?)
    }

    pub fn digest(&self) -> HashValue {
        self.signed_node_digest_info.digest
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn info(&self) -> &SignedNodeDigestInfo {
        &self.signed_node_digest_info
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn signature(self) -> bls12381::Signature {
        self.signature
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NodeCertificate {
    signed_node_digest_info: SignedNodeDigestInfo,
    multi_signature: AggregateSignature,
}

impl NodeCertificate {
    pub fn new(
        signed_node_digest_info: SignedNodeDigestInfo,
        multi_signature: AggregateSignature,
    ) -> Self {
        Self {
            signed_node_digest_info,
            multi_signature,
        }
    }

    pub fn digest(&self) -> HashValue {
        self.signed_node_digest_info.digest
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        validator
            .verify_multi_signatures(&self.signed_node_digest_info, &self.multi_signature)
            .context("Failed to verify NodeCertificate")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct NodeMetaData {
    epoch: u64,
    round: u64,
    source: PeerId,
    digest: HashValue,
    timestamp: u64, // TODO: maybe move it out to save space on the network.
}

impl NodeMetaData {
    pub fn new(epoch: u64, round: u64, source: PeerId, digest: HashValue, timestamp: u64) -> Self {
        Self {
            epoch,
            round,
            source,
            digest,
            timestamp,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn source(&self) -> PeerId {
        self.source
    }

    pub fn source_ref(&self) -> &PeerId {
        &self.source
    }

    pub fn digest(&self) -> HashValue {
        self.digest
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

fn compute_node_digest(
    epoch: u64,
    round: u64,
    source: PeerId,
    timestamp: u64,
    payload: &Payload,
    parents: &HashSet<NodeMetaData>,
) -> HashValue {
    #[derive(Serialize)]
    // struct NodeWithoutDigest<'a> {
    struct NodeWithoutDigest {
        epoch: u64,
        round: u64,
        source: PeerId,
        timestamp: u64,
        // payload: &'a Payload, TODO: fix this.
        // parents: &'a HashSet<NodeMetaData>,
    }

    let node_without_digest = NodeWithoutDigest {
        epoch,
        round,
        source,
        timestamp,
        // payload,
        // parents,
    };

    let mut hasher = DefaultHasher::new(b"Node");
    let bytes = bcs::to_bytes(&node_without_digest).unwrap(); // TODO: verify that the data behind the pointer is considered.
    hasher.update(&bytes);
    hasher.finish()
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Node {
    metadata: NodeMetaData,
    consensus_payload: Payload,
    parents: HashSet<NodeMetaData>,
}

impl Node {
    pub fn new(
        epoch: u64,
        round: u64,
        source: PeerId,
        payload: Payload,
        parents: HashSet<NodeMetaData>,
        timestamp: u64,
    ) -> Self {
        let digest = compute_node_digest(epoch, round, source, timestamp, &payload, &parents);
        let metadata = NodeMetaData::new(epoch, round, source, digest, timestamp);

        Self {
            metadata,
            consensus_payload: payload,
            parents,
        }
    }

    pub fn strong_links(&self) -> HashSet<PeerId> {
        self.parents
            .iter()
            .filter(|n| n.round == self.round() - 1)
            .map(|n| n.source)
            .collect()
    }

    pub fn verify_digest(&self) -> bool {
        let digest = compute_node_digest(
            self.epoch(),
            self.round(),
            self.source(),
            self.timestamp(),
            &self.consensus_payload,
            &self.parents,
        );
        self.digest() == digest
    }

    pub fn verify(&self, validator: &ValidatorVerifier, peer_id: PeerId) -> anyhow::Result<()> {
        // Insuring authentication
        if self.source() != peer_id {
            return Err(anyhow::anyhow!(
                "Failed to verify Node due to sender mismatch: self: {}, network: {}",
                self.source(),
                peer_id
            ));
        }

        // Node must point to 2/3 stake in previous round
        if self.round() > 0 {
            let strong_parents_peer_id = self
                .parents
                .iter()
                .filter(|md| md.round == self.round() - 1)
                .map(|md| &md.source);
            if validator
                .check_voting_power(strong_parents_peer_id)
                .is_err()
            {
                return Err(anyhow::anyhow!(
                    "Failed to verify Node due to not enough strong links"
                ));
            }
        }

        // Digest must match the node
        if !self.verify_digest() {
            return Err(anyhow::anyhow!("Failed to verify Node due to wrong digest"));
        }

        Ok(())
    }

    pub fn digest(&self) -> HashValue {
        self.metadata.digest()
    }

    pub fn metadata(&self) -> &NodeMetaData {
        &self.metadata
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch()
    }

    pub fn round(&self) -> u64 {
        self.metadata.round
    }

    pub fn source(&self) -> PeerId {
        self.metadata.source()
    }

    pub fn source_ref(&self) -> &PeerId {
        &self.metadata.source_ref()
    }

    pub fn parents(&self) -> &HashSet<NodeMetaData> {
        &self.parents
    }

    pub fn maybe_payload(&self) -> Option<&Payload> {
        Some(&self.consensus_payload)
    }

    pub fn take_payload(self) -> Payload {
        self.consensus_payload
    }

    pub fn timestamp(&self) -> u64 {
        self.metadata.timestamp
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CertifiedNode {
    header: Node,
    certificate: NodeCertificate,
}

impl CertifiedNode {
    pub fn new(header: Node, certificate: NodeCertificate) -> Self {
        Self {
            header,
            certificate,
        }
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        if !self.header.verify_digest() {
            return Err(anyhow::anyhow!(
                "Failed to verify CertifiedNode due to wrong Node digest"
            ));
        }

        if self.header.digest() != self.certificate.digest() {
            return Err(anyhow::anyhow!(
                "Failed to verify CertifiedNode due to digest mismatch between node and certificate"
            ));
        }

        self.certificate.verify(validator)
    }

    pub fn take_node(self) -> Node {
        self.header
    }

    pub fn node(&self) -> &Node {
        &self.header
    }

    pub fn digest(&self) -> HashValue {
        self.header.digest()
    }

    pub fn epoch(&self) -> u64 {
        self.header.epoch()
    }

    pub fn round(&self) -> u64 {
        self.header.round()
    }

    pub fn source(&self) -> PeerId {
        self.header.source()
    }

    pub fn parents(&self) -> &HashSet<NodeMetaData> {
        &self.header.parents()
    }

    pub fn metadata(&self) -> &NodeMetaData {
        self.header.metadata()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CertifiedNodeAck {
    epoch: u64,
    digest: HashValue,
    peer_id: PeerId,
}

impl CertifiedNodeAck {
    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if peer_id != self.peer_id {
            return Err(anyhow::anyhow!(
                "Failed to verify CertifiedNodeAck due to wrong peer_id: self {}, network {}",
                self.peer_id,
                peer_id,
            ));
        }

        Ok(())
    }

    pub fn new(epoch: u64, digest: HashValue, peer_id: PeerId) -> Self {
        Self {
            epoch,
            digest,
            peer_id,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn digest(&self) -> HashValue {
        self.digest
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CertifiedNodeRequest {
    metadata: NodeMetaData,
    requester: PeerId,
}

impl CertifiedNodeRequest {
    pub fn new(metadata: NodeMetaData, requester: PeerId) -> Self {
        Self {
            metadata,
            requester,
        }
    }

    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if peer_id != self.requester {
            return Err(anyhow::anyhow!(
                "Failed to verify CertifiedNodeRequest due to wrong peer_id: self {}, network {}",
                self.requester,
                peer_id,
            ));
        }

        Ok(())
    }

    pub fn digest(&self) -> HashValue {
        self.metadata.digest()
    }

    pub fn requester(&self) -> PeerId {
        self.requester
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch()
    }

    pub fn source(&self) -> PeerId {
        self.metadata.source()
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }
}
