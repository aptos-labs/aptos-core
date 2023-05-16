// Copyright © Aptos Foundation

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::node::{CertifiedNode, CertifiedNodeAck, NodeCertificate, NodeMetaData, SignedNodeDigest, SignedNodeDigestError, SignedNodeDigestInfo};
use aptos_crypto::{bls12381, HashValue};
use aptos_types::{
    aggregate_signature::PartialSignatures, PeerId, validator_verifier::ValidatorVerifier,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use serde::{Deserialize, Serialize};
use aptos_logger::info;
use aptos_schemadb::schema::{KeyCodec, ValueCodec};
use aptos_types::block_info::Round;
use std::io::{Cursor, Write};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use peer_index_map::PeerIndexMap;
use peer_status_list::{PeerStatusList, PeerStatusListItem};
use crate::dag::dag_storage::{DagStoreWriteBatch, ItemId};
use crate::dag::types::dag_round_list::DagRoundList;
// pub(crate) trait MissingPeers {
//     fn get_peers_signatures() -> HashSet<PeerId>;
// }

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct IncrementalNodeCertificateState {
    signed_node_digest_info: SignedNodeDigestInfo,
    aggregated_signature: BTreeMap<PeerId, bls12381::Signature>,
}

#[allow(dead_code)]
impl IncrementalNodeCertificateState {
    pub fn new(digest: HashValue) -> Self {
        Self {
            signed_node_digest_info: SignedNodeDigestInfo::new(digest),
            aggregated_signature: BTreeMap::new(),
        }
    }

    pub(crate) fn missing_peers_signatures(
        &self,
        validator_verifier: &ValidatorVerifier,
    ) -> Vec<PeerId> {
        let all_peers: HashSet<&PeerId> = validator_verifier
            .address_to_validator_index()
            .keys()
            .collect();
        let singers: HashSet<&PeerId> = self.aggregated_signature.keys().collect();
        all_peers.difference(&singers).cloned().cloned().collect()
    }

    //Signature we already verified
    pub(crate) fn add_signature(
        &mut self,
        signed_node_digest: SignedNodeDigest,
    ) -> Result<(), SignedNodeDigestError> {
        if signed_node_digest.info() != &self.signed_node_digest_info {
            return Err(SignedNodeDigestError::WrongDigest);
        }

        if self
            .aggregated_signature
            .contains_key(&signed_node_digest.peer_id())
        {
            return Err(SignedNodeDigestError::DuplicatedSignature);
        }

        self.aggregated_signature
            .insert(signed_node_digest.peer_id(), signed_node_digest.signature());
        Ok(())
    }

    pub(crate) fn ready(&self, validator_verifier: &ValidatorVerifier) -> bool {
        validator_verifier
            .check_voting_power(self.aggregated_signature.keys())
            .is_ok()
    }

    pub(crate) fn take(&self, validator_verifier: &ValidatorVerifier) -> NodeCertificate {
        let proof = match validator_verifier
            .aggregate_signatures(&PartialSignatures::new(self.aggregated_signature.clone()))
        {
            Ok(sig) => NodeCertificate::new(self.signed_node_digest_info.clone(), sig),
            Err(e) => unreachable!("Cannot aggregate signatures on digest err = {:?}", e),
        };
        proof
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct AckSet {
    digest: HashValue,
    set: HashSet<PeerId>,
}

impl AckSet {
    pub fn new(digest: HashValue) -> Self {
        Self {
            digest,
            set: HashSet::new(),
        }
    }

    pub fn add(&mut self, ack: CertifiedNodeAck) {
        if ack.digest() == self.digest {
            self.set.insert(ack.peer_id());
        }
    }

    pub fn missing_peers(&self, verifier: &ValidatorVerifier) -> Vec<PeerId> {
        let all_peers: HashSet<PeerId> = verifier
            .address_to_validator_index()
            .keys()
            .cloned()
            .collect();
        all_peers.difference(&self.set).cloned().collect()
    }
}

// TODO: bug - what if I link to a node but before broadcasting I already create a node in the next round.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) enum PeerStatus {// <=88
    Linked(Round),
    NotLinked(NodeMetaData),
}

impl PeerStatus {
    pub fn round(&self) -> Round {
        match self {
            PeerStatus::Linked(round) => *round,
            PeerStatus::NotLinked(metadata) => metadata.round(),
        }
    }

    pub fn not_linked(&self) -> bool {
        match self {
            PeerStatus::Linked(_) => false,
            PeerStatus::NotLinked(_) => true,
        }
    }

    fn metadata(self) -> NodeMetaData {
        match self {
            PeerStatus::Linked(_) => panic!("no metadata"),
            PeerStatus::NotLinked(metadata) => metadata,
        }
    }

    pub fn mark_linked(&mut self) -> Option<NodeMetaData> {
        let round = match self {
            PeerStatus::Linked(_) => None,
            PeerStatus::NotLinked(node_meta_data) => Some(node_meta_data.round()),
        };

        round.map(|r| std::mem::replace(self, PeerStatus::Linked(r)).metadata())
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct AbsentInfo {
    metadata: NodeMetaData, //88
    peers_to_request: HashSet<PeerId>, // <= 32 * 100
    immediate_dependencies: HashSet<HashValue>, // <= 32 * 100
}

impl AbsentInfo {
    pub fn new(metadata: NodeMetaData) -> Self {
        Self {
            metadata,
            peers_to_request: HashSet::new(),
            immediate_dependencies: HashSet::new(),
        }
    }

    pub fn metadata(&self) -> NodeMetaData {
        self.metadata.clone()
    }

    pub fn peers_to_request(&self) -> &HashSet<PeerId> {
        &self.peers_to_request
    }

    // pub fn take_immediate_dependencies(self) -> HashSet<HashValue> {
    //     self.immediate_dependencies
    // }

    pub fn immediate_dependencies(&self) -> &HashSet<HashValue> {
        &self.immediate_dependencies
    }

    pub fn add_dependency(&mut self, digest: HashValue) {
        self.immediate_dependencies.insert(digest);
    }

    pub fn add_peer(&mut self, peer_id: PeerId) {
        self.peers_to_request.insert(peer_id);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PendingInfo {
    certified_node: CertifiedNode,
    missing_parents: HashSet<HashValue>,
    immediate_dependencies: HashSet<HashValue>,
}

impl PendingInfo {
    pub fn new(
        certified_node: CertifiedNode,
        missing_parents: HashSet<HashValue>,
        immediate_dependencies: HashSet<HashValue>,
    ) -> Self {
        Self {
            certified_node,
            missing_parents,
            immediate_dependencies,
        }
    }

    pub fn certified_node(&self) -> &CertifiedNode {
        &self.certified_node
    }

    pub fn metadata(&self) -> NodeMetaData {
        self.certified_node.metadata().clone()
    }

    // pub fn immediate_dependencies(&self) -> &HashSet<HashValue> {
    //     &self.immediate_dependencies
    // }

    pub fn missing_parents(&self) -> &HashSet<HashValue> {
        &self.missing_parents
    }

    pub fn take(self) -> (CertifiedNode, HashSet<HashValue>) {
        (self.certified_node, self.immediate_dependencies)
    }

    // pub fn take_immediate_dependencies(self) -> HashSet<HashValue> {
    //     self.immediate_dependencies
    // }

    pub fn remove_missing_parent(&mut self, digest: HashValue) {
        self.missing_parents.remove(&digest);
    }

    pub fn ready_to_be_added(&self) -> bool {
        self.missing_parents.is_empty()
    }

    pub fn add_dependency(&mut self, digest: HashValue) {
        self.immediate_dependencies.insert(digest);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) enum MissingDagNodeStatus {
    Absent(AbsentInfo),
    Pending(PendingInfo),
}

impl MissingDagNodeStatus {
    pub fn update_to_pending(
        &mut self,
        certified_node: CertifiedNode,
        missing_parents: HashSet<NodeMetaData>,
    ) {
        match self {
            MissingDagNodeStatus::Absent(absent_info) => {
                let dependencies = absent_info.immediate_dependencies().clone(); // can trade this clone with mem::replace.
                let missing_parents_digest = missing_parents
                    .iter()
                    .map(|metadata| metadata.digest())
                    .collect();
                let pending_info =
                    PendingInfo::new(certified_node, missing_parents_digest, dependencies);
                *self = MissingDagNodeStatus::Pending(pending_info);
                // std::mem::replace(self, MissingDagNodeStatus::Pending(pending_info));
            },
            MissingDagNodeStatus::Pending(_) => {},
        }
    }

    pub fn peers_to_request(&self) -> HashSet<PeerId> {
        match self {
            MissingDagNodeStatus::Absent(info) => info.peers_to_request().clone(),
            MissingDagNodeStatus::Pending(_) => {
                unreachable!("dag: should not call peers_to_request when node is pending")
            },
        }
    }

    pub fn get_certified_node(&self) -> Option<CertifiedNode> {
        match self {
            MissingDagNodeStatus::Absent(_) => None,
            MissingDagNodeStatus::Pending(info) => Some(info.certified_node().clone()),
        }
    }

    pub fn metadata(&self) -> NodeMetaData {
        match self {
            MissingDagNodeStatus::Absent(info) => info.metadata(),
            MissingDagNodeStatus::Pending(info) => info.metadata(),
        }
    }

    pub fn absent(&self) -> bool {
        match self {
            MissingDagNodeStatus::Absent(_) => true,
            MissingDagNodeStatus::Pending(_) => false,
        }
    }

    pub fn take_node_and_dependencies(self) -> (CertifiedNode, HashSet<HashValue>) {
        match self {
            MissingDagNodeStatus::Absent(_) => {
                unreachable!("dag: should not call take_node_and_dependencies when node is absent")
            },
            MissingDagNodeStatus::Pending(info) => info.take(),
        }
    }

    // pub fn take_dependencies(self) -> HashSet<HashValue> {
    //     match self {
    //         MissingDagNodeStatus::Absent(info) => info.take_immediate_dependencies(),
    //         MissingDagNodeStatus::Pending(info) => info.take_immediate_dependencies(),
    //     }
    // }

    pub fn remove_missing_parent(&mut self, digets: HashValue) {
        match self {
            MissingDagNodeStatus::Absent(_) => {
                unreachable!("dag: node is absent, no missing parents")
            },
            MissingDagNodeStatus::Pending(info) => info.remove_missing_parent(digets),
        }
    }

    pub fn ready_to_be_added(&self) -> bool {
        match self {
            MissingDagNodeStatus::Absent(_) => false,
            MissingDagNodeStatus::Pending(info) => info.ready_to_be_added(),
        }
    }

    pub fn add_dependency(&mut self, digest: HashValue) {
        match self {
            MissingDagNodeStatus::Absent(info) => info.add_dependency(digest),
            MissingDagNodeStatus::Pending(info) => info.add_dependency(digest),
        }
    }

    pub fn add_peer_to_request(&mut self, peer_id: PeerId) {
        match self {
            MissingDagNodeStatus::Absent(info) => {
                info.add_peer(peer_id)
            },
            MissingDagNodeStatus::Pending(_) => {},
        }
    }
}

pub(crate) mod dag_round_list;
pub(crate) mod peer_status_list;
pub(crate) mod peer_index_map;
pub(crate) mod week_link_creator;
pub(crate) mod peer_node_map;
pub(crate) mod dag_in_mem;
pub(crate) mod missing_node_status_map;
