// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    types::{DagSnapshotBitmask, NodeMetadata},
    Node,
};
use crate::{
    dag::{
        storage::DAGStorage,
        types::{CertifiedNode, NodeCertificate},
    },
    payload_manager::TPayloadManager,
};
use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_logger::{debug, error, warn};
use aptos_types::{epoch_state::EpochState, validator_verifier::ValidatorVerifier};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone)]
pub enum NodeStatus {
    Unordered {
        node: Arc<CertifiedNode>,
        aggregated_weak_voting_power: u128,
        aggregated_strong_voting_power: u128,
    },
    Ordered(Arc<CertifiedNode>),
}

impl NodeStatus {
    pub fn as_node(&self) -> &Arc<CertifiedNode> {
        match self {
            NodeStatus::Unordered { node, .. } | NodeStatus::Ordered(node) => node,
        }
    }

    pub fn mark_as_ordered(&mut self) {
        assert!(matches!(self, NodeStatus::Unordered { .. }));
        *self = NodeStatus::Ordered(self.as_node().clone());
    }
}
/// Data structure that stores the in-memory DAG representation, it maintains round based index.
#[derive(Clone)]
pub struct InMemDag {
    nodes_by_round: BTreeMap<Round, Vec<Option<NodeStatus>>>,
    /// Map between peer id to vector index
    author_to_index: HashMap<Author, usize>,
    start_round: Round,
    epoch_state: Arc<EpochState>,
    /// The window we maintain between highest committed round and initial round
    window_size: u64,
}

impl InMemDag {
    pub fn new_empty(epoch_state: Arc<EpochState>, start_round: Round, window_size: u64) -> Self {
        let author_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let nodes_by_round = BTreeMap::new();
        Self {
            nodes_by_round,
            author_to_index,
            start_round,
            epoch_state,
            window_size,
        }
    }

    pub(crate) fn lowest_round(&self) -> Round {
        self.start_round
    }

    pub fn highest_round(&self) -> Round {
        // If stale nodes exist on the BTreeMap, ignore their rounds when calculating
        // the highest round.
        *self
            .nodes_by_round
            .last_key_value()
            .map(|(round, _)| round)
            .unwrap_or(&self.start_round)
            .max(&self.start_round)
    }

    /// The highest strong links round is either the highest round or the highest round - 1
    /// because we ensure all parents (strong links) exist for any nodes in the store
    pub fn highest_strong_links_round(&self, validator_verifier: &ValidatorVerifier) -> Round {
        let highest_round = self.highest_round();
        self.get_strong_links_for_round(highest_round, validator_verifier)
            .map_or_else(|| highest_round.saturating_sub(1), |_| highest_round)
    }

    #[cfg(test)]
    pub fn add_node_for_test(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        self.validate_new_node(&node)?;
        self.add_validated_node(node)
    }

    fn add_validated_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let round = node.round();
        ensure!(
            round >= self.lowest_round(),
            "dag was pruned. given round: {}, lowest round: {}",
            round,
            self.lowest_round()
        );

        let node = Arc::new(node);
        // Invariant violation, we must get the node ref (COMMENT ME)
        #[allow(clippy::unwrap_in_result)]
        let round_ref = self
            .get_node_ref_mut(node.round(), node.author())
            .expect("must be present");
        ensure!(round_ref.is_none(), "race during insertion");
        *round_ref = Some(NodeStatus::Unordered {
            node: node.clone(),
            aggregated_weak_voting_power: 0,
            aggregated_strong_voting_power: 0,
        });
        self.update_votes(&node, true);
        Ok(())
    }

    fn validate_new_node(&mut self, node: &CertifiedNode) -> anyhow::Result<()> {
        ensure!(
            node.epoch() == self.epoch_state.epoch,
            "different epoch {}, current {}",
            node.epoch(),
            self.epoch_state.epoch
        );
        let author = node.metadata().author();
        let index = *self
            .author_to_index
            .get(author)
            .ok_or_else(|| anyhow!("unknown author"))?;
        let round = node.metadata().round();
        ensure!(
            round >= self.lowest_round(),
            "round too low {}, lowest in dag {}",
            round,
            self.lowest_round()
        );
        ensure!(
            round <= self.highest_round() + 1,
            "round too high {}, highest in dag {}",
            round,
            self.highest_round()
        );
        if round > self.lowest_round() {
            for parent in node.parents() {
                ensure!(self.exists(parent.metadata()), "parent not exist");
            }
        }
        let round_ref = self
            .nodes_by_round
            .entry(round)
            .or_insert_with(|| vec![None; self.author_to_index.len()]);
        ensure!(round_ref[index].is_none(), "duplicate node");
        Ok(())
    }

    pub fn update_votes(&mut self, node: &Node, update_link_power: bool) {
        if node.round() <= self.lowest_round() {
            return;
        }

        let voting_power = self
            .epoch_state
            .verifier
            .get_voting_power(node.author())
            .expect("must exist");

        for parent in node.parents_metadata() {
            let node_status = self
                .get_node_ref_mut(parent.round(), parent.author())
                .expect("must exist");
            match node_status {
                Some(NodeStatus::Unordered {
                    aggregated_weak_voting_power,
                    aggregated_strong_voting_power,
                    ..
                }) => {
                    if update_link_power {
                        *aggregated_strong_voting_power += voting_power as u128;
                    } else {
                        *aggregated_weak_voting_power += voting_power as u128;
                    }
                },
                Some(NodeStatus::Ordered(_)) => {},
                None => unreachable!("parents must exist before voting for a node"),
            }
        }
    }

    pub fn exists(&self, metadata: &NodeMetadata) -> bool {
        self.get_node_ref_by_metadata(metadata).is_some()
    }

    pub fn all_exists<'a>(&self, nodes: impl Iterator<Item = &'a NodeMetadata>) -> bool {
        self.filter_missing(nodes).next().is_none()
    }

    pub fn all_exists_by_round_author<'a>(
        &self,
        mut nodes: impl Iterator<Item = &'a (Round, Author)>,
    ) -> bool {
        nodes.all(|(round, author)| self.get_node_ref(*round, author).is_some())
    }

    pub fn filter_missing<'a, 'b>(
        &'b self,
        nodes: impl Iterator<Item = &'a NodeMetadata> + 'b,
    ) -> impl Iterator<Item = &'a NodeMetadata> + 'b {
        nodes.filter(|node_metadata| !self.exists(node_metadata))
    }

    fn get_node_ref_by_metadata(&self, metadata: &NodeMetadata) -> Option<&NodeStatus> {
        self.get_node_ref(metadata.round(), metadata.author())
    }

    pub fn get_node_ref(&self, round: Round, author: &Author) -> Option<&NodeStatus> {
        let index = self.author_to_index.get(author)?;
        let round_ref = self.nodes_by_round.get(&round)?;
        round_ref[*index].as_ref()
    }

    fn get_node_ref_mut(
        &mut self,
        round: Round,
        author: &Author,
    ) -> Option<&mut Option<NodeStatus>> {
        let index = self.author_to_index.get(author)?;
        let round_ref = self.nodes_by_round.get_mut(&round)?;
        Some(&mut round_ref[*index])
    }

    fn get_round_iter(&self, round: Round) -> Option<impl Iterator<Item = &NodeStatus>> {
        self.nodes_by_round
            .get(&round)
            .map(|round_ref| round_ref.iter().flatten())
    }

    pub fn get_node(&self, metadata: &NodeMetadata) -> Option<Arc<CertifiedNode>> {
        self.get_node_ref_by_metadata(metadata)
            .map(|node_status| node_status.as_node().clone())
    }

    pub fn get_node_by_round_author(
        &self,
        round: Round,
        author: &Author,
    ) -> Option<&Arc<CertifiedNode>> {
        self.get_node_ref(round, author)
            .map(|node_status| node_status.as_node())
    }

    pub fn check_votes_for_node(
        &self,
        metadata: &NodeMetadata,
        validator_verifier: &ValidatorVerifier,
    ) -> bool {
        self.get_node_ref_by_metadata(metadata)
            .map(|node_status| match node_status {
                NodeStatus::Unordered {
                    aggregated_weak_voting_power,
                    aggregated_strong_voting_power,
                    ..
                } => {
                    validator_verifier
                        .check_aggregated_voting_power(*aggregated_weak_voting_power, true)
                        .is_ok()
                        || validator_verifier
                            .check_aggregated_voting_power(*aggregated_strong_voting_power, false)
                            .is_ok()
                },
                NodeStatus::Ordered(_) => {
                    error!("checking voting power for Ordered node");
                    true
                },
            })
            .unwrap_or(false)
    }

    fn reachable_filter(start: Vec<HashValue>) -> impl FnMut(&Arc<CertifiedNode>) -> bool {
        let mut reachable: HashSet<HashValue> = HashSet::from_iter(start);
        move |node| {
            if reachable.contains(&node.digest()) {
                for parent in node.parents() {
                    reachable.insert(*parent.metadata().digest());
                }
                true
            } else {
                false
            }
        }
    }

    pub fn reachable_mut(
        &mut self,
        from: &Arc<CertifiedNode>,
        until: Option<Round>,
    ) -> impl Iterator<Item = &mut NodeStatus> + use<'_> {
        let until = until.unwrap_or(self.lowest_round());
        let mut reachable_filter = Self::reachable_filter(vec![from.digest()]);
        self.nodes_by_round
            .range_mut(until..=from.round())
            .rev()
            .flat_map(|(_, round_ref)| round_ref.iter_mut())
            .flatten()
            .filter(move |node_status| {
                matches!(node_status, NodeStatus::Unordered { .. })
                    && reachable_filter(node_status.as_node())
            })
    }

    pub fn reachable<'a>(
        &self,
        targets: impl Iterator<Item = &'a NodeMetadata> + Clone,
        until: Option<Round>,
        // TODO: replace filter with bool to filter unordered
        filter: impl Fn(&NodeStatus) -> bool,
    ) -> impl Iterator<Item = &NodeStatus> {
        let until = until.unwrap_or(self.lowest_round());
        let initial_round = targets
            .clone()
            .map(|t| t.round())
            .max()
            .expect("Round should be not empty!");
        let initial = targets.map(|t| *t.digest()).collect();

        let mut reachable_filter = Self::reachable_filter(initial);
        self.nodes_by_round
            .range(until..=initial_round)
            .rev()
            .flat_map(|(_, round_ref)| round_ref.iter())
            .flatten()
            .filter(move |node_status| {
                filter(node_status) && reachable_filter(node_status.as_node())
            })
    }

    pub fn get_strong_links_for_round(
        &self,
        round: Round,
        validator_verifier: &ValidatorVerifier,
    ) -> Option<Vec<NodeCertificate>> {
        if validator_verifier
            .check_voting_power(
                self.get_round_iter(round)?
                    .map(|node_status| node_status.as_node().metadata().author()),
                true,
            )
            .is_ok()
        {
            Some(
                self.get_round_iter(round)?
                    .map(|node_status| node_status.as_node().certificate())
                    .collect(),
            )
        } else {
            None
        }
    }

    pub fn lowest_incomplete_round(&self) -> Round {
        if self.nodes_by_round.is_empty() {
            return self.lowest_round();
        }

        for (round, round_nodes) in &self.nodes_by_round {
            if round_nodes.iter().any(|node| node.is_none()) {
                return *round;
            }
        }

        self.highest_round() + 1
    }

    pub fn bitmask(&self, target_round: Round) -> DagSnapshotBitmask {
        let from_round = if self.is_empty() {
            self.lowest_round()
        } else {
            target_round
                .saturating_sub(self.window_size)
                .max(self.lowest_incomplete_round())
                .max(self.lowest_round())
        };
        let mut bitmask: Vec<_> = self
            .nodes_by_round
            .range(from_round..=target_round)
            .map(|(_, round_nodes)| round_nodes.iter().map(|node| node.is_some()).collect())
            .collect();

        bitmask.resize(
            (target_round - from_round + 1) as usize,
            vec![false; self.author_to_index.len()],
        );

        DagSnapshotBitmask::new(from_round, bitmask)
    }

    /// unwrap is only used in debug mode
    #[allow(clippy::unwrap_used)]
    pub(super) fn prune(&mut self) -> BTreeMap<u64, Vec<Option<NodeStatus>>> {
        let to_keep = self.nodes_by_round.split_off(&self.start_round);
        let to_prune = std::mem::replace(&mut self.nodes_by_round, to_keep);
        debug!(
            "pruning dag. start round {}. pruning from {}",
            self.start_round,
            to_prune.first_key_value().map(|v| v.0).unwrap()
        );
        to_prune
    }

    fn commit_callback(
        &mut self,
        commit_round: Round,
    ) -> Option<BTreeMap<u64, Vec<Option<NodeStatus>>>> {
        let new_start_round = commit_round.saturating_sub(3 * self.window_size);
        if new_start_round > self.start_round {
            self.start_round = new_start_round;
            return Some(self.prune());
        }
        None
    }

    pub(super) fn highest_ordered_anchor_round(&self) -> Option<Round> {
        for (round, round_nodes) in self.nodes_by_round.iter().rev() {
            for maybe_node_status in round_nodes {
                if matches!(maybe_node_status, Some(NodeStatus::Ordered(_))) {
                    return Some(*round);
                }
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.nodes_by_round.is_empty() && self.start_round > 1
    }
}

pub struct DagStore {
    dag: RwLock<InMemDag>,
    storage: Arc<dyn DAGStorage>,
    payload_manager: Arc<dyn TPayloadManager>,
}

impl DagStore {
    pub fn new(
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        payload_manager: Arc<dyn TPayloadManager>,
        start_round: Round,
        window_size: u64,
    ) -> Self {
        let mut all_nodes = storage.get_certified_nodes().unwrap_or_default();
        all_nodes.sort_unstable_by_key(|(_, node)| node.round());
        let mut to_prune = vec![];
        // Reconstruct the continuous dag starting from start_round and gc unrelated nodes
        let dag = Self::new_empty(
            epoch_state,
            storage.clone(),
            payload_manager,
            start_round,
            window_size,
        );
        for (digest, certified_node) in all_nodes {
            // TODO: save the storage call in this case
            if let Err(e) = dag.add_node(certified_node) {
                debug!("Delete node after bootstrap due to {}", e);
                to_prune.push(digest);
            }
        }
        if let Err(e) = storage.delete_certified_nodes(to_prune) {
            error!("Error deleting expired nodes: {:?}", e);
        }
        if dag.read().is_empty() {
            warn!(
                "[DAG] Start with empty DAG store at {}, need state sync",
                start_round
            );
        }
        dag
    }

    pub fn new_empty(
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        payload_manager: Arc<dyn TPayloadManager>,
        start_round: Round,
        window_size: u64,
    ) -> Self {
        let dag = InMemDag::new_empty(epoch_state, start_round, window_size);
        Self {
            dag: RwLock::new(dag),
            storage,
            payload_manager,
        }
    }

    pub fn new_for_test(
        dag: InMemDag,
        storage: Arc<dyn DAGStorage>,
        payload_manager: Arc<dyn TPayloadManager>,
    ) -> Self {
        Self {
            dag: RwLock::new(dag),
            storage,
            payload_manager,
        }
    }

    pub fn add_node(&self, node: CertifiedNode) -> anyhow::Result<()> {
        self.dag.write().validate_new_node(&node)?;

        // Note on concurrency: it is possible that a prune operation kicks in here and
        // moves the window forward making the `node` stale. Any stale node inserted
        // due to this race will be cleaned up with the next prune operation.

        // mutate after all checks pass
        self.storage.save_certified_node(&node)?;

        debug!("Added node {}", node.id());
        self.payload_manager.prefetch_payload_data(
            node.payload(),
            *node.author(),
            node.metadata().timestamp(),
        );

        self.dag.write().add_validated_node(node)
    }

    pub fn commit_callback(&self, commit_round: Round) {
        let to_prune = self.dag.write().commit_callback(commit_round);
        if let Some(to_prune) = to_prune {
            let digests = to_prune
                .iter()
                .flat_map(|(_, round_ref)| round_ref.iter().flatten())
                .map(|node_status| *node_status.as_node().metadata().digest())
                .collect();
            if let Err(e) = self.storage.delete_certified_nodes(digests) {
                error!("Error deleting expired nodes: {:?}", e);
            }
        }
    }
}

impl Deref for DagStore {
    type Target = RwLock<InMemDag>;

    fn deref(&self) -> &Self::Target {
        &self.dag
    }
}
