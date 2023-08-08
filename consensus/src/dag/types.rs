// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{network::TConsensusMsg, network_interface::ConsensusMsg};
use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_crypto::{
    bls12381,
    bls12381::Signature,
    hash::{CryptoHash, CryptoHasher},
    CryptoMaterialError, HashValue,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage};
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    epoch_state::EpochState,
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, ops::Deref, sync::Arc};

pub trait TDAGMessage: Into<DAGMessage> + TryFrom<DAGMessage> {
    fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()>;
}

impl TDAGMessage for Vote {
    fn verify(&self, _verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        todo!()
    }
}

impl TDAGMessage for CertifiedAck {
    fn verify(&self, _verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Serialize)]
struct NodeWithoutDigest<'a> {
    epoch: u64,
    round: Round,
    author: Author,
    timestamp: u64,
    payload: &'a Payload,
    parents: &'a Vec<NodeCertificate>,
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
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher, BCSCryptoHash)]
pub struct NodeMetadata {
    node_id: NodeId,
    timestamp: u64,
    digest: HashValue,
}

impl NodeMetadata {
    #[cfg(test)]
    pub fn new_for_test(
        epoch: u64,
        round: Round,
        author: Author,
        timestamp: u64,
        digest: HashValue,
    ) -> Self {
        Self {
            node_id: NodeId {
                epoch,
                round,
                author,
            },
            timestamp,
            digest,
        }
    }

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

impl Deref for NodeMetadata {
    type Target = NodeId;

    fn deref(&self) -> &Self::Target {
        &self.node_id
    }
}

/// Node representation in the DAG, parents contain 2f+1 strong links (links to previous round)
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct Node {
    metadata: NodeMetadata,
    payload: Payload,
    parents: Vec<NodeCertificate>,
}

impl Node {
    pub fn new(
        epoch: u64,
        round: Round,
        author: Author,
        timestamp: u64,
        payload: Payload,
        parents: Vec<NodeCertificate>,
    ) -> Self {
        let digest =
            Self::calculate_digest_internal(epoch, round, author, timestamp, &payload, &parents);

        Self {
            metadata: NodeMetadata {
                node_id: NodeId {
                    epoch,
                    round,
                    author,
                },
                timestamp,
                digest,
            },
            payload,
            parents,
        }
    }

    #[cfg(test)]
    pub fn new_for_test(
        metadata: NodeMetadata,
        payload: Payload,
        parents: Vec<NodeCertificate>,
    ) -> Self {
        Self {
            metadata,
            payload,
            parents,
        }
    }

    /// Calculate the node digest based on all fields in the node
    fn calculate_digest_internal(
        epoch: u64,
        round: Round,
        author: Author,
        timestamp: u64,
        payload: &Payload,
        parents: &Vec<NodeCertificate>,
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

    fn calculate_digest(&self) -> HashValue {
        Self::calculate_digest_internal(
            self.metadata.epoch,
            self.metadata.round,
            self.metadata.author,
            self.metadata.timestamp,
            &self.payload,
            &self.parents,
        )
    }

    pub fn digest(&self) -> HashValue {
        self.metadata.digest
    }

    pub fn metadata(&self) -> &NodeMetadata {
        &self.metadata
    }

    pub fn parents(&self) -> &[NodeCertificate] {
        &self.parents
    }

    pub fn parents_metadata(&self) -> impl Iterator<Item = &NodeMetadata> {
        self.parents().iter().map(|cert| &cert.metadata)
    }

    pub fn author(&self) -> &Author {
        self.metadata.author()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn id(&self) -> NodeId {
        NodeId::new(self.epoch(), self.round(), *self.author())
    }

    pub fn sign_vote(&self, signer: &ValidatorSigner) -> Result<Signature, CryptoMaterialError> {
        signer.sign(self.metadata())
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }
}

impl TDAGMessage for Node {
    fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        // TODO: move this check to rpc process logic to delay it as much as possible for performance
        ensure!(self.digest() == self.calculate_digest(), "invalid digest");

        let current_round = self.metadata().round();

        if current_round == 0 {
            ensure!(self.parents().is_empty(), "invalid parents for round 0");
            return Ok(());
        }

        let prev_round = current_round - 1;
        // check if the parents' round is the node's round - 1
        ensure!(
            self.parents()
                .iter()
                .all(|parent| parent.metadata().round() == prev_round),
            "invalid parent round"
        );

        ensure!(
            verifier
                .check_voting_power(
                    self.parents()
                        .iter()
                        .map(|parent| parent.metadata().author()),
                    true,
                )
                .is_ok(),
            "not enough parents to satisfy voting power"
        );

        // TODO: validate timestamp

        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone)]
pub struct NodeId {
    epoch: u64,
    round: Round,
    author: Author,
}

impl NodeId {
    pub fn new(epoch: u64, round: Round, author: Author) -> Self {
        Self {
            epoch,
            round,
            author,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn author(&self) -> Author {
        self.author
    }
}

/// Quorum signatures over the node digest
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct NodeCertificate {
    metadata: NodeMetadata,
    signatures: AggregateSignature,
}

impl NodeCertificate {
    pub fn new(metadata: NodeMetadata, signatures: AggregateSignature) -> Self {
        Self {
            metadata,
            signatures,
        }
    }

    pub fn metadata(&self) -> &NodeMetadata {
        &self.metadata
    }

    pub fn signers(&self, validators: &[Author]) -> Vec<Author> {
        self.signatures.get_signers_addresses(validators)
    }

    pub fn signatures(&self) -> &AggregateSignature {
        &self.signatures
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(verifier.verify_multi_signatures(self.metadata(), self.signatures())?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CertifiedNode {
    node: Node,
    signatures: AggregateSignature,
}

impl CertifiedNode {
    pub fn new(node: Node, signatures: AggregateSignature) -> Self {
        Self { node, signatures }
    }

    pub fn signatures(&self) -> &AggregateSignature {
        &self.signatures
    }

    pub fn certificate(&self) -> NodeCertificate {
        NodeCertificate::new(self.node.metadata.clone(), self.signatures.clone())
    }
}

impl Deref for CertifiedNode {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl TDAGMessage for CertifiedNode {
    fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(self.digest() == self.calculate_digest(), "invalid digest");

        verifier
            .verify_multi_signatures(self.metadata(), self.certificate().signatures())
            .map_err(|e| anyhow::anyhow!("unable to verify: {}", e))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Vote {
    metadata: NodeMetadata,
    signature: bls12381::Signature,
}

impl Vote {
    pub(crate) fn new(metadata: NodeMetadata, signature: Signature) -> Self {
        Self {
            metadata,
            signature,
        }
    }

    pub fn signature(&self) -> &bls12381::Signature {
        &self.signature
    }
}

pub struct SignatureBuilder {
    metadata: NodeMetadata,
    partial_signatures: PartialSignatures,
    epoch_state: Arc<EpochState>,
}

impl SignatureBuilder {
    pub fn new(metadata: NodeMetadata, epoch_state: Arc<EpochState>) -> Self {
        Self {
            metadata,
            partial_signatures: PartialSignatures::empty(),
            epoch_state,
        }
    }
}

impl<M> BroadcastStatus<M> for SignatureBuilder
where
    M: RBMessage,
    Vote: TryFrom<M> + Into<M>,
    Node: TryFrom<M> + Into<M>,
{
    type Ack = Vote;
    type Aggregated = NodeCertificate;
    type Message = Node;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(self.metadata == ack.metadata, "Digest mismatch");
        self.partial_signatures.add_signature(peer, ack.signature);
        Ok(self
            .epoch_state
            .verifier
            .check_voting_power(self.partial_signatures.signatures().keys(), true)
            .ok()
            .map(|_| {
                let aggregated_signature = self
                    .epoch_state
                    .verifier
                    .aggregate_signatures(&self.partial_signatures)
                    .expect("Signature aggregation should succeed");
                NodeCertificate::new(self.metadata.clone(), aggregated_signature)
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CertifiedAck {
    epoch: u64,
}

impl CertifiedAck {
    pub fn new(epoch: u64) -> Self {
        Self { epoch }
    }
}

impl<M> BroadcastStatus<M> for CertificateAckState
where
    M: RBMessage,
    CertifiedAck: TryFrom<M> + Into<M>,
    CertifiedNode: TryFrom<M> + Into<M>,
{
    type Ack = CertifiedAck;
    type Aggregated = ();
    type Message = CertifiedNode;

    fn add(&mut self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        self.received.insert(peer);
        if self.received.len() == self.num_validators {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

/// Represents a request to fetch missing dependencies for `target`, `start_round` represents
/// the first round we care about in the DAG, `exists_bitmask` is a two dimensional bitmask represents
/// if a node exist at [start_round + index][validator_index].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoteFetchRequest {
    epoch: u64,
    targets: Vec<NodeMetadata>,
    exists_bitmask: DagSnapshotBitmask,
}

impl RemoteFetchRequest {
    pub fn new(epoch: u64, parents: Vec<NodeMetadata>, exists_bitmask: DagSnapshotBitmask) -> Self {
        Self {
            epoch,
            targets: parents,
            exists_bitmask,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn targets(&self) -> &[NodeMetadata] {
        &self.targets
    }

    pub fn exists_bitmask(&self) -> &DagSnapshotBitmask {
        &self.exists_bitmask
    }
}

impl TDAGMessage for RemoteFetchRequest {
    fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.exists_bitmask
                .bitmask
                .iter()
                .all(|round| round.len() == verifier.len()),
            "invalid bitmask: each round length is not equal to validator count"
        );

        Ok(())
    }
}

/// Represents a response to FetchRequest, `certified_nodes` are indexed by [round][validator_index]
/// It should fill in gaps from the `exists_bitmask` according to the parents from the `target_digest` node.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FetchResponse {
    epoch: u64,
    certified_nodes: Vec<CertifiedNode>,
}

impl FetchResponse {
    pub fn new(epoch: u64, certified_nodes: Vec<CertifiedNode>) -> Self {
        Self {
            epoch,
            certified_nodes,
        }
    }

    pub fn certified_nodes(self) -> Vec<CertifiedNode> {
        self.certified_nodes
    }

    pub fn verify(
        self,
        _request: &RemoteFetchRequest,
        validator_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<Self> {
        ensure!(
            self.certified_nodes
                .iter()
                .all(|node| node.verify(validator_verifier).is_ok()),
            "unable to verify certified nodes"
        );

        Ok(self)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DAGNetworkMessage {
    pub epoch: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion)]
pub enum DAGMessage {
    NodeMsg(Node),
    VoteMsg(Vote),
    CertifiedNodeMsg(CertifiedNode),
    CertifiedAckMsg(CertifiedAck),
    FetchRequest(RemoteFetchRequest),
    FetchResponse(FetchResponse),

    #[cfg(test)]
    TestMessage(TestMessage),
    #[cfg(test)]
    TestAck(TestAck),
}

impl DAGMessage {
    pub fn name(&self) -> &str {
        match self {
            DAGMessage::NodeMsg(_) => "NodeMsg",
            DAGMessage::VoteMsg(_) => "VoteMsg",
            DAGMessage::CertifiedNodeMsg(_) => "CertifiedNodeMsg",
            DAGMessage::CertifiedAckMsg(_) => "CertifiedAckMsg",
            DAGMessage::FetchRequest(_) => "FetchRequest",
            DAGMessage::FetchResponse(_) => "FetchResponse",
            #[cfg(test)]
            DAGMessage::TestMessage(_) => "TestMessage",
            #[cfg(test)]
            DAGMessage::TestAck(_) => "TestAck",
        }
    }

    pub fn author(&self) -> anyhow::Result<Author> {
        match self {
            DAGMessage::NodeMsg(node) => Ok(node.metadata.author),
            DAGMessage::CertifiedNodeMsg(node) => Ok(node.metadata.author),
            _ => bail!("message does not support author field"),
        }
    }
}

impl RBMessage for DAGMessage {}

impl TConsensusMsg for DAGMessage {
    fn epoch(&self) -> u64 {
        match self {
            DAGMessage::NodeMsg(node) => node.metadata.epoch,
            DAGMessage::VoteMsg(vote) => vote.metadata.epoch,
            DAGMessage::CertifiedNodeMsg(node) => node.metadata.epoch,
            DAGMessage::CertifiedAckMsg(ack) => ack.epoch,
            DAGMessage::FetchRequest(req) => req.epoch,
            DAGMessage::FetchResponse(res) => res.epoch,
            #[cfg(test)]
            DAGMessage::TestMessage(_) => 1,
            #[cfg(test)]
            DAGMessage::TestAck(_) => 1,
        }
    }

    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::DAGMessage(DAGNetworkMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).unwrap(),
        })
    }
}

impl TryFrom<DAGNetworkMessage> for DAGMessage {
    type Error = anyhow::Error;

    fn try_from(msg: DAGNetworkMessage) -> Result<Self, Self::Error> {
        Ok(bcs::from_bytes(&msg.data)?)
    }
}

impl TryFrom<ConsensusMsg> for DAGMessage {
    type Error = anyhow::Error;

    fn try_from(msg: ConsensusMsg) -> Result<Self, Self::Error> {
        TConsensusMsg::from_network_message(msg)
    }
}

#[cfg(test)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TestMessage(pub Vec<u8>);

#[cfg(test)]
impl TDAGMessage for TestMessage {
    fn verify(&self, _verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestAck(pub Vec<u8>);

#[cfg(test)]
impl TDAGMessage for TestAck {
    fn verify(&self, _verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DagSnapshotBitmask {
    bitmask: Vec<Vec<bool>>,
    first_round: Round,
}

impl DagSnapshotBitmask {
    pub fn new(first_round: Round, bitmask: Vec<Vec<bool>>) -> Self {
        Self {
            bitmask,
            first_round,
        }
    }

    pub fn has(&self, round: Round, author_idx: usize) -> bool {
        let round_idx = match round.checked_sub(self.first_round) {
            Some(idx) => idx as usize,
            None => return false,
        };
        self.bitmask
            .get(round_idx)
            .and_then(|round| round.get(author_idx).cloned())
            .unwrap_or(false)
    }

    pub fn first_round(&self) -> Round {
        self.first_round
    }
}
