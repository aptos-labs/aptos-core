// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{dag::reliable_broadcast::BroadcastStatus, network::ConsensusMessageTrait};
use anyhow::ensure;
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_crypto::{
    bls12381,
    bls12381::Signature,
    hash::{CryptoHash, CryptoHasher},
    CryptoMaterialError, HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    epoch_state::EpochState,
    validator_signer::ValidatorSigner,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, ops::Deref, sync::Arc};

#[derive(Serialize)]
struct NodeWithoutDigest<'a> {
    epoch: u64,
    round: Round,
    author: Author,
    timestamp: u64,
    payload: &'a Payload,
    parents: &'a Vec<NodeMetadata>,
}

impl<'a> CryptoHash for NodeWithoutDigest<'a> {
    type Hasher = NodeHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::new();
        let bytes = bcs::to_bytes(&self).expect("Unable to serialize node");
        state.update(&bytes);
        state.finish()
    }
}

impl<'a> From<&'a Node> for NodeWithoutDigest<'a> {
    fn from(node: &'a Node) -> Self {
        Self {
            epoch: node.metadata.epoch,
            round: node.metadata.round,
            author: node.metadata.author,
            timestamp: node.metadata.timestamp,
            payload: &node.payload,
            parents: &node.parents,
        }
    }
}

/// Represents the metadata about the node, without payload and parents from Node
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NodeMetadata {
    epoch: u64,
    round: Round,
    author: Author,
    timestamp: u64,
    digest: HashValue,
}

impl NodeMetadata {
    pub fn digest(&self) -> &HashValue {
        &self.digest
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// Node representation in the DAG, parents contain 2f+1 strong links (links to previous round)
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug)]
pub struct Node {
    metadata: NodeMetadata,
    payload: Payload,
    parents: Vec<NodeMetadata>,
}

impl Node {
    pub fn new(
        epoch: u64,
        round: Round,
        author: Author,
        timestamp: u64,
        payload: Payload,
        parents: Vec<NodeMetadata>,
    ) -> Self {
        let digest = Self::calculate_digest(epoch, round, author, timestamp, &payload, &parents);

        Self {
            metadata: NodeMetadata {
                epoch,
                round,
                author,
                timestamp,
                digest,
            },
            payload,
            parents,
        }
    }

    /// Calculate the node digest based on all fields in the node
    fn calculate_digest(
        epoch: u64,
        round: Round,
        author: Author,
        timestamp: u64,
        payload: &Payload,
        parents: &Vec<NodeMetadata>,
    ) -> HashValue {
        let node_with_out_digest = NodeWithoutDigest {
            epoch,
            round,
            author,
            timestamp,
            payload,
            parents,
        };
        node_with_out_digest.hash()
    }

    pub fn digest(&self) -> HashValue {
        self.metadata.digest
    }

    pub fn metadata(&self) -> &NodeMetadata {
        &self.metadata
    }

    pub fn parents(&self) -> &[NodeMetadata] {
        &self.parents
    }

    pub fn sign(&self, signer: &ValidatorSigner) -> Result<Signature, CryptoMaterialError> {
        let node_without_digest: NodeWithoutDigest = self.into();
        signer.sign(&node_without_digest)
    }
}

/// Quorum signatures over the node digest
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NodeCertificate {
    epoch: u64,
    digest: HashValue,
    signatures: AggregateSignature,
}

impl NodeCertificate {
    pub fn new(epoch: u64, digest: HashValue, signatures: AggregateSignature) -> Self {
        Self {
            epoch,
            digest,
            signatures,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertifiedNode {
    node: Node,
    certificate: NodeCertificate,
}

impl CertifiedNode {
    pub fn new(node: Node, certificate: NodeCertificate) -> Self {
        Self { node, certificate }
    }
}

impl Deref for CertifiedNode {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NodeDigestSignature {
    epoch: u64,
    digest: HashValue,
    signature: bls12381::Signature,
}

impl NodeDigestSignature {
    pub(crate) fn new(epoch: u64, digest: HashValue, signature: Signature) -> Self {
        Self {
            epoch,
            digest,
            signature,
        }
    }
}

pub struct SignatureBuilder {
    digest: HashValue,
    partial_signatures: PartialSignatures,
    epoch_state: Arc<EpochState>,
}

impl SignatureBuilder {
    pub fn new(digest: HashValue, epoch_state: Arc<EpochState>) -> Self {
        Self {
            digest,
            partial_signatures: PartialSignatures::empty(),
            epoch_state,
        }
    }
}

impl BroadcastStatus for SignatureBuilder {
    type Ack = NodeDigestSignature;
    type Aggregated = NodeCertificate;
    type Message = Node;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(self.digest == ack.digest, "Digest mismatch");
        self.partial_signatures.add_signature(peer, ack.signature);
        Ok(self
            .epoch_state
            .verifier
            .check_voting_power(self.partial_signatures.signatures().keys())
            .ok()
            .map(|_| {
                let aggregated_signature = self
                    .epoch_state
                    .verifier
                    .aggregate_signatures(&self.partial_signatures)
                    .expect("Signature aggregation should succeed");
                NodeCertificate {
                    epoch: self.epoch_state.epoch,
                    digest: self.digest,
                    signatures: aggregated_signature,
                }
            }))
    }
}

pub struct CertificateAckState {
    num_validators: usize,
    received: HashSet<Author>,
}

impl CertificateAckState {
    pub fn new(num_validators: usize) -> Self {
        Self {
            num_validators,
            received: HashSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CertifiedAck {
    epoch: u64,
}

impl BroadcastStatus for CertificateAckState {
    type Ack = CertifiedAck;
    type Aggregated = ();
    type Message = NodeCertificate;

    fn add(&mut self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        self.received.insert(peer);
        if self.received.len() == self.num_validators {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DAGNetworkMessage {
    pub epoch: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DAGMessage {
    NodeMsg(Node),
    NodeDigestSignatureMsg(NodeDigestSignature),
    NodeCertificateMsg(NodeCertificate),
    CertifiedAckMsg(CertifiedAck),

    #[cfg(test)]
    TestMessage(TestMessage),
    #[cfg(test)]
    TestAck(TestAck),
}

impl DAGMessage {
    pub fn name(&self) -> &str {
        match self {
            DAGMessage::NodeMsg(_) => "NodeMsg",
            DAGMessage::NodeDigestSignatureMsg(_) => "NodeDigestSignatureMsg",
            DAGMessage::NodeCertificateMsg(_) => "NodeCertificateMsg",
            DAGMessage::CertifiedAckMsg(_) => "CertifiedAckMsg",
            #[cfg(test)]
            DAGMessage::TestMessage(_) => "TestMessage",
            #[cfg(test)]
            DAGMessage::TestAck(_) => "TestAck",
        }
    }
}

impl ConsensusMessageTrait for DAGMessage {
    fn epoch(&self) -> u64 {
        match self {
            DAGMessage::NodeMsg(node) => node.metadata.epoch,
            DAGMessage::NodeDigestSignatureMsg(signature) => signature.epoch,
            DAGMessage::NodeCertificateMsg(certificate) => certificate.epoch,
            DAGMessage::CertifiedAckMsg(ack) => ack.epoch,
            #[cfg(test)]
            DAGMessage::TestMessage(_) => 1,
            #[cfg(test)]
            DAGMessage::TestAck(_) => 1,
        }
    }
}

impl TryFrom<DAGMessage> for Node {
    type Error = anyhow::Error;

    fn try_from(msg: DAGMessage) -> Result<Self, Self::Error> {
        match msg {
            DAGMessage::NodeMsg(node) => Ok(node),
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

impl TryFrom<DAGMessage> for NodeDigestSignature {
    type Error = anyhow::Error;

    fn try_from(msg: DAGMessage) -> Result<Self, Self::Error> {
        match msg {
            DAGMessage::NodeDigestSignatureMsg(node) => Ok(node),
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

impl TryFrom<DAGMessage> for NodeCertificate {
    type Error = anyhow::Error;

    fn try_from(msg: DAGMessage) -> Result<Self, Self::Error> {
        match msg {
            DAGMessage::NodeCertificateMsg(certificate) => Ok(certificate),
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

impl TryFrom<DAGMessage> for CertifiedAck {
    type Error = anyhow::Error;

    fn try_from(msg: DAGMessage) -> Result<Self, Self::Error> {
        match msg {
            DAGMessage::CertifiedAckMsg(ack) => Ok(ack),
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

impl From<Node> for DAGMessage {
    fn from(node: Node) -> Self {
        Self::NodeMsg(node)
    }
}

impl From<NodeDigestSignature> for DAGMessage {
    fn from(signature: NodeDigestSignature) -> Self {
        Self::NodeDigestSignatureMsg(signature)
    }
}

impl From<NodeCertificate> for DAGMessage {
    fn from(node: NodeCertificate) -> Self {
        Self::NodeCertificateMsg(node)
    }
}

impl From<CertifiedAck> for DAGMessage {
    fn from(ack: CertifiedAck) -> Self {
        Self::CertifiedAckMsg(ack)
    }
}

pub trait DAGMessageTrait: Into<DAGMessage> + TryFrom<DAGMessage> {}

impl DAGMessageTrait for Node {}
impl DAGMessageTrait for NodeDigestSignature {}
impl DAGMessageTrait for NodeCertificate {}
impl DAGMessageTrait for CertifiedAck {}

#[cfg(test)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TestMessage(pub Vec<u8>);

#[cfg(test)]
impl From<TestMessage> for DAGMessage {
    fn from(msg: TestMessage) -> DAGMessage {
        DAGMessage::TestMessage(msg)
    }
}

#[cfg(test)]
impl TryFrom<DAGMessage> for TestMessage {
    type Error = anyhow::Error;

    fn try_from(msg: DAGMessage) -> Result<Self, Self::Error> {
        match msg {
            DAGMessage::TestMessage(ack) => Ok(ack),
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

#[cfg(test)]
impl DAGMessageTrait for TestMessage {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TestAck(pub Vec<u8>);

#[cfg(test)]
impl From<TestAck> for DAGMessage {
    fn from(ack: TestAck) -> DAGMessage {
        DAGMessage::TestAck(ack)
    }
}

#[cfg(test)]
impl TryFrom<DAGMessage> for TestAck {
    type Error = anyhow::Error;

    fn try_from(msg: DAGMessage) -> Result<Self, Self::Error> {
        match msg {
            DAGMessage::TestAck(ack) => Ok(ack),
            _ => Err(anyhow::anyhow!("invalid message type")),
        }
    }
}

#[cfg(test)]
impl DAGMessageTrait for TestAck {}
