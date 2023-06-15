// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{dag::reliable_broadcast::BroadcastStatus, network_interface::ConsensusMsg};
use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_crypto::{
    bls12381,
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    epoch_state::EpochState,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, ops::Deref, sync::Arc};

pub trait DAGMessage: Sized + Clone + Serialize + DeserializeOwned {
    fn epoch(&self) -> u64;

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self> {
        match msg {
            ConsensusMsg::DAGMessage(msg) => Ok(bcs::from_bytes(&msg.data)?),
            _ => bail!("unexpected consensus message type in dag"),
        }
    }

    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::DAGMessage(DAGNetworkMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).unwrap(),
        })
    }
}

/// Represents the metadata about the node, without payload and parents from Node
#[derive(Clone, Serialize, Deserialize)]
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
}

/// Node representation in the DAG, parents contain 2f+1 strong links (links to previous round)
#[derive(Clone, Serialize, Deserialize, CryptoHasher)]
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
}

/// Quorum signatures over the node digest
#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeDigestSignature {
    epoch: u64,
    digest: HashValue,
    signature: bls12381::Signature,
}

impl DAGMessage for Node {
    fn epoch(&self) -> u64 {
        self.metadata.epoch
    }
}

impl DAGMessage for NodeDigestSignature {
    fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl DAGMessage for NodeCertificate {
    fn epoch(&self) -> u64 {
        self.epoch
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

#[derive(Serialize, Deserialize, Clone)]
pub struct CertifiedAck {
    epoch: u64,
}

impl DAGMessage for CertifiedAck {
    fn epoch(&self) -> u64 {
        self.epoch
    }
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
