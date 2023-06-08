// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_types::{aggregate_signature::AggregateSignature, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

/// Represents the metadata about the node, without payload and parents from Node
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    epoch: u64,
    round: Round,
    author: Author,
    timestamp: u64,
    digest: HashValue,
}

/// Node representation in the DAG, parents contain 2f+1 strong links (links to previous round)
/// plus weak links (links to lower round)
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

    pub fn metadata(&self) -> NodeMetadata {
        self.metadata.clone()
    }
}

/// Quorum signatures over the node digest
#[derive(Clone)]
pub struct NodeCertificate {
    digest: HashValue,
    signatures: AggregateSignature,
}

impl NodeCertificate {
    pub fn new(digest: HashValue, signatures: AggregateSignature) -> Self {
        Self { digest, signatures }
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

/// Data structure that stores the DAG representation, it maintains both hash based index and
/// round based index.
pub struct Dag {
    nodes_by_digest: HashMap<HashValue, Arc<CertifiedNode>>,
    nodes_by_round: BTreeMap<Round, Vec<Option<Arc<CertifiedNode>>>>,
    /// Map between peer id to vector index
    author_to_index: HashMap<Author, usize>,
    /// Highest head nodes that are not linked by other nodes
    highest_unlinked_nodes_by_author: Vec<Option<Arc<CertifiedNode>>>,
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
            highest_unlinked_nodes_by_author: vec![None; num_nodes],
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
            .get(&node.metadata.author)
            .ok_or_else(|| anyhow!("unknown author"))?;
        let round = node.metadata.round;
        ensure!(round >= self.lowest_round(), "round too low");
        ensure!(round <= self.highest_round() + 1, "round too high");
        for parent in &node.parents {
            ensure!(self.exists(&parent.digest), "parent not exist");
        }
        ensure!(
            self.nodes_by_digest
                .insert(node.metadata.digest, node.clone())
                .is_none(),
            "duplicate node"
        );
        ensure!(
            self.nodes_by_round
                .entry(round)
                .or_insert_with(|| vec![None; self.author_to_index.len()])[index]
                .replace(node.clone())
                .is_none(),
            "equivocate node"
        );
        if round
            > self.highest_unlinked_nodes_by_author[index]
                .as_ref()
                .map_or(0, |node| node.metadata.round)
        {
            self.highest_unlinked_nodes_by_author[index].replace(node);
        }
        Ok(())
    }

    pub fn exists(&self, digest: &HashValue) -> bool {
        self.nodes_by_digest.contains_key(digest)
    }

    pub fn get_node(&self, digest: &HashValue) -> Option<Arc<CertifiedNode>> {
        self.nodes_by_digest.get(digest).cloned()
    }

    pub fn get_unlinked_nodes_for_new_round(
        &self,
        validator_verifier: &ValidatorVerifier,
    ) -> Option<Vec<NodeMetadata>> {
        let current_round = self.highest_round();
        let strong_link_authors =
            self.highest_unlinked_nodes_by_author
                .iter()
                .filter_map(|maybe_node| {
                    maybe_node.as_ref().and_then(|node| {
                        if node.metadata.round == current_round {
                            Some(&node.metadata.author)
                        } else {
                            None
                        }
                    })
                });
        if validator_verifier
            .check_voting_power(strong_link_authors)
            .is_ok()
        {
            Some(
                self.highest_unlinked_nodes_by_author
                    .iter()
                    .filter_map(|maybe_node| maybe_node.as_ref().map(|node| node.metadata.clone()))
                    .collect(),
            )
        } else {
            None
        }
    }

    pub fn mark_nodes_linked(&mut self, node_metadata: &[NodeMetadata]) {
        let digests: HashSet<_> = node_metadata.iter().map(|node| node.digest).collect();
        for maybe_node in &mut self.highest_unlinked_nodes_by_author {
            if let Some(node) = maybe_node {
                if digests.contains(&node.metadata.digest) {
                    *maybe_node = None;
                }
            }
        }
    }
}
