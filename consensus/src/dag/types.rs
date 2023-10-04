// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::observability::{
        logging::{LogEvent, LogSchema},
        tracing::{observe_node, NodeStage},
    },
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
};
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
use aptos_logger::debug;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage};
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::min,
    collections::HashSet,
    fmt::{Display, Formatter},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub enum Extensions {
    Empty,
    // Reserved for future extensions such as randomness shares
}

impl Extensions {
    pub fn empty() -> Self {
        Self::Empty
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
    extensions: &'a Extensions,
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
            extensions: &node.extensions,
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

    pub fn timestamp(&self) -> u64 {
        self.timestamp
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
    extensions: Extensions,
}

impl Node {
    pub fn new(
        epoch: u64,
        round: Round,
        author: Author,
        timestamp: u64,
        payload: Payload,
        parents: Vec<NodeCertificate>,
        extensions: Extensions,
    ) -> Self {
        let digest = Self::calculate_digest_internal(
            epoch,
            round,
            author,
            timestamp,
            &payload,
            &parents,
            &extensions,
        );

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
            extensions,
        }
    }

    #[cfg(test)]
    pub fn new_for_test(
        metadata: NodeMetadata,
        payload: Payload,
        parents: Vec<NodeCertificate>,
        extensions: Extensions,
    ) -> Self {
        Self {
            metadata,
            payload,
            parents,
            extensions,
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
        extensions: &Extensions,
    ) -> HashValue {
        let node_with_out_digest = NodeWithoutDigest {
            epoch,
            round,
            author,
            timestamp,
            payload,
            parents,
            extensions,
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
            &self.extensions,
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

    pub fn timestamp(&self) -> u64 {
        self.metadata.timestamp
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

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn verify(&self, sender: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            sender == *self.author(),
            "Author {} doesn't match sender {}",
            self.author(),
            sender
        );
        // TODO: move this check to rpc process logic to delay it as much as possible for performance
        ensure!(self.digest() == self.calculate_digest(), "invalid digest");

        let node_round = self.metadata().round();

        ensure!(node_round > 0, "current round cannot be zero");

        if node_round == 1 {
            ensure!(self.parents().is_empty(), "invalid parents for round 1");
            return Ok(());
        }

        let prev_round = node_round - 1;
        // check if the parents' round is the node's round - 1
        ensure!(
            self.parents()
                .iter()
                .all(|parent| parent.metadata().round() == prev_round),
            "invalid parent round"
        );

        // Verification of the certificate is delayed until we need to fetch it
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

    pub fn author(&self) -> &Author {
        &self.author
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeId: [epoch: {}, round: {}, author: {}]",
            self.epoch, self.round, self.author
        )
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

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(self.digest() == self.calculate_digest(), "invalid digest");

        Ok(verifier.verify_multi_signatures(self.metadata(), self.signatures())?)
    }
}

impl Deref for CertifiedNode {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CertifiedNodeMessage {
    certified_node: CertifiedNode,
    ledger_info: LedgerInfoWithSignatures,
}

impl CertifiedNodeMessage {
    pub fn new(certified_node: CertifiedNode, ledger_info: LedgerInfoWithSignatures) -> Self {
        Self {
            certified_node,
            ledger_info,
        }
    }

    pub fn certified_node(self) -> CertifiedNode {
        self.certified_node
    }

    pub fn ledger_info(&self) -> &LedgerInfoWithSignatures {
        &self.ledger_info
    }

    pub fn verify(&self, sender: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            *self.certified_node.author() == sender,
            "Author {} doesn't match sender {}",
            self.certified_node.author(),
            sender
        );
        ensure!(
            self.certified_node.epoch() == self.ledger_info.commit_info().epoch(),
            "Epoch {} from node doesn't match epoch {} from ledger info",
            self.certified_node.epoch(),
            self.ledger_info().commit_info().epoch()
        );
        self.certified_node.verify(verifier)?;
        if self.ledger_info.commit_info().round() > 0 {
            Ok(self.ledger_info.verify_signatures(verifier)?)
        } else {
            Ok(())
        }
    }
}

impl Deref for CertifiedNodeMessage {
    type Target = CertifiedNode;

    fn deref(&self) -> &Self::Target {
        &self.certified_node
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

    pub fn verify(&self, author: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(verifier.verify(author, &self.metadata, self.signature())?)
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

impl BroadcastStatus<DAGMessage> for SignatureBuilder {
    type Ack = Vote;
    type Aggregated = NodeCertificate;
    type Message = Node;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(self.metadata == ack.metadata, "Digest mismatch");
        ack.verify(peer, &self.epoch_state.verifier)?;
        debug!(LogSchema::new(LogEvent::ReceiveVote)
            .remote_peer(peer)
            .round(self.metadata.round()));
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
                observe_node(self.metadata.timestamp(), NodeStage::CertAggregated);
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

impl BroadcastStatus<DAGMessage> for CertificateAckState {
    type Ack = CertifiedAck;
    type Aggregated = ();
    type Message = CertifiedNodeMessage;

    fn add(&mut self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        debug!(LogSchema::new(LogEvent::ReceiveAck).remote_peer(peer));
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
    pub fn new(epoch: u64, targets: Vec<NodeMetadata>, exists_bitmask: DagSnapshotBitmask) -> Self {
        Self {
            epoch,
            targets,
            exists_bitmask,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn targets(&self) -> impl Iterator<Item = &NodeMetadata> + Clone {
        self.targets.iter()
    }

    pub fn exists_bitmask(&self) -> &DagSnapshotBitmask {
        &self.exists_bitmask
    }

    pub fn start_round(&self) -> Round {
        self.exists_bitmask.first_round()
    }

    pub fn target_round(&self) -> Round {
        self.targets[0].round
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.exists_bitmask
                .bitmask
                .iter()
                .all(|round| round.len() == verifier.len()),
            "invalid bitmask: each round length is not equal to validator count"
        );
        ensure!(!self.targets.is_empty(), "Targets is empty");
        let target_round = self.targets[0].round();
        ensure!(
            self.targets().all(|node| node.round() == target_round),
            "Target round is not consistent"
        );
        ensure!(
            self.exists_bitmask.first_round() + self.exists_bitmask.bitmask.len() as u64
                == target_round,
            "Bitmask length doesn't match, first_round {}, length {}, target {}",
            self.exists_bitmask.first_round(),
            self.exists_bitmask.bitmask.len(),
            target_round
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
        request: &RemoteFetchRequest,
        validator_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<Self> {
        ensure!(
            self.certified_nodes.iter().all(|node| {
                let round = node.round();
                let author = node.author();
                if let Some(author_idx) =
                    validator_verifier.address_to_validator_index().get(author)
                {
                    !request.exists_bitmask.has(round, *author_idx)
                } else {
                    false
                }
            }),
            "nodes don't match requested bitmask"
        );
        ensure!(
            self.certified_nodes
                .iter()
                .all(|node| node.verify(validator_verifier).is_ok()),
            "unable to verify certified nodes"
        );

        Ok(self)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DAGNetworkMessage {
    pub epoch: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl core::fmt::Debug for DAGNetworkMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DAGNetworkMessage")
            .field("epoch", &self.epoch)
            .field("data", &hex::encode(&self.data[..min(20, self.data.len())]))
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion)]
pub enum DAGMessage {
    NodeMsg(Node),
    VoteMsg(Vote),
    CertifiedNodeMsg(CertifiedNodeMessage),
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

    pub fn verify(&self, sender: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            DAGMessage::NodeMsg(node) => node.verify(sender, verifier),
            DAGMessage::CertifiedNodeMsg(certified_node) => certified_node.verify(sender, verifier),
            DAGMessage::FetchRequest(fetch_request) => fetch_request.verify(verifier),
            DAGMessage::VoteMsg(_)
            | DAGMessage::CertifiedAckMsg(_)
            | DAGMessage::FetchResponse(_) => {
                bail!("Unexpected to verify {} in rpc handler", self.name())
            },
            #[cfg(test)]
            DAGMessage::TestMessage(_) | DAGMessage::TestAck(_) => {
                bail!("Unexpected to verify {}", self.name())
            },
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestAck(pub Vec<u8>);

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

    pub fn num_missing(&self) -> usize {
        self.bitmask
            .iter()
            .map(|round| round.iter().map(|exist| !*exist as usize).sum::<usize>())
            .sum::<usize>()
    }

    pub fn first_round(&self) -> Round {
        self.first_round
    }
}
