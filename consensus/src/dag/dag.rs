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
use std::rc::Weak;
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
use crate::dag::types::{AbsentInfo, DagInMem, DagInMem_Key, DagRoundList, MissingDagNodeStatus, MissingNodeIdToStatusMap, PeerIdToCertifiedNodeMap, PendingInfo, WeakLinksCreator};

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
        let in_mem = match storage.load_dag_in_mem(&key).expect("235922") {
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
        self.in_mem.get_dag()
            .get(round as usize)
            .map(|m| m.contains_key(&source))
            == Some(true)
    }

    fn get_node_metadata_from_dag(&self, round: Round, source: PeerId) -> Option<NodeMetaData> {
        self.in_mem.get_dag()
            .get(round as usize)
            .map(|m| m.get(&source).map(|m| m.metadata().clone()))
            .map_or(None, |o| o)
    }

    pub fn get_node(&self, node_request: &CertifiedNodeRequest) -> Option<CertifiedNode> {
        let maybe_from_dag = self.in_mem
            .get_dag()
            .get(node_request.round() as usize)
            .map(|m| m.get(&node_request.source()).cloned())
            .unwrap_or_default();

        let maybe_from_pending = self
            .in_mem.get_missing_nodes()
            .get(&node_request.digest())
            .map(|status| status.get_certified_node())
            .unwrap_or_default();

        maybe_from_dag.or(maybe_from_pending)
    }

    fn pending(&self, digest: HashValue) -> bool {
        match self.in_mem.get_missing_nodes().get(&digest) {
            None => false,
            Some(status) => match status {
                MissingDagNodeStatus::Absent(_) => false,
                MissingDagNodeStatus::Pending(_) => true,
            },
        }
    }

    pub fn missing_nodes_metadata(&self) -> HashSet<(NodeMetaData, Vec<PeerId>)> {
        self.in_mem.get_missing_nodes()
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
        self.in_mem.get_dag()
            .get(self.in_mem.current_round as usize)
            .unwrap()
            .iter()
            .map(|(_, certified_node)| certified_node.node().metadata().clone())
            .collect()
    }

    fn current_round_peers(&self) -> impl Iterator<Item = &PeerId> {
        info!("current_round={}", self.in_mem.current_round);
        info!("dag_len={}", self.in_mem.get_dag().len());
        self.in_mem.get_dag()
            .get(self.in_mem.current_round as usize)
            .unwrap()
            .iter()
            .map(|(_, certified_node)| certified_node.node().source_ref())
    }

    async fn add_to_dag(&mut self, certified_node: CertifiedNode, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
        let round = certified_node.node().round() as usize;
        // assert!(self.in_mem.dag.len() >= round - 1);

        if self.in_mem.get_dag().len() <= round {
            self.in_mem.get_dag_mut().push(PeerIdToCertifiedNodeMap::new());
        }
        self.in_mem.get_dag_mut().get_mut(round).unwrap().insert(certified_node.node().source(), certified_node.clone());

        self.in_mem.get_front_mut()
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
    async fn add_to_dag_and_update_pending(&mut self, node_status: MissingDagNodeStatus, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
        let (certified_node, dependencies) = node_status.take_node_and_dependencies();
        let digest = certified_node.digest();
        self.add_to_dag(certified_node, storage_diff).await;
        self.update_pending_nodes(dependencies, digest, storage_diff).await;
        // TODO: should we persist?
    }

    #[async_recursion]
    async fn update_pending_nodes(
        &mut self,
        recently_added_node_dependencies: HashSet<HashValue>,
        recently_added_node_digest: HashValue,
        storage_diff: &mut Box<dyn DagStoreWriteBatch>,
    ) {
        for digest in recently_added_node_dependencies {
            let mut maybe_status = None;
            match self.in_mem.get_missing_nodes_mut().entry(digest) {
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
                self.add_to_dag_and_update_pending(status, storage_diff).await;
            }
        }
    }

    fn add_peers_recursively(&mut self, digest: HashValue, source: PeerId) {
        let missing_parents = match self.in_mem.get_missing_nodes_mut().get(&digest).unwrap() {
            MissingDagNodeStatus::Absent(_) => HashSet::new(),
            MissingDagNodeStatus::Pending(info) => info.missing_parents().clone(),
        };

        for parent_digest in missing_parents {
            match self.in_mem.get_missing_nodes_mut().entry(parent_digest) {
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
        storage_diff: &mut Box<dyn DagStoreWriteBatch>,
    ) {
        let pending_peer_id = certified_node.node().source();
        let pending_digest = certified_node.node().digest();
        let missing_parents_digest = missing_parents
            .iter()
            .map(|metadata| metadata.digest())
            .collect();

        let pending_info = PendingInfo::new(certified_node, missing_parents_digest, HashSet::new());
        self.in_mem.get_missing_nodes_mut()
            .insert(pending_digest, MissingDagNodeStatus::Pending(pending_info));

        // TODO: Persist

        for node_meta_data in missing_parents {
            let digest = node_meta_data.digest();
            let status = self
                .in_mem.get_missing_nodes_mut()
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
                    .in_mem.get_dag()
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
        let current_round = self.in_mem.current_round;
        self.in_mem.get_front_mut()
            .update_with_strong_links(current_round, strong_links_peers);
        self.in_mem.current_round += 1;

        if self.in_mem.get_dag().get(self.in_mem.current_round as usize).is_none() {
            let new_node_map = PeerIdToCertifiedNodeMap::new();
            self.in_mem.get_dag_mut().push(new_node_map);
        }

        let mut batch = self.storage.new_write_batch();
        batch.put_dag_in_mem(&self.in_mem).unwrap();
        self.storage.commit_write_batch(batch).unwrap();
        let new_round = self.in_mem.current_round;
        return Some(
            parents
                .union(&self.in_mem.get_front_mut().get_weak_links(new_round))
                .cloned()
                .collect(),
        );
    }

    pub async fn try_add_node(&mut self, certified_node: CertifiedNode, storage_diff: &mut Box<dyn DagStoreWriteBatch>) {
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

        match self.in_mem.get_missing_nodes_mut().entry(certified_node.digest()) {
            // Node not in the system
            Entry::Vacant(_) => {
                if missing_parents.is_empty() {
                    self.add_to_dag(certified_node, storage_diff).await;
                } else {
                    self.add_to_pending(certified_node, missing_parents, storage_diff);
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
            self.add_to_dag_and_update_pending(node_status, storage_diff).await;
        }
    }
}
