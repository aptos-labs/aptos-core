// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    storage::DAGStorage,
    types::{CertifiedNode, NodeCertificate},
};
use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_logger::error;
use aptos_types::{epoch_state::EpochState, validator_verifier::ValidatorVerifier};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

/// Data structure that stores the DAG representation, it maintains both hash based index and
/// round based index.
pub struct Dag {
    nodes_by_digest: HashMap<HashValue, Arc<CertifiedNode>>,
    nodes_by_round: BTreeMap<Round, Vec<Option<Arc<CertifiedNode>>>>,
    /// Map between peer id to vector index
    author_to_index: HashMap<Author, usize>,
    storage: Arc<dyn DAGStorage>,
}

impl Dag {
    pub fn new(epoch_state: Arc<EpochState>, storage: Arc<dyn DAGStorage>) -> Self {
        let epoch = epoch_state.epoch;
        let author_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let num_validators = author_to_index.len();
        let all_nodes = storage.get_certified_nodes().unwrap_or_default();
        let mut expired = vec![];
        let mut nodes_by_digest = HashMap::new();
        let mut nodes_by_round = BTreeMap::new();
        for (digest, certified_node) in all_nodes {
            if certified_node.metadata().epoch() == epoch {
                let arc_node = Arc::new(certified_node);
                nodes_by_digest.insert(digest, arc_node.clone());
                let index = *author_to_index
                    .get(arc_node.metadata().author())
                    .expect("Author from certified node should exist");
                let round = arc_node.metadata().round();
                nodes_by_round
                    .entry(round)
                    .or_insert_with(|| vec![None; num_validators])[index] = Some(arc_node);
            } else {
                expired.push(digest);
            }
        }
        if let Err(e) = storage.delete_certified_nodes(expired) {
            error!("Error deleting expired nodes: {:?}", e);
        }
        Self {
            nodes_by_digest,
            nodes_by_round,
            author_to_index,
            storage,
        }
    }

    pub(crate) fn lowest_round(&self) -> Round {
        *self
            .nodes_by_round
            .first_key_value()
            .map(|(round, _)| round)
            .unwrap_or(&0)
    }

    pub fn highest_round(&self) -> Round {
        *self
            .nodes_by_round
            .last_key_value()
            .map(|(round, _)| round)
            .unwrap_or(&0)
    }

    pub fn add_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let node = Arc::new(node);
        let index = *self
            .author_to_index
            .get(node.metadata().author())
            .ok_or_else(|| anyhow!("unknown author"))?;
        let round = node.metadata().round();
        ensure!(round >= self.lowest_round(), "round too low");
        ensure!(round <= self.highest_round() + 1, "round too high");
        for parent in node.parents() {
            ensure!(self.exists(parent.metadata().digest()), "parent not exist");
        }
        self.storage.save_certified_node(&node)?;
        ensure!(
            self.nodes_by_digest
                .insert(node.digest(), node.clone())
                .is_none(),
            "duplicate node"
        );
        let round_ref = self
            .nodes_by_round
            .entry(round)
            .or_insert_with(|| vec![None; self.author_to_index.len()]);
        ensure!(round_ref[index].is_none(), "equivocate node");
        round_ref[index] = Some(node);
        Ok(())
    }

    pub fn exists(&self, digest: &HashValue) -> bool {
        self.nodes_by_digest.contains_key(digest)
    }

    pub fn all_exists(&self, nodes: &[NodeCertificate]) -> bool {
        nodes.iter().all(|certificate| {
            self.nodes_by_digest
                .contains_key(certificate.metadata().digest())
        })
    }

    pub fn get_node(&self, digest: &HashValue) -> Option<Arc<CertifiedNode>> {
        self.nodes_by_digest.get(digest).cloned()
    }

    pub fn get_strong_links_for_round(
        &self,
        round: Round,
        validator_verifier: &ValidatorVerifier,
    ) -> Option<Vec<NodeCertificate>> {
        let all_nodes_in_round = self.nodes_by_round.get(&round)?.iter().flatten();
        if validator_verifier
            .check_voting_power(
                all_nodes_in_round
                    .clone()
                    .map(|node| node.metadata().author()),
            )
            .is_ok()
        {
            Some(all_nodes_in_round.map(|node| node.certificate()).collect())
        } else {
            None
        }
    }

    pub fn bitmask(&self) -> Vec<Vec<bool>> {
        // TODO: extract local bitvec
        todo!();
    }
}
