// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::errors::DAGRpcError;
use crate::{
    dag::observability::{
        logging::{LogEvent, LogSchema},
        tracing::{observe_node, NodeStage},
    },
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
};
use anyhow::{bail, ensure};
use velor_bitvec::BitVec;
use velor_consensus_types::common::{Author, Payload, Round};
use velor_crypto::{
    bls12381::Signature,
    hash::{CryptoHash, CryptoHasher},
    CryptoMaterialError, HashValue,
};
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use velor_enum_conversion_derive::EnumConversion;
use velor_infallible::Mutex;
use velor_logger::debug;
use velor_reliable_broadcast::{BroadcastStatus, RBMessage};
use velor_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    validator_signer::ValidatorSigner,
    validator_txn::ValidatorTransaction,
    validator_verifier::ValidatorVerifier,
};
use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use std::{
    cmp::min,
    collections::HashSet,
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
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
    validator_txns: &'a Vec<ValidatorTransaction>,
    payload: &'a Payload,
    parents: &'a Vec<NodeCertificate>,
    extensions: &'a Extensions,
}

impl CryptoHash for NodeWithoutDigest<'_> {
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
            validator_txns: &node.validator_txns,
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
    validator_txns: Vec<ValidatorTransaction>,
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
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        parents: Vec<NodeCertificate>,
        extensions: Extensions,
    ) -> Self {
        let digest = Self::calculate_digest_internal(
            epoch,
            round,
            author,
            timestamp,
            &validator_txns,
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
            validator_txns,
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
            validator_txns: vec![],
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
        validator_txns: &Vec<ValidatorTransaction>,
        payload: &Payload,
        parents: &Vec<NodeCertificate>,
        extensions: &Extensions,
    ) -> HashValue {
        let node_with_out_digest = NodeWithoutDigest {
            epoch,
            round,
            author,
            timestamp,
            validator_txns,
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
            &self.validator_txns,
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

    pub fn validator_txns(&self) -> &Vec<ValidatorTransaction> {
        &self.validator_txns
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

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, PartialOrd, Ord)]
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
        self.certified_node.verify(verifier)
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
    signature: Signature,
}

impl Vote {
    pub(crate) fn new(metadata: NodeMetadata, signature: Signature) -> Self {
        Self {
            metadata,
            signature,
        }
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn verify(&self, author: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(verifier.verify(author, &self.metadata, self.signature())?)
    }
}

impl From<Vote> for DAGRpcResult {
    fn from(vote: Vote) -> Self {
        DAGRpcResult(Ok(DAGMessage::VoteMsg(vote)))
    }
}

impl TryFrom<DAGRpcResult> for Vote {
    type Error = anyhow::Error;

    fn try_from(result: DAGRpcResult) -> Result<Self, Self::Error> {
        result.0?.try_into()
    }
}

pub struct SignatureBuilder {
    metadata: NodeMetadata,
    inner: Mutex<(PartialSignatures, Option<oneshot::Sender<NodeCertificate>>)>,
    epoch_state: Arc<EpochState>,
}

impl SignatureBuilder {
    pub fn new(
        metadata: NodeMetadata,
        epoch_state: Arc<EpochState>,
        tx: oneshot::Sender<NodeCertificate>,
    ) -> Arc<Self> {
        Arc::new(Self {
            metadata,
            inner: Mutex::new((PartialSignatures::empty(), Some(tx))),
            epoch_state,
        })
    }
}

impl BroadcastStatus<DAGMessage, DAGRpcResult> for Arc<SignatureBuilder> {
    type Aggregated = ();
    type Message = Node;
    type Response = Vote;

    /// Processes the [Vote]s received for a given [Node]. Once a supermajority voting power
    /// is reached, this method sends [NodeCertificate] into a channel. It will only return
    /// successfully when [Vote]s are received from all the peers.
    fn add(&self, peer: Author, ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(self.metadata == ack.metadata, "Digest mismatch");
        ack.verify(peer, &self.epoch_state.verifier)?;
        debug!(LogSchema::new(LogEvent::ReceiveVote)
            .remote_peer(peer)
            .round(self.metadata.round()));
        let mut guard = self.inner.lock();
        let (partial_signatures, tx) = guard.deref_mut();
        partial_signatures.add_signature(peer, ack.signature);

        if tx.is_some()
            && self
                .epoch_state
                .verifier
                .check_voting_power(partial_signatures.signatures().keys(), true)
                .is_ok()
        {
            let aggregated_signature = match self
                .epoch_state
                .verifier
                .aggregate_signatures(partial_signatures.signatures_iter())
            {
                Ok(signature) => signature,
                Err(_) => return Err(anyhow::anyhow!("Signature aggregation failed")),
            };
            observe_node(self.metadata.timestamp(), NodeStage::CertAggregated);
            let certificate = NodeCertificate::new(self.metadata.clone(), aggregated_signature);

            // Invariant Violation: The one-shot channel sender must exist to send the NodeCertificate
            _ = tx
                .take()
                .expect("The one-shot channel sender must exist to send the NodeCertificate")
                .send(certificate);
        }

        if partial_signatures.signatures().len() == self.epoch_state.verifier.len() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

pub struct CertificateAckState {
    num_validators: usize,
    received: Mutex<HashSet<Author>>,
}

impl CertificateAckState {
    pub fn new(num_validators: usize) -> Arc<Self> {
        Arc::new(Self {
            num_validators,
            received: Mutex::new(HashSet::new()),
        })
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

impl From<CertifiedAck> for DAGRpcResult {
    fn from(ack: CertifiedAck) -> Self {
        DAGRpcResult(Ok(DAGMessage::CertifiedAckMsg(ack)))
    }
}

impl TryFrom<DAGRpcResult> for CertifiedAck {
    type Error = anyhow::Error;

    fn try_from(result: DAGRpcResult) -> Result<Self, Self::Error> {
        result.0?.try_into()
    }
}

impl BroadcastStatus<DAGMessage, DAGRpcResult> for Arc<CertificateAckState> {
    type Aggregated = ();
    type Message = CertifiedNodeMessage;
    type Response = CertifiedAck;

    fn add(&self, peer: Author, _ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        debug!(LogSchema::new(LogEvent::ReceiveAck).remote_peer(peer));
        let mut received = self.received.lock();
        received.insert(peer);
        if received.len() == self.num_validators {
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
            self.exists_bitmask.first_round() + self.exists_bitmask.bitmask.len() as u64 - 1
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
    epoch: u64,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl DAGNetworkMessage {
    pub fn new(epoch: u64, data: Vec<u8>) -> Self {
        Self { epoch, data }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
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

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self> {
        match msg {
            ConsensusMsg::DAGMessage(msg) => Ok(bcs::from_bytes(&msg.data)?),
            _ => bail!("unexpected consensus message type {:?}", msg),
        }
    }

    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::DAGMessage(DAGNetworkMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).expect("ConsensusMsg should serialize to bytes"),
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DAGRpcResult(pub Result<DAGMessage, DAGRpcError>);

impl TConsensusMsg for DAGRpcResult {
    fn epoch(&self) -> u64 {
        match &self.0 {
            Ok(dag_message) => dag_message.epoch(),
            Err(error) => error.epoch(),
        }
    }

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self> {
        match msg {
            ConsensusMsg::DAGMessage(msg) => Ok(bcs::from_bytes(&msg.data)?),
            _ => bail!("unexpected consensus message type {:?}", msg),
        }
    }

    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::DAGMessage(DAGNetworkMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).expect("ConsensusMsg should serialize to bytes!"),
        })
    }
}

impl RBMessage for DAGRpcResult {}

impl Deref for DAGRpcResult {
    type Target = Result<DAGMessage, DAGRpcError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Result<DAGMessage, DAGRpcError>> for DAGRpcResult {
    fn from(result: Result<DAGMessage, DAGRpcError>) -> Self {
        Self(result)
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

    pub fn len(&self) -> usize {
        self.bitmask.len()
    }

    pub fn bitvec(&self, round: Round) -> Option<BitVec> {
        let round_idx = round.checked_sub(self.first_round)? as usize;
        self.bitmask.get(round_idx).map(|bitvec| bitvec.into())
    }
}
