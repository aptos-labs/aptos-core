// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sparse_merkle::utils::partition;
use velor_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use velor_types::proof::{
    definition::NodeInProof, SparseMerkleInternalNode, SparseMerkleLeafNode, SparseMerkleProofExt,
};
use bitvec::prelude::*;
use dashmap::DashMap;
use std::collections::BTreeMap;

type Cache = DashMap<BitVec<u8, Msb0>, NodeInProof>;

struct NaiveSubTree<'a> {
    leaves: &'a [(HashValue, HashValue)],
    depth: usize,
}

impl<'a> NaiveSubTree<'a> {
    fn get_proof(
        &'a self,
        key: &HashValue,
        cache: &Cache,
    ) -> (Option<SparseMerkleLeafNode>, Vec<NodeInProof>) {
        let (leaf, rev_proof) = self.get_proof_(key, cache);
        (leaf, rev_proof.into_iter().rev().collect())
    }

    fn get_proof_(
        &'a self,
        key: &HashValue,
        cache: &Cache,
    ) -> (Option<SparseMerkleLeafNode>, Vec<NodeInProof>) {
        if self.is_empty() {
            (None, Vec::new())
        } else if self.leaves.len() == 1 {
            let only_leaf = self.leaves[0];
            (
                Some(SparseMerkleLeafNode::new(only_leaf.0, only_leaf.1)),
                Vec::new(),
            )
        } else {
            let (left, right) = self.children();
            if key.bit(self.depth) {
                let (ret_leaf, mut ret_siblings) = right.get_proof_(key, cache);
                ret_siblings.push(left.get_node_in_proof(cache));
                (ret_leaf, ret_siblings)
            } else {
                let (ret_leaf, mut ret_siblings) = left.get_proof_(key, cache);
                ret_siblings.push(right.get_node_in_proof(cache));
                (ret_leaf, ret_siblings)
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    fn get_node_in_proof(&self, cache: &Cache) -> NodeInProof {
        if self.leaves.is_empty() {
            return NodeInProof::from(*SPARSE_MERKLE_PLACEHOLDER_HASH);
        }

        let position = self.leaves[0]
            .0
            .view_bits()
            .split_at(self.depth)
            .0
            .to_bitvec();

        match cache.get(&position) {
            Some(node) => *node,
            None => {
                let node = self.get_node_in_proof_uncached(cache);
                cache.insert(position, node);
                node
            },
        }
    }

    fn get_node_in_proof_uncached(&self, cache: &Cache) -> NodeInProof {
        assert!(!self.leaves.is_empty());
        if self.leaves.len() == 1 {
            let only_leaf = self.leaves[0];
            SparseMerkleLeafNode::new(only_leaf.0, only_leaf.1).into()
        } else {
            let (left, right) = self.children();
            SparseMerkleInternalNode::new(
                left.get_node_in_proof(cache).hash(),
                right.get_node_in_proof(cache).hash(),
            )
            .hash()
            .into()
        }
    }

    fn children(&self) -> (Self, Self) {
        let pivot = partition(self.leaves, self.depth);
        let (left, right) = self.leaves.split_at(pivot);
        (
            Self {
                leaves: left,
                depth: self.depth + 1,
            },
            Self {
                leaves: right,
                depth: self.depth + 1,
            },
        )
    }
}

#[derive(Clone, Default)]
pub struct NaiveSmt {
    pub leaves: Vec<(HashValue, HashValue)>,
    cache: Cache,
}

impl NaiveSmt {
    pub fn new(leaves: &[(HashValue, HashValue)]) -> Self {
        Self::default().update(
            leaves
                .iter()
                .map(|(k, v)| (*k, Some(*v)))
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }

    pub fn update(self, updates: &[(HashValue, Option<HashValue>)]) -> Self {
        let mut leaves = self.leaves.into_iter().collect::<BTreeMap<_, _>>();
        for (key, value_option) in updates {
            if let Some(value) = value_option {
                leaves.insert(*key, *value);
            } else {
                leaves.remove(key);
            }
        }

        Self {
            leaves: leaves.into_iter().collect::<Vec<_>>(),
            cache: Cache::new(),
        }
    }

    pub fn get_proof(&self, key: &HashValue) -> SparseMerkleProofExt {
        let root = NaiveSubTree {
            leaves: &self.leaves,
            depth: 0,
        };

        let (leaf, siblings) = root.get_proof(key, &self.cache);
        SparseMerkleProofExt::new(leaf, siblings)
    }

    pub fn get_root_hash(&self) -> HashValue {
        let root = NaiveSubTree {
            leaves: &self.leaves,
            depth: 0,
        };

        root.get_node_in_proof(&self.cache).hash()
    }
}
