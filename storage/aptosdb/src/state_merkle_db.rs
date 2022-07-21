// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema;
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_jellyfish_merkle::{
    node_type::NodeKey, JellyfishMerkleTree, TreeReader, TreeUpdateBatch, TreeWriter,
};
use aptos_types::{
    nibble::{nibble_path::NibblePath, ROOT_NIBBLE_HEIGHT},
    proof::{SparseMerkleProof, SparseMerkleRangeProof},
    state_store::state_key::StateKey,
    transaction::Version,
};
use schemadb::{SchemaBatch, DB};
use std::{collections::HashMap, ops::Deref, sync::Arc};

pub(crate) type LeafNode = aptos_jellyfish_merkle::node_type::LeafNode<StateKey>;
pub(crate) type Node = aptos_jellyfish_merkle::node_type::Node<StateKey>;
type NodeBatch = aptos_jellyfish_merkle::NodeBatch<StateKey>;

#[derive(Debug)]
pub struct StateMerkleDb(Arc<DB>);

impl Deref for StateMerkleDb {
    type Target = DB;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StateMerkleDb {
    pub fn new(state_merkle_rocksdb: Arc<DB>) -> Self {
        Self(state_merkle_rocksdb)
    }

    pub fn get_with_proof(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<(HashValue, (StateKey, Version))>, SparseMerkleProof)> {
        JellyfishMerkleTree::new(self).get_with_proof(state_key.hash(), version)
    }

    pub fn get_range_proof(
        &self,
        rightmost_key: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        JellyfishMerkleTree::new(self).get_range_proof(rightmost_key, version)
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        JellyfishMerkleTree::new(self).get_root_hash(version)
    }

    pub fn get_leaf_count(&self, version: Version) -> Result<usize> {
        JellyfishMerkleTree::new(self).get_leaf_count(version)
    }

    pub fn batch_put_value_set(
        &self,
        value_set: Vec<(HashValue, &(HashValue, StateKey))>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(HashValue, TreeUpdateBatch<StateKey>)> {
        JellyfishMerkleTree::new(self).batch_put_value_set(
            value_set,
            node_hashes,
            persisted_version,
            version,
        )
    }

    pub fn get_state_snapshot_version_before(
        &self,
        next_version: Version,
    ) -> Result<Option<Version>> {
        if next_version > 0 {
            let max_possible_version = next_version - 1;
            let mut iter = self.rev_iter::<JellyfishMerkleNodeSchema>(Default::default())?;
            iter.seek_for_prev(&NodeKey::new_empty_path(max_possible_version))?;
            if let Some((key, _node)) = iter.next().transpose()? {
                // TODO: If we break up a single update batch to multiple commits, we would need to
                // deal with a partial version, which hasn't got the root committed.
                return Ok(Some(key.version()));
            }
        }
        // No version before genesis.
        Ok(None)
    }

    /// Finds the rightmost leaf by scanning the entire DB.
    #[cfg(test)]
    pub fn get_rightmost_leaf_naive(&self) -> Result<Option<(NodeKey, LeafNode)>> {
        let mut ret = None;

        let mut iter = self.iter::<JellyfishMerkleNodeSchema>(Default::default())?;
        iter.seek_to_first();

        while let Some((node_key, node)) = iter.next().transpose()? {
            if let Node::Leaf(leaf_node) = node {
                match ret {
                    None => ret = Some((node_key, leaf_node)),
                    Some(ref other) => {
                        if leaf_node.account_key() > other.1.account_key() {
                            ret = Some((node_key, leaf_node));
                        }
                    }
                }
            }
        }

        Ok(ret)
    }
}

impl TreeReader<StateKey> for StateMerkleDb {
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        self.get::<JellyfishMerkleNodeSchema>(node_key)
    }

    fn get_rightmost_leaf(&self) -> Result<Option<(NodeKey, LeafNode)>> {
        // Since everything has the same version during restore, we seek to the first node and get
        // its version.
        let mut iter = self.iter::<JellyfishMerkleNodeSchema>(Default::default())?;
        iter.seek_to_first();
        let version = match iter.next().transpose()? {
            Some((node_key, _node)) => node_key.version(),
            None => return Ok(None),
        };

        // The encoding of key and value in DB looks like:
        //
        // | <-------------- key --------------> | <- value -> |
        // | version | num_nibbles | nibble_path |    node     |
        //
        // Here version is fixed. For each num_nibbles, there could be a range of nibble paths
        // of the same length. If one of them is the rightmost leaf R, it must be at the end of this
        // range. Otherwise let's assume the R is in the middle of the range, so we
        // call the node at the end of this range X:
        //   1. If X is leaf, then X.account_key() > R.account_key(), because the nibble path is a
        //      prefix of the account key. So R is not the rightmost leaf.
        //   2. If X is internal node, then X must be on the right side of R, so all its children's
        //      account keys are larger than R.account_key(). So R is not the rightmost leaf.
        //
        // Given that num_nibbles ranges from 0 to ROOT_NIBBLE_HEIGHT, there are only
        // ROOT_NIBBLE_HEIGHT+1 ranges, so we can just find the node at the end of each range and
        // then pick the one with the largest account key.
        let mut ret = None;

        for num_nibbles in 1..=ROOT_NIBBLE_HEIGHT + 1 {
            let mut iter = self.iter::<JellyfishMerkleNodeSchema>(Default::default())?;
            // nibble_path is always non-empty except for the root, so if we use an empty nibble
            // path as the seek key, the iterator will end up pointing to the end of the previous
            // range.
            let seek_key = (version, num_nibbles as u8);
            iter.seek_for_prev(&seek_key)?;

            if let Some((node_key, node)) = iter.next().transpose()? {
                debug_assert_eq!(node_key.version(), version);
                debug_assert!(node_key.nibble_path().num_nibbles() < num_nibbles);

                if let Node::Leaf(leaf_node) = node {
                    match ret {
                        None => ret = Some((node_key, leaf_node)),
                        Some(ref other) => {
                            if leaf_node.account_key() > other.1.account_key() {
                                ret = Some((node_key, leaf_node));
                            }
                        }
                    }
                }
            }
        }

        Ok(ret)
    }
}

impl TreeWriter<StateKey> for StateMerkleDb {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> Result<()> {
        let mut batch = SchemaBatch::new();
        add_node_batch(&mut batch, node_batch.iter())?;
        self.write_schemas(batch)
    }
}

pub fn add_node_batch<'a>(
    batch: &'a mut SchemaBatch,
    mut node_batch: impl Iterator<Item = (&'a NodeKey, &'a Node)>,
) -> Result<()> {
    node_batch
        .try_for_each(|(node_key, node)| batch.put::<JellyfishMerkleNodeSchema>(node_key, node))
}
