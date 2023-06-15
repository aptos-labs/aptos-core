// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::types::{CertifiedNode, NodeMetadata};
use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
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
}

impl Dag {
    pub fn new(author_to_index: HashMap<Author, usize>, initial_round: Round) -> Self {
        let mut nodes_by_round = BTreeMap::new();
        let num_nodes = author_to_index.len();
        nodes_by_round.insert(initial_round, vec![None; num_nodes]);
        Self {
            nodes_by_digest: HashMap::new(),
            nodes_by_round,
            author_to_index,
        }
    }

    fn lowest_round(&self) -> Round {
        *self
            .nodes_by_round
            .first_key_value()
            .map(|(round, _)| round)
            .unwrap_or(&0)
    }

    fn highest_round(&self) -> Round {
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
            ensure!(self.exists(parent.digest()), "parent not exist");
        }
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

    pub fn all_exists<'a>(&self, mut digests: impl Iterator<Item = &'a HashValue>) -> bool {
        digests.all(|digest| self.nodes_by_digest.contains_key(digest))
    }

    pub fn get_node(&self, digest: &HashValue) -> Option<Arc<CertifiedNode>> {
        self.nodes_by_digest.get(digest).cloned()
    }

    pub fn get_strong_links_for_round(
        &self,
        round: Round,
        validator_verifier: &ValidatorVerifier,
    ) -> Option<Vec<NodeMetadata>> {
        let all_nodes_in_round = self.nodes_by_round.get(&round)?.iter().flatten();
        if validator_verifier
            .check_voting_power(
                all_nodes_in_round
                    .clone()
                    .map(|node| node.metadata().author()),
            )
            .is_ok()
        {
            Some(
                all_nodes_in_round
                    .map(|node| node.metadata().clone())
                    .collect(),
            )
        } else {
            None
        }
    }
}
