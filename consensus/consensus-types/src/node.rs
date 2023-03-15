// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, Round};
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

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedNodeDigest {
    signed_node_digest_info: SignedNodeDigestInfo,
    peer_id: PeerId,
    signature: bls12381::Signature,
}

impl SignedNodeDigest {
    pub fn new(
        digest: HashValue,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Result<Self, CryptoMaterialError> {
        let info = SignedNodeDigestInfo::new(digest);
        let signature = validator_signer.sign(&info)?;

        Ok(Self {
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

#[allow(dead_code)]
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

    pub fn digest(&self) -> &HashValue {
        &self.signed_node_digest_info.digest
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        validator
            .verify_multi_signatures(&self.signed_node_digest_info, &self.multi_signature)
            .context("Failed to verify ProofOfStore")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct NodeMetaData {
    epoch: u64,
    round: u64,
    source: PeerId,
    digest: HashValue,
}

impl NodeMetaData {
    pub fn new(epoch: u64, round: u64, source: PeerId, digest: HashValue) -> Self {
        Self {
            epoch,
            round,
            source,
            digest,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn source(&self) -> &PeerId {
        &self.source
    }

    pub fn digest(&self) -> HashValue {
        self.digest
    }
}

// TODO: check source in msg.verify()
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    ) -> Self {
        #[derive(Serialize)]
        struct NodeWithoutDigest<'a> {
            epoch: u64,
            round: u64,
            source: PeerId,
            payload: &'a Payload,
            parents: &'a HashSet<NodeMetaData>,
        }

        let node_without_digest = NodeWithoutDigest {
            epoch,
            round,
            source,
            payload: &payload,
            parents: &parents,
        };

        let mut hasher = DefaultHasher::new(b"Node");
        let bytes = bcs::to_bytes(&node_without_digest).unwrap(); // TODO: verify that the data behind the pointer is considered.
        hasher.update(&bytes);
        let metadata = NodeMetaData::new(epoch, round, source, hasher.finish());

        Self {
            metadata,
            consensus_payload: payload,
            parents,
        }
    }

    pub fn verify(&self, validator: &ValidatorVerifier, peer_id: PeerId) -> anyhow::Result<()> {

        TODO: think about this.

        if self.source() != peer_id {
            Err(anyhow::anyhow!(
                "Sender mismatch: peer_id: {}, source: {}",
                self.source(),
                peer_id
            ))
        }

        if self.round() == 0 {
            Ok(())
        } else {
            let strong_parents_peer_id = self.parents.iter().filter(|md| md.round == self.round() - 1).map(|md| md.source);
            validator
                .check_voting_power(strong_parents_peer_id)
                .context("Failed to verify Node")
        }
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

    pub fn source(&self) -> &PeerId {
        self.metadata.source()
    }

    pub fn parents(&self) -> &HashSet<NodeMetaData> {
        &self.parents
    }

    pub fn payload(&self) -> Option<&Payload> {
        Some(&self.consensus_payload)
    }
}

#[allow(dead_code)]
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
        *self.header.source()
    }

    pub fn parents(&self) -> &HashSet<NodeMetaData> {
        &self.header.parents()
    }

    pub fn metadata(&self) -> &NodeMetaData {
        self.header.metadata()
    }
}

// TODO: check peer_id in msg.verify()
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CertifiedNodeAck {
    digest: HashValue,
    peer_id: PeerId,
}

impl CertifiedNodeAck {
    pub fn new(digest: HashValue, peer_id: PeerId) -> Self {
        Self { digest, peer_id }
    }

    pub fn digest(&self) -> HashValue {
        self.digest
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }
}

// TODO: marge with CertifiedNodeAck? Need a good name.
#[allow(dead_code)]
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

    pub fn digest(&self) -> HashValue {
        self.metadata.digest()
    }

    pub fn requester(&self) -> PeerId {
        self.requester
    }

    pub fn source(&self) -> PeerId {
        *self.metadata.source()
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }
}
