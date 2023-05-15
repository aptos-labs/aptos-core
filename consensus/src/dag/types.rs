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
use aptos_schemadb::define_schema;
use crate::dag::dag_storage::{DagStoreWriteBatch, ItemId};
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundList {
    pub(crate) id: ItemId,
    pub(crate) inner: Vec<PeerNodeMap>,
}

impl DagRoundList {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: vec![],
        }
    }

    pub(crate) fn metadata(&self) -> DagRoundList_Metadata {
        DagRoundList_Metadata {
            id: self.id,
            len: self.inner.len() as u64,
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&PeerNodeMap> {
        self.inner.get(index)
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut PeerNodeMap> {
        self.inner.get_mut(index)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn push(&mut self, dag_round: PeerNodeMap) {
        self.inner.push(dag_round)
    }

    pub(crate) fn iter(&self) -> core::slice::Iter<PeerNodeMap> {
        self.inner.iter()
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundList_Metadata {
    pub(crate) id: ItemId,
    pub(crate) len: u64,
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundListItem_Key {
    pub(crate) id: ItemId,
    pub(crate) index: u64,
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundListItem {
    pub(crate) list_id: ItemId,
    pub(crate) index: u64,
    pub(crate) content_id: ItemId,
}

impl DagRoundListItem {
    pub(crate) fn key(&self) -> DagRoundListItem_Key {
        DagRoundListItem_Key {
            id: self.list_id,
            index: self.index,
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusListItem {
    pub(crate) list_id: ItemId,
    pub(crate) index: usize,
    pub(crate) content: Option<PeerStatus>,
}

impl PeerStatusListItem {
    pub(crate) fn key(&self) -> PeerStatusListItem_Key {
        PeerStatusListItem_Key {
            list_id: self.list_id,
            index: self.index,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusListItem_Key {
    pub(crate) list_id: ItemId,
    pub(crate) index: usize,
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusList_Metadata {
    pub(crate) id: ItemId,
    pub(crate) len: u64,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerStatusList {
    pub(crate) id: ItemId,
    pub(crate) inner: Vec<Option<PeerStatus>>,
}

impl PeerStatusList {
    pub(crate) fn new(list: Vec<Option<PeerStatus>>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: list
        }
    }

    pub(crate) fn metadata(&self) -> PeerStatusList_Metadata {
        PeerStatusList_Metadata {
            id: self.id,
            len: self.inner.len() as u64,
        }
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<Option<PeerStatus>>{
        self.inner.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> std::slice::IterMut<Option<PeerStatus>>{
        self.inner.iter_mut()
    }

    pub(crate) fn get(&self, i: usize) -> Option<&Option<PeerStatus>> {
        self.inner.get(i)
    }

    pub(crate) fn get_mut(&mut self, i: usize) -> Option<&mut Option<PeerStatus>> {
        self.inner.get_mut(i)
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerIndexMap {
    pub(crate) id: ItemId,
    inner: HashMap<PeerId, usize>,
}

impl PeerIndexMap {
    pub(crate) fn new(inner: HashMap<PeerId, usize>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner,
        }
    }

    pub(crate) fn get(&self, k: &PeerId) -> Option<&usize> {
        self.inner.get(k)
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct WeakLinksCreatorMetadata {
    pub(crate) id: ItemId,
    pub(crate) my_id: PeerId,
    pub(crate) latest_nodes_metadata: ItemId,
    pub(crate) address_to_validator_index: ItemId,
}
///keeps track of weak links. None indicates that a (strong or weak) link was already added.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct WeakLinksCreator {
    pub(crate) id: ItemId,
    pub(crate) my_id: PeerId,
    pub(crate) latest_nodes_metadata: PeerStatusList,
    pub(crate) address_to_validator_index: PeerIndexMap,
}

impl WeakLinksCreator {
    pub fn new(my_id: PeerId, verifier: &ValidatorVerifier) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            my_id,
            latest_nodes_metadata: PeerStatusList::new(verifier
                .address_to_validator_index()
                .iter()
                .map(|_| None)
                .collect()),
            address_to_validator_index: PeerIndexMap::new(verifier.address_to_validator_index().clone()),
        }
    }

    pub fn metadata(&self) -> WeakLinksCreatorMetadata {
        WeakLinksCreatorMetadata {
            id: self.id,
            my_id: self.my_id,
            latest_nodes_metadata: self.latest_nodes_metadata.id,
            address_to_validator_index: self.address_to_validator_index.id,
        }
    }

    pub fn get_weak_links(&mut self, new_round: Round) -> HashSet<NodeMetaData> {
        self.latest_nodes_metadata
            .iter_mut()
            .filter(|node_status| {
                node_status.is_some()
                    && node_status.as_ref().unwrap().not_linked()
                    && node_status.as_ref().unwrap().round() < new_round - 1
            })
            .map(|node_status| node_status.as_mut().unwrap().mark_linked().unwrap())
            .collect()
    }

    pub fn update_peer_latest_node(&mut self, node_meta_data: NodeMetaData, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
        let peer_index = self
            .address_to_validator_index
            .get(&node_meta_data.source())
            .expect("invalid peer_id node metadata");

        let need_to_update = match &self.latest_nodes_metadata.get(*peer_index).unwrap() {
            Some(status) => status.round() < node_meta_data.round(),
            None => true,
        };
        if need_to_update {
            info!(
                "DAG: updating peer latest node: my_id {}, round {} peer_index {}",
                self.my_id,
                node_meta_data.round(),
                *peer_index
            );
            let new_val = Some(PeerStatus::NotLinked(node_meta_data));
            *self.latest_nodes_metadata.get_mut(*peer_index).unwrap() = new_val.clone();
            let list_item = PeerStatusListItem {
                list_id: self.latest_nodes_metadata.id,
                index: *peer_index,
                content: new_val.clone(),
            };
            storage_diff.put_peer_status_list_item(&list_item).unwrap();
            // storage_diff.put_peer_status_list(&self.latest_nodes_metadata).unwrap(); //TODO: only write the diff.
        } else {
            info!("DAG: not updating peer latest node: my_id {},", self.my_id);
        }
    }

    pub fn update_with_strong_links(&mut self, round: Round, strong_links: Vec<PeerId>, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
        for peer_id in strong_links {
            let index = self.address_to_validator_index.get(&peer_id).unwrap();
            debug_assert!(self.latest_nodes_metadata.get(*index).unwrap().as_ref().unwrap().round() >= round);
            if self.latest_nodes_metadata.get(*index).unwrap().as_ref().unwrap().round() == round {
                debug_assert!(self.latest_nodes_metadata.get(*index).unwrap()
                    .as_ref()
                    .unwrap()
                    .not_linked());
                self.latest_nodes_metadata.get_mut(*index)
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .mark_linked();
                storage_diff.put_peer_status_list_item(&PeerStatusListItem{
                    list_id: self.latest_nodes_metadata.id,
                    index: *index,
                    content: self.latest_nodes_metadata.get(*index).unwrap().clone(),
                }).unwrap();
            }
        }
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerNodeMap {
    pub(crate) id: ItemId,
    pub(crate) inner: HashMap<PeerId, CertifiedNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerNodeMapMetadata {
    pub(crate) id: ItemId,
    //TODO: either add some fields (like len), or delete this column family.
}

impl PeerNodeMap {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new()
        }
    }

    pub(crate) fn metadata(&self) -> PeerNodeMapMetadata {
        PeerNodeMapMetadata { id: self.id }
    }

    pub fn get(&self, k: &PeerId) -> Option<&CertifiedNode> {
        self.inner.get(k)
    }

    pub fn insert(&mut self, k: PeerId, v: CertifiedNode) -> Option<CertifiedNode> {
        self.inner.insert(k, v)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<PeerId, CertifiedNode> {
        self.inner.iter()
    }

    pub fn contains_key(&self, k: &PeerId) -> bool {
        self.inner.contains_key(k)
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerIdToCertifiedNodeMapEntry {
    pub(crate) map_id: ItemId,
    pub(crate) key: PeerId,
    pub(crate) value_id: HashValue,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerIdToCertifiedNodeMapEntry_Key {
    pub(crate) map_id: ItemId,
    pub(crate) key: Option<PeerId>,
}

impl PeerIdToCertifiedNodeMapEntry {
    pub(crate) fn key(&self) -> PeerIdToCertifiedNodeMapEntry_Key {
        PeerIdToCertifiedNodeMapEntry_Key {
            map_id: self.map_id,
            key: Some(self.key),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem_Key {
    pub(crate) my_id: PeerId,
    pub(crate) epoch: u64,
}

/// The part of the DAG data that should be persisted.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem {
    pub(crate) my_id: PeerId,
    pub(crate) epoch: u64,
    pub(crate) current_round: u64,
    // starts from 0, which is genesys
    pub(crate) front: WeakLinksCreator,
    pub(crate) dag: DagRoundList,
    // TODO: protect from DDoS - currently validators can add unbounded number of entries
    pub(crate) missing_nodes: MissingNodeStatusMap,
}

impl DagInMem {
    pub(crate) fn metadata(&self) -> DagInMem_Metadata {
        DagInMem_Metadata {
            my_id: self.my_id,
            epoch: self.epoch,
            current_round: self.current_round,
            front: self.front.id,
            dag: self.dag.id,
            missing_nodes: self.missing_nodes.id,
        }
    }

    pub(crate) fn key(&self) -> DagInMem_Key {
        DagInMem_Key {
            my_id: self.my_id,
            epoch: self.epoch,
        }
    }

    pub(crate) fn get_dag(&self) -> &DagRoundList {
        &self.dag
    }

    pub(crate) fn get_dag_mut(&mut self) -> &mut DagRoundList {
        &mut self.dag
    }

    pub(crate) fn get_front(&self) -> &WeakLinksCreator {
        &self.front
    }

    pub(crate) fn get_front_mut(&mut self) -> &mut WeakLinksCreator {
        &mut self.front
    }

    pub(crate) fn get_missing_nodes(&self) -> &MissingNodeStatusMap {
        &self.missing_nodes
    }

    pub(crate) fn get_missing_nodes_mut(&mut self) -> &mut MissingNodeStatusMap {
        &mut self.missing_nodes
    }
}

/// The part of the DAG data that should be persisted.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem_Metadata {
    pub(crate) my_id: PeerId,
    pub(crate) epoch: u64,
    pub(crate) current_round: u64,
    pub(crate) front: ItemId,
    pub(crate) dag: ItemId,
    pub(crate) missing_nodes: ItemId,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeStatusMap {
    pub(crate) id: ItemId,
    inner: HashMap<HashValue, MissingDagNodeStatus>,
}

impl MissingNodeStatusMap {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, k: &HashValue) -> Option<&MissingDagNodeStatus> {
        self.inner.get(k)
    }

    pub(crate) fn entry(&mut self, k: HashValue) -> std::collections::hash_map::Entry<'_, HashValue, MissingDagNodeStatus> {
        self.inner.entry(k)
    }

    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, HashValue, MissingDagNodeStatus> {
        self.inner.iter()
    }

    pub(crate) fn insert(&mut self, k: HashValue, v: MissingDagNodeStatus) -> Option<MissingDagNodeStatus> {
        self.inner.insert(k, v)
    }
}

////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeIdToStatusMap_Entry_Key {
    pub(crate) map_id: ItemId,
    pub(crate) key: Option<HashValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeIdToStatusMap_Entry {
    pub(crate) map_id: ItemId,
    pub(crate) key: HashValue,
    pub(crate) value: MissingDagNodeStatus,
}

impl MissingNodeIdToStatusMap_Entry {
    pub(crate) fn key(&self) -> MissingNodeIdToStatusMap_Entry_Key {
        MissingNodeIdToStatusMap_Entry_Key {
            map_id: self.map_id,
            key: Some(self.key),
        }
    }
}
