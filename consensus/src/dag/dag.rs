// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{anchor_election::AnchorElection, bullshark::Bullshark},
    payload_manager::PayloadManager,
};
use aptos_consensus_types::{block::Block, node::{CertifiedNode, CertifiedNodeRequest, NodeMetaData}};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{block_info::Round, PeerId, validator_verifier::ValidatorVerifier};
use async_recursion::async_recursion;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::Arc,
};
use std::borrow::Borrow;
use std::collections::hash_map::Iter;
use std::io::{Cursor, Read, Write};
use std::ops::{Index, IndexMut};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::Buf;
use chrono::format::Item;
use futures::AsyncWriteExt;
use tokio::sync::Mutex;
use aptos_network::peer::Peer;
use aptos_schemadb::{ColumnFamilyName, define_schema, SchemaBatch};
use aptos_schemadb::schema::{KeyCodec, Schema, ValueCodec};
use crate::dag::dag_storage::{ContainsKey, DagStorage, DagStoreWriteBatch, ItemId, null_id};
use serde::{Deserialize, Serialize};
use crate::dag::dag_storage::naive::NaiveDagStoreWriteBatch;


// TODO: bug - what if I link to a node but before broadcasting I already create a node in the next round.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
enum PeerStatus {
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

///keeps track of weak links. None indicates that a (strong or weak) link was already added.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct WeakLinksCreator {
    id: ItemId,
    my_id: PeerId,
    latest_nodes_metadata: Vec<Option<PeerStatus>>,
    address_to_validator_index: HashMap<PeerId, usize>,
}

impl WeakLinksCreator {
    pub fn new(my_id: PeerId, verifier: &ValidatorVerifier) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            my_id,
            latest_nodes_metadata: verifier
                .address_to_validator_index()
                .iter()
                .map(|_| None)
                .collect(),
            address_to_validator_index: verifier.address_to_validator_index().clone(),
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

    pub fn update_peer_latest_node(&mut self, node_meta_data: NodeMetaData) {
        let peer_index = self
            .address_to_validator_index
            .get(&node_meta_data.source())
            .expect("invalid peer_id node metadata");

        let need_to_update = match &self.latest_nodes_metadata[*peer_index] {
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
            self.latest_nodes_metadata[*peer_index] = Some(PeerStatus::NotLinked(node_meta_data));
        } else {
            info!("DAG: not updating peer latest node: my_id {},", self.my_id);
        }
    }

    pub fn update_with_strong_links(&mut self, round: Round, strong_links: Vec<PeerId>) {
        for peer_id in strong_links {
            let index = self.address_to_validator_index.get(&peer_id).unwrap();
            debug_assert!(self.latest_nodes_metadata[*index].as_ref().unwrap().round() >= round);
            if self.latest_nodes_metadata[*index].as_ref().unwrap().round() == round {
                debug_assert!(self.latest_nodes_metadata[*index]
                    .as_ref()
                    .unwrap()
                    .not_linked());
                self.latest_nodes_metadata[*index]
                    .as_mut()
                    .unwrap()
                    .mark_linked();
            }
        }
    }
}

impl ContainsKey for WeakLinksCreator {
    type Key = ItemId;

    fn key(&self) -> Self::Key {
        self.id
    }
}

define_schema!(WeakLinksCreatorSchema, ItemId, WeakLinksCreator, "WeakLinksCreator");

impl KeyCodec<WeakLinksCreatorSchema> for ItemId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        let x = ItemId::try_from(data)?;
        Ok(x)
    }
}

impl ValueCodec<WeakLinksCreatorSchema> for WeakLinksCreator {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let buf = bcs::to_bytes(self)?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct AbsentInfo {
    metadata: NodeMetaData, //88
    peers_to_request: HashSet<PeerId>,
    immediate_dependencies: HashSet<HashValue>,
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

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
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



#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct PeerIdToCertifiedNodeMap {
    id: ItemId,
    inner: HashMap<PeerId, CertifiedNode>,
}

impl PeerIdToCertifiedNodeMap {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new()
        }
    }

    pub fn get(&self, k: &PeerId) -> Option<&CertifiedNode> {
        self.inner.get(k)
    }

    pub fn insert(&mut self, k: PeerId, v: CertifiedNode) -> Option<CertifiedNode> {
        self.inner.insert(k, v)
    }

    pub fn iter(&self) -> Iter<PeerId, CertifiedNode> {
        self.inner.iter()
    }

    pub fn contains_key(&self, k: &PeerId) -> bool {
        self.inner.contains_key(k)
    }
}

impl ContainsKey for PeerIdToCertifiedNodeMap {
    type Key = ItemId;

    fn key(&self) -> Self::Key {
        self.id
    }
}




#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagRoundList {
    id: ItemId,
    inner: Vec<PeerIdToCertifiedNodeMap>,
}

impl DagRoundList {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: vec![],
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&PeerIdToCertifiedNodeMap> {
        self.inner.get(index)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn push(&mut self, dag_round: PeerIdToCertifiedNodeMap) {
        self.inner.push(dag_round)
    }
}

impl ContainsKey for DagRoundList {
    type Key = ItemId;

    fn key(&self) -> Self::Key {
        self.id
    }
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeIdToStatusMap {
    id: ItemId,
    inner: HashMap<HashValue, MissingDagNodeStatus>,
}

impl MissingNodeIdToStatusMap {
}

impl MissingNodeIdToStatusMap {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, k: &HashValue) -> Option<&MissingDagNodeStatus> {
        self.inner.get(k)
    }

    fn key(&self) -> ItemId {
        self.id
    }
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub (crate) struct DagInMem_Key {
    my_id: PeerId,
    epoch: u64,
}

/// The part of the DAG data that should be persisted.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem {
    my_id: PeerId,
    epoch: u64,
    current_round: u64,
    // starts from 0, which is genesys
    front: WeakLinksCreator,
    dag: DagRoundList,
    // TODO: protect from DDoS - currently validators can add unbounded number of entries
    missing_nodes: MissingNodeIdToStatusMap,
}

impl DagInMem {
    pub(crate) fn partial(&self) -> DagInMem_Partial {
        DagInMem_Partial {
            my_id: self.my_id,
            epoch: self.epoch,
            current_round: self.current_round,
            front: self.front.key(),
            dag: self.dag.key(),
            missing_nodes: self.missing_nodes.key(),
        }
    }
}
/// The part of the DAG data that should be persisted.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct DagInMem_Partial {
    my_id: PeerId,
    epoch: u64,
    current_round: u64,
    front: ItemId,
    dag: ItemId,
    missing_nodes: ItemId,
}


impl ContainsKey for DagInMem {
    type Key = DagInMem_Key;

    fn key(&self) -> Self::Key {
        DagInMem_Key {
            my_id: self.my_id,
            epoch: self.epoch,
        }
    }
}

impl KeyCodec<DagInMemSchema> for DagInMem_Key {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<DagInMemSchema> for DagInMem_Partial {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Write::write(&mut buf, self.my_id.as_slice())?;
        buf.write_u64::<BigEndian>(self.epoch)?;
        buf.write_u64::<BigEndian>(self.current_round)?;
        Write::write(&mut buf, self.front.as_slice())?;
        Write::write(&mut buf, self.dag.as_slice())?;
        Write::write(&mut buf, self.missing_nodes.as_slice())?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        let mut c = Cursor::new(data);
        let my_id = PeerId::from_bytes(read_bytes(&mut c, 32)?).unwrap();
        let epoch = c.read_u64::<BigEndian>()?;
        let current_round = c.read_u64::<BigEndian>()?;
        let front = ItemId::try_from(read_bytes(&mut c, 16)?).unwrap();
        let dag = ItemId::try_from(read_bytes(&mut c, 16)?).unwrap();
        let missing_nodes = ItemId::try_from(read_bytes(&mut c, 16)?).unwrap();
        let ret = Self {
            my_id,
            epoch,
            current_round,
            front,
            dag,
            missing_nodes,
        };
        Ok(ret)
    }
}

fn read_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        let byte = cursor.read_u8()?;
        bytes.push(byte);
    }
    Ok(bytes)
}

define_schema!(DagInMemSchema, DagInMem_Key, DagInMem_Partial, "DagInMem");

// TODO: persist all every update
#[allow(dead_code)]
pub(crate) struct Dag {
    in_mem: DagInMem,
    // Arc to something that returns the anchors
    proposer_election: Arc<dyn AnchorElection>,
    bullshark: Arc<Mutex<Bullshark>>,
    verifier: ValidatorVerifier,
    payload_manager: Arc<PayloadManager>,
    storage: Arc<dyn DagStorage>,
}

#[allow(dead_code)]
impl Dag {
    pub fn new(
        my_id: PeerId,
        epoch: u64,
        bullshark: Arc<Mutex<Bullshark>>,
        verifier: ValidatorVerifier,
        proposer_election: Arc<dyn AnchorElection>,
        payload_manager: Arc<PayloadManager>,
        mut storage: Arc<dyn DagStorage>,
    ) -> Self {
        let key = DagInMem_Key { my_id, epoch };
        let in_mem = match storage.get_dag_in_mem(&key).expect("235922") {
            Some(in_mem) => in_mem,
            None => {
                let mut round_list = DagRoundList::new();
                round_list.push(PeerIdToCertifiedNodeMap::new());
                let in_mem = DagInMem {
                    my_id,
                    epoch,
                    current_round: 0,
                    front: WeakLinksCreator::new(my_id, &verifier),
                    dag: round_list,
                    missing_nodes: MissingNodeIdToStatusMap::new(),
                };
                let mut batch = storage.new_write_batch();
                batch.put_dag_in_mem(&in_mem).unwrap();
                storage.commit_write_batch(batch).unwrap();
                in_mem
            }
        };

        Self {
            in_mem,
            proposer_election,
            bullshark,
            verifier,
            payload_manager,
            storage,
        }
    }

    fn contains(&self, metadata: &NodeMetaData) -> bool {
        self.in_dag(metadata.round(), metadata.source()) || self.pending(metadata.digest())
    }

    fn in_dag(&self, round: Round, source: PeerId) -> bool {
        self.in_mem.dag
            .get(round as usize)
            .map(|m| m.contains_key(&source))
            == Some(true)
    }

    fn get_node_metadata_from_dag(&self, round: Round, source: PeerId) -> Option<NodeMetaData> {
        self.in_mem.dag
            .get(round as usize)
            .map(|m| m.get(&source).map(|m| m.metadata().clone()))
            .map_or(None, |o| o)
    }

    pub fn get_node(&self, node_request: &CertifiedNodeRequest) -> Option<CertifiedNode> {
        let maybe_from_dag = self.in_mem
            .dag
            .get(node_request.round() as usize)
            .map(|m| m.get(&node_request.source()).cloned())
            .unwrap_or_default();

        let maybe_from_pending = self
            .in_mem.missing_nodes.inner
            .get(&node_request.digest())
            .map(|status| status.get_certified_node())
            .unwrap_or_default();

        maybe_from_dag.or(maybe_from_pending)
    }

    fn pending(&self, digest: HashValue) -> bool {
        match self.in_mem.missing_nodes.get(&digest) {
            None => false,
            Some(status) => match status {
                MissingDagNodeStatus::Absent(_) => false,
                MissingDagNodeStatus::Pending(_) => true,
            },
        }
    }

    pub fn missing_nodes_metadata(&self) -> HashSet<(NodeMetaData, Vec<PeerId>)> {
        self.in_mem.missing_nodes.inner
            .iter()
            .filter(|(_, status)| status.absent())
            .map(|(_, status)| {
                (
                    status.metadata(),
                    status.peers_to_request().into_iter().collect(),
                )
            })
            .collect()
    }

    fn current_round_nodes_metadata(&self) -> HashSet<NodeMetaData> {
        self.in_mem.dag
            .get(self.in_mem.current_round as usize)
            .unwrap()
            .iter()
            .map(|(_, certified_node)| certified_node.node().metadata().clone())
            .collect()
    }

    fn current_round_peers(&self) -> impl Iterator<Item = &PeerId> {
        info!("current_round={}", self.in_mem.current_round);
        info!("dag_len={}", self.in_mem.dag.len());
        self.in_mem.dag
            .get(self.in_mem.current_round as usize)
            .unwrap()
            .iter()
            .map(|(_, certified_node)| certified_node.node().source_ref())
    }

    async fn add_to_dag(&mut self, certified_node: CertifiedNode) {
        let round = certified_node.node().round() as usize;
        // assert!(self.in_mem.dag.len() >= round - 1);

        if self.in_mem.dag.len() <= round {
            self.in_mem.dag.push(PeerIdToCertifiedNodeMap::new());
        }
        self.in_mem.dag.inner[round].insert(certified_node.node().source(), certified_node.clone());

        self.in_mem.front
            .update_peer_latest_node(certified_node.node().metadata().clone());

        //TODO: write the diff only.
        let mut batch = self.storage.new_write_batch();
        batch.put_dag_in_mem(&self.in_mem).unwrap();
        self.storage.commit_write_batch(batch).unwrap();

        self.payload_manager
            .prefetch_payload_data_inner(
                self.in_mem.epoch,
                self.in_mem.current_round,
                certified_node.node().timestamp(),
                certified_node.node().maybe_payload().unwrap(),
            )
            .await;

        let mut bs = self.bullshark.lock().await;

        bs.try_ordering(certified_node.take_node()).await;

        // TODO: send/call to all subscribed application and make sure shutdown logic is safe with the expect.
    }

    #[async_recursion]
    async fn add_to_dag_and_update_pending(&mut self, node_status: MissingDagNodeStatus) {
        let (certified_node, dependencies) = node_status.take_node_and_dependencies();
        let digest = certified_node.digest();
        self.add_to_dag(certified_node).await;
        self.update_pending_nodes(dependencies, digest).await;
        // TODO: should we persist?
    }

    #[async_recursion]
    async fn update_pending_nodes(
        &mut self,
        recently_added_node_dependencies: HashSet<HashValue>,
        recently_added_node_digest: HashValue,
    ) {
        for digest in recently_added_node_dependencies {
            let mut maybe_status = None;
            match self.in_mem.missing_nodes.inner.entry(digest) {
                Entry::Occupied(mut entry) => {
                    entry
                        .get_mut()
                        .remove_missing_parent(recently_added_node_digest);

                    // TODO: make this a method and call from try_add_node_and_advance_round if getting a missing node.
                    if entry.get_mut().ready_to_be_added() {
                        maybe_status = Some(entry.remove());
                        // self.add_to_dag_and_update_pending(entry.remove());
                    }
                },
                Entry::Vacant(_) => unreachable!("pending node is missing"),
            }
            if let Some(status) = maybe_status {
                self.add_to_dag_and_update_pending(status).await;
            }
        }
    }

    fn add_peers_recursively(&mut self, digest: HashValue, source: PeerId) {
        let missing_parents = match self.in_mem.missing_nodes.get(&digest).unwrap() {
            MissingDagNodeStatus::Absent(_) => HashSet::new(),
            MissingDagNodeStatus::Pending(info) => info.missing_parents().clone(),
        };

        for parent_digest in missing_parents {
            match self.in_mem.missing_nodes.inner.entry(parent_digest) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().add_peer_to_request(source);
                    self.add_peers_recursively(parent_digest, source);
                },
                Entry::Vacant(_) => unreachable!("node should exist in missing nodes"),
            };
        }
    }

    fn add_to_pending(
        &mut self,
        certified_node: CertifiedNode, // assumption that node not pending.
        missing_parents: HashSet<NodeMetaData>,
    ) {
        let pending_peer_id = certified_node.node().source();
        let pending_digest = certified_node.node().digest();
        let missing_parents_digest = missing_parents
            .iter()
            .map(|metadata| metadata.digest())
            .collect();

        let pending_info = PendingInfo::new(certified_node, missing_parents_digest, HashSet::new());
        self.in_mem.missing_nodes.inner
            .insert(pending_digest, MissingDagNodeStatus::Pending(pending_info));

        // TODO: Persist

        for node_meta_data in missing_parents {
            let digest = node_meta_data.digest();
            let status = self
                .in_mem.missing_nodes.inner
                .entry(digest)
                .or_insert(MissingDagNodeStatus::Absent(AbsentInfo::new(
                    node_meta_data,
                )));

            status.add_dependency(pending_digest);
            status.add_peer_to_request(pending_peer_id);

            self.add_peers_recursively(digest, pending_peer_id); // Recursively update source_peers.
        }
    }

    fn round_ready(&self, timeout: bool) -> bool {
        if self
            .verifier
            .check_voting_power(self.current_round_peers())
            .is_err()
        {
            return false;
        }
        if timeout {
            return true;
        }

        let wave = self.in_mem.current_round / 2;
        let anchor = self.proposer_election.get_round_anchor_peer_id(wave);
        let maybe_anchor_node_meta_data =
            self.get_node_metadata_from_dag(self.in_mem.current_round, anchor);

        return if self.in_mem.current_round % 2 == 0 {
            maybe_anchor_node_meta_data.is_some()
        } else {
            // TODO: since commit rule is f+1 we do not need to timeout on odd rounds. Verify!
            if let Some(anchor_node_meta_data) = maybe_anchor_node_meta_data {
                let voting_peers = self
                    .in_mem.dag
                    .get(self.in_mem.current_round as usize)
                    .unwrap()
                    .iter()
                    .filter(|(_, certified_node)| {
                        certified_node
                            .node()
                            .parents()
                            .contains(&anchor_node_meta_data)
                    })
                    .map(|(_, certified_node)| certified_node.node().source_ref());

                self.verifier
                    .check_minority_voting_power(voting_peers)
                    .is_ok()
            } else {
                false
            }
        };
    }

    pub fn try_advance_round(&mut self, timeout: bool) -> Option<HashSet<NodeMetaData>> {
        if !self.round_ready(timeout) {
            return None;
        }

        info!("ready to move to round {}", self.in_mem.current_round + 1);

        let parents = self.current_round_nodes_metadata();
        let strong_links_peers = parents.iter().map(|m| m.source().clone()).collect();
        self.in_mem.front
            .update_with_strong_links(self.in_mem.current_round, strong_links_peers);
        self.in_mem.current_round += 1;

        if self.in_mem.dag.get(self.in_mem.current_round as usize).is_none() {
            let new_node_map = PeerIdToCertifiedNodeMap::new();
            self.in_mem.dag.push(new_node_map);
        }

        let mut batch = self.storage.new_write_batch();
        batch.put_dag_in_mem(&self.in_mem).unwrap();
        self.storage.commit_write_batch(batch).unwrap();

        return Some(
            parents
                .union(&self.in_mem.front.get_weak_links(self.in_mem.current_round))
                .cloned()
                .collect(),
        );
    }

    pub async fn try_add_node(&mut self, certified_node: CertifiedNode) {
        info!(
            "DAG: trying to add node: my_id {}, round {}, peer_id {}",
            self.in_mem.my_id,
            certified_node.round(),
            certified_node.source()
        );
        if self.contains(certified_node.metadata()) {
            return;
        }

        let missing_parents: HashSet<NodeMetaData> = certified_node
            .parents()
            .iter()
            .filter(|metadata| !self.in_dag(metadata.round(), metadata.source()))
            .cloned()
            .collect();

        let mut maybe_node_status = None;

        match self.in_mem.missing_nodes.inner.entry(certified_node.digest()) {
            // Node not in the system
            Entry::Vacant(_) => {
                if missing_parents.is_empty() {
                    self.add_to_dag(certified_node).await;
                } else {
                    self.add_to_pending(certified_node, missing_parents);
                }
            },

            // Node is absent
            Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .update_to_pending(certified_node, missing_parents);
                if entry.get_mut().ready_to_be_added() {
                    maybe_node_status = Some(entry.remove());
                }
            },
        }

        if let Some(node_status) = maybe_node_status {
            self.add_to_dag_and_update_pending(node_status).await;
        }
    }
}
