// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::types::{DagSnapshotBitmask, NodeMetadata};
use crate::dag::{
    storage::DAGStorage,
    types::{CertifiedNode, NodeCertificate},
};
use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_logger::{debug, error};
use aptos_types::{epoch_state::EpochState, validator_verifier::ValidatorVerifier};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone)]
pub enum NodeStatus {
    Unordered(Arc<CertifiedNode>),
    Ordered(Arc<CertifiedNode>),
}

impl NodeStatus {
    pub fn as_node(&self) -> &Arc<CertifiedNode> {
        match self {
            NodeStatus::Unordered(node) | NodeStatus::Ordered(node) => node,
        }
    }

    pub fn mark_as_ordered(&mut self) {
        assert!(matches!(self, NodeStatus::Unordered(_)));
        *self = NodeStatus::Ordered(self.as_node().clone());
    }
}
/// Data structure that stores the DAG representation, it maintains round based index.
#[derive(Clone)]
pub struct Dag {
    nodes_by_round: BTreeMap<Round, Vec<Option<NodeStatus>>>,
    /// Map between peer id to vector index
    author_to_index: HashMap<Author, usize>,
    storage: Arc<dyn DAGStorage>,
    initial_round: Round,
    epoch_state: Arc<EpochState>,
}

impl Dag {
    pub fn new(
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        initial_round: Round,
        _dag_window_size_config: usize,
    ) -> Self {
        let epoch = epoch_state.epoch;
        let author_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let num_validators = author_to_index.len();
        let all_nodes = storage.get_certified_nodes().unwrap_or_default();
        let mut expired = vec![];
        let mut nodes_by_round = BTreeMap::new();
        for (digest, certified_node) in all_nodes {
            if certified_node.metadata().epoch() == epoch && certified_node.round() >= initial_round
            {
                let arc_node = Arc::new(certified_node);
                let index = *author_to_index
                    .get(arc_node.metadata().author())
                    .expect("Author from certified node should exist");
                let round = arc_node.metadata().round();
                debug!("Recovered node {} from storage", arc_node.id());
                nodes_by_round
                    .entry(round)
                    .or_insert_with(|| vec![None; num_validators])[index] =
                    Some(NodeStatus::Unordered(arc_node));
            } else {
                expired.push(digest);
            }
        }
        if let Err(e) = storage.delete_certified_nodes(expired) {
            error!("Error deleting expired nodes: {:?}", e);
        }
        Self {
            nodes_by_round,
            author_to_index,
            storage,
            initial_round,
            epoch_state,
        }
    }

    pub fn new_empty(
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        initial_round: Round,
    ) -> Self {
        let author_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let nodes_by_round = BTreeMap::new();
        Self {
            nodes_by_round,
            author_to_index,
            storage,
            initial_round,
            epoch_state,
        }
    }

    pub(crate) fn lowest_round(&self) -> Round {
        *self
            .nodes_by_round
            .first_key_value()
            .map(|(round, _)| round)
            .unwrap_or(&self.initial_round)
    }

    pub fn highest_round(&self) -> Round {
        *self
            .nodes_by_round
            .last_key_value()
            .map(|(round, _)| round)
            .unwrap_or(&self.initial_round)
    }

    /// The highest strong links round is either the highest round or the highest round - 1
    /// because we ensure all parents (strong links) exist for any nodes in the store
    pub fn highest_strong_links_round(&self, validator_verifier: &ValidatorVerifier) -> Round {
        let highest_round = self.highest_round();
        self.get_strong_links_for_round(highest_round, validator_verifier)
            .map_or_else(|| highest_round.saturating_sub(1), |_| highest_round)
    }

    pub fn add_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let node = Arc::new(node);
        let author = node.metadata().author();
        let index = *self
            .author_to_index
            .get(author)
            .ok_or_else(|| anyhow!("unknown author"))?;
        let round = node.metadata().round();
        ensure!(round >= self.lowest_round(), "round too low");
        ensure!(round <= self.highest_round() + 1, "round too high");
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

        // mutate after all checks pass
        self.storage.save_certified_node(&node)?;
        debug!("Added node {}", node.id());
        round_ref[index] = Some(NodeStatus::Unordered(node.clone()));
        Ok(())
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

    pub fn get_node_ref_mut(&mut self, round: Round, author: &Author) -> Option<&mut NodeStatus> {
        let index = self.author_to_index.get(author)?;
        let round_ref = self.nodes_by_round.get_mut(&round)?;
        round_ref[*index].as_mut()
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

    // TODO: I think we can cache votes in the NodeStatus::Unordered
    pub fn check_votes_for_node(
        &self,
        metadata: &NodeMetadata,
        validator_verifier: &ValidatorVerifier,
    ) -> bool {
        self.get_round_iter(metadata.round() + 1)
            .map(|next_round_iter| {
                let votes = next_round_iter
                    .filter(|node_status| {
                        node_status
                            .as_node()
                            .parents()
                            .iter()
                            .any(|cert| cert.metadata() == metadata)
                    })
                    .map(|node_status| node_status.as_node().author());
                validator_verifier.check_voting_power(votes, false).is_ok()
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
    ) -> impl Iterator<Item = &mut NodeStatus> {
        let until = until.unwrap_or(self.lowest_round());
        let mut reachable_filter = Self::reachable_filter(vec![from.digest()]);
        self.nodes_by_round
            .range_mut(until..=from.round())
            .rev()
            .flat_map(|(_, round_ref)| round_ref.iter_mut())
            .flatten()
            .filter(move |node_status| {
                matches!(node_status, NodeStatus::Unordered(_))
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
        let initial_round = targets.clone().map(|t| t.round()).max().unwrap();
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
        let lowest_round = self.lowest_incomplete_round();

        let bitmask = self
            .nodes_by_round
            .range(lowest_round..target_round)
            .map(|(_, round_nodes)| round_nodes.iter().map(|node| node.is_some()).collect())
            .collect();

        DagSnapshotBitmask::new(lowest_round, bitmask)
    }

    pub(super) fn prune(&mut self) {
        let all_nodes = self.storage.get_certified_nodes().unwrap_or_default();
        let mut expired = vec![];
        for (digest, certified_node) in all_nodes {
            if certified_node.metadata().epoch() != self.epoch_state.epoch
                || certified_node.metadata().round() < self.initial_round
            {
                expired.push(digest);
                self.nodes_by_round
                    .remove(&certified_node.metadata().round());
            }
        }
        if let Err(e) = self.storage.delete_certified_nodes(expired) {
            error!("Error deleting expired nodes: {:?}", e);
        }
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
}
