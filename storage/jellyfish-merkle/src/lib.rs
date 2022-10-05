// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This module implements [`JellyfishMerkleTree`] backed by storage module. The tree itself doesn't
//! persist anything, but realizes the logic of R/W only. The write path will produce all the
//! intermediate results in a batch for storage layer to commit and the read path will return
//! results directly. The public APIs are only [`new`], [`batch_put_value_set`], and
//! [`get_with_proof`]. After each put with a `value_set` based on a known version, the tree will
//! return a new root hash with a [`TreeUpdateBatch`] containing all the new nodes and indices of
//! stale nodes.
//!
//! A Jellyfish Merkle Tree itself logically is a 256-bit sparse Merkle tree with an optimization
//! that any subtree containing 0 or 1 leaf node will be replaced by that leaf node or a placeholder
//! node with default hash value. With this optimization we can save CPU by avoiding hashing on
//! many sparse levels in the tree. Physically, the tree is structurally similar to the modified
//! Patricia Merkle tree of Ethereum but with some modifications. A standard Jellyfish Merkle tree
//! will look like the following figure:
//!
//! ```text
//!                                    .──────────────────────.
//!                            _.─────'                        `──────.
//!                       _.──'                                        `───.
//!                   _.─'                                                  `──.
//!               _.─'                                                          `──.
//!             ,'                                                                  `.
//!          ,─'                                                                      '─.
//!        ,'                                                                            `.
//!      ,'                                                                                `.
//!     ╱                                                                                    ╲
//!    ╱                                                                                      ╲
//!   ╱                                                                                        ╲
//!  ╱                                                                                          ╲
//! ;                                                                                            :
//! ;                                                                                            :
//!;                                                                                              :
//!│                                                                                              │
//!+──────────────────────────────────────────────────────────────────────────────────────────────+
//! .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.  .''.
//!/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \/    \
//!+----++----++----++----++----++----++----++----++----++----++----++----++----++----++----++----+
//! (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (
//!  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )
//! (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (
//!  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )
//! (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (
//!  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )
//! (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (
//!  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )  )
//! (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (  (
//! ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■  ■
//! ■: the [`Value`] type this tree stores.
//! ```
//!
//! A Jellyfish Merkle Tree consists of [`InternalNode`] and [`LeafNode`]. [`InternalNode`] is like
//! branch node in ethereum patricia merkle with 16 children to represent a 4-level binary tree and
//! [`LeafNode`] is similar to that in patricia merkle too. In the above figure, each `bell` in the
//! jellyfish is an [`InternalNode`] while each tentacle is a [`LeafNode`]. It is noted that
//! Jellyfish merkle doesn't have a counterpart for `extension` node of ethereum patricia merkle.
//!
//! [`JellyfishMerkleTree`]: struct.JellyfishMerkleTree.html
//! [`new`]: struct.JellyfishMerkleTree.html#method.new
//! [`put_value_sets`]: struct.JellyfishMerkleTree.html#method.put_value_sets
//! [`put_value_set`]: struct.JellyfishMerkleTree.html#method.put_value_set
//! [`get_with_proof`]: struct.JellyfishMerkleTree.html#method.get_with_proof
//! [`TreeUpdateBatch`]: struct.TreeUpdateBatch.html
//! [`InternalNode`]: node_type/struct.InternalNode.html
//! [`LeafNode`]: node_type/struct.LeafNode.html

pub mod iterator;
#[cfg(test)]
mod jellyfish_merkle_test;
pub mod metrics;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_tree_store;
pub mod node_type;
pub mod restore;
#[cfg(any(test, feature = "fuzzing"))]
pub mod test_helper;

use crate::metrics::APTOS_JELLYFISH_LEAF_COUNT;
use anyhow::{bail, ensure, format_err, Result};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_types::{
    nibble::{nibble_path::NibblePath, Nibble, ROOT_NIBBLE_HEIGHT},
    proof::{SparseMerkleProof, SparseMerkleProofExt, SparseMerkleRangeProof},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use node_type::{Child, Children, InternalNode, LeafNode, Node, NodeKey};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::arbitrary::Arbitrary;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    marker::PhantomData,
};
use thiserror::Error;

const MAX_PARALLELIZABLE_DEPTH: usize = 2;
const NUM_IO_THREADS: usize = 32;

pub static IO_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPoolBuilder::new()
        .num_threads(NUM_IO_THREADS)
        .thread_name(|index| format!("jmt-io-{}", index))
        .build()
        .unwrap()
});

#[derive(Error, Debug)]
#[error("Missing state root node at version {version}, probably pruned.")]
pub struct MissingRootError {
    pub version: Version,
}

/// `TreeReader` defines the interface between
/// [`JellyfishMerkleTree`](struct.JellyfishMerkleTree.html)
/// and underlying storage holding nodes.
pub trait TreeReader<K> {
    /// Gets node given a node key. Returns error if the node does not exist.
    fn get_node(&self, node_key: &NodeKey) -> Result<Node<K>> {
        self.get_node_option(node_key)?
            .ok_or_else(|| format_err!("Missing node at {:?}.", node_key))
    }

    /// Gets node given a node key. Returns `None` if the node does not exist.
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node<K>>>;

    /// Gets the rightmost leaf at a version. Note that this assumes we are in the process of
    /// restoring the tree and all nodes are at the same version.
    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode<K>)>>;
}

pub trait TreeWriter<K>: Send + Sync {
    /// Writes a node batch into storage.
    fn write_node_batch(&self, node_batch: &HashMap<NodeKey, Node<K>>) -> Result<()>;
}

pub trait Key: Clone + Serialize + DeserializeOwned + Send + Sync + 'static {
    fn key_size(&self) -> usize;
}

/// `Value` defines the types of data that can be stored in a Jellyfish Merkle tree.
pub trait Value: Clone + CryptoHash + Serialize + DeserializeOwned + Send + Sync {
    fn value_size(&self) -> usize;
}

/// `TestKey` defines the types of data that can be stored in a Jellyfish Merkle tree and used in
/// tests.
#[cfg(any(test, feature = "fuzzing"))]
pub trait TestKey:
    Key + Arbitrary + std::fmt::Debug + Eq + Hash + Ord + PartialOrd + PartialEq + 'static
{
}

/// `TestValue` defines the types of data that can be stored in a Jellyfish Merkle tree and used in
/// tests.
#[cfg(any(test, feature = "fuzzing"))]
pub trait TestValue: Value + Arbitrary + std::fmt::Debug + Eq + PartialEq + 'static {}

impl Key for StateKey {
    fn key_size(&self) -> usize {
        self.size()
    }
}

impl Value for StateValue {
    fn value_size(&self) -> usize {
        self.size()
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl TestKey for StateKey {}

/// Node batch that will be written into db atomically with other batches.
pub type NodeBatch<K> = HashMap<NodeKey, Node<K>>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeStats {
    pub new_nodes: usize,
    pub new_leaves: usize,
    pub stale_nodes: usize,
    pub stale_leaves: usize,
}

/// Indicates a node becomes stale since `stale_since_version`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct StaleNodeIndex {
    /// The version since when the node is overwritten and becomes stale.
    pub stale_since_version: Version,
    /// The [`NodeKey`](node_type/struct.NodeKey.html) identifying the node associated with this
    /// record.
    pub node_key: NodeKey,
}

/// This is a wrapper of [`NodeBatch`](type.NodeBatch.html),
/// [`StaleNodeIndexBatch`](type.StaleNodeIndexBatch.html) and some stats of nodes that represents
/// the incremental updates of a tree and pruning indices after applying a write set,
/// which is a vector of `hashed_account_address` and `new_value` pairs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TreeUpdateBatch<K> {
    pub node_batch: Vec<Vec<(NodeKey, Node<K>)>>,
    pub stale_node_index_batch: Vec<Vec<StaleNodeIndex>>,
    pub num_new_leaves: usize,
    pub num_stale_leaves: usize,
}

impl<K> TreeUpdateBatch<K>
where
    K: Key,
{
    pub fn new() -> Self {
        Self {
            node_batch: vec![vec![]],
            stale_node_index_batch: vec![vec![]],
            num_new_leaves: 0,
            num_stale_leaves: 0,
        }
    }

    pub fn combine(&mut self, other: Self) {
        let Self {
            node_batch,
            stale_node_index_batch,
            num_new_leaves,
            num_stale_leaves,
        } = other;

        self.node_batch.extend(node_batch);
        self.stale_node_index_batch.extend(stale_node_index_batch);
        self.num_new_leaves += num_new_leaves;
        self.num_stale_leaves += num_stale_leaves;
    }

    #[cfg(test)]
    pub fn num_stale_node(&self) -> usize {
        self.stale_node_index_batch.iter().map(Vec::len).sum()
    }

    fn inc_num_new_leaves(&mut self) {
        self.num_new_leaves += 1;
    }

    fn inc_num_stale_leaves(&mut self) {
        self.num_stale_leaves += 1;
    }

    pub fn put_node(&mut self, node_key: NodeKey, node: Node<K>) {
        if node.is_leaf() {
            self.inc_num_new_leaves();
        }
        self.node_batch[0].push((node_key, node))
    }

    pub fn put_stale_node(
        &mut self,
        node_key: NodeKey,
        stale_since_version: Version,
        node: &Node<K>,
    ) {
        if node.is_leaf() {
            self.inc_num_stale_leaves();
        }
        self.stale_node_index_batch[0].push(StaleNodeIndex {
            node_key,
            stale_since_version,
        });
    }
}

/// An iterator that iterates the index range (inclusive) of each different nibble at given
/// `nibble_idx` of all the keys in a sorted key-value pairs which have the identical HashValue
/// prefix (up to nibble_idx).
struct NibbleRangeIterator<'a, K> {
    sorted_kvs: &'a [(HashValue, K)],
    nibble_idx: usize,
    pos: usize,
}

impl<'a, K> NibbleRangeIterator<'a, K> {
    fn new(sorted_kvs: &'a [(HashValue, K)], nibble_idx: usize) -> Self {
        assert!(nibble_idx < ROOT_NIBBLE_HEIGHT);
        NibbleRangeIterator {
            sorted_kvs,
            nibble_idx,
            pos: 0,
        }
    }
}

impl<'a, K> std::iter::Iterator for NibbleRangeIterator<'a, K> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let left = self.pos;
        if self.pos < self.sorted_kvs.len() {
            let cur_nibble: u8 = self.sorted_kvs[left].0.nibble(self.nibble_idx);
            let (mut i, mut j) = (left, self.sorted_kvs.len() - 1);
            // Find the last index of the cur_nibble.
            while i < j {
                let mid = j - (j - i) / 2;
                if self.sorted_kvs[mid].0.nibble(self.nibble_idx) > cur_nibble {
                    j = mid - 1;
                } else {
                    i = mid;
                }
            }
            self.pos = i + 1;
            Some((left, i))
        } else {
            None
        }
    }
}

/// The Jellyfish Merkle tree data structure. See [`crate`] for description.
pub struct JellyfishMerkleTree<'a, R, K> {
    reader: &'a R,
    phantom_value: PhantomData<K>,
}

impl<'a, R, K> JellyfishMerkleTree<'a, R, K>
where
    R: 'a + TreeReader<K> + Sync,
    K: Key,
{
    /// Creates a `JellyfishMerkleTree` backed by the given [`TreeReader`](trait.TreeReader.html).
    pub fn new(reader: &'a R) -> Self {
        Self {
            reader,
            phantom_value: PhantomData,
        }
    }

    /// Get the node hash from the cache if cache is provided, otherwise (for test only) compute it.
    fn get_hash(
        node_key: &NodeKey,
        node: &Node<K>,
        hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
    ) -> HashValue {
        if let Some(cache) = hash_cache {
            match cache.get(node_key.nibble_path()) {
                Some(hash) => *hash,
                None => unreachable!("{:?} can not be found in hash cache", node_key),
            }
        } else {
            node.hash()
        }
    }

    /// For each value set:
    /// Returns the new nodes and values in a batch after applying `value_set`. For
    /// example, if after transaction `T_i` the committed state of tree in the persistent storage
    /// looks like the following structure:
    ///
    /// ```text
    ///              S_i
    ///             /   \
    ///            .     .
    ///           .       .
    ///          /         \
    ///         o           x
    ///        / \
    ///       A   B
    ///        storage (disk)
    /// ```
    ///
    /// where `A` and `B` denote the states of two adjacent accounts, and `x` is a sibling subtree
    /// of the path from root to A and B in the tree. Then a `value_set` produced by the next
    /// transaction `T_{i+1}` modifies other accounts `C` and `D` exist in the subtree under `x`, a
    /// new partial tree will be constructed in memory and the structure will be:
    ///
    /// ```text
    ///                 S_i      |      S_{i+1}
    ///                /   \     |     /       \
    ///               .     .    |    .         .
    ///              .       .   |   .           .
    ///             /         \  |  /             \
    ///            /           x | /               x'
    ///           o<-------------+-               / \
    ///          / \             |               C   D
    ///         A   B            |
    ///           storage (disk) |    cache (memory)
    /// ```
    ///
    /// With this design, we are able to query the global state in persistent storage and
    /// generate the proposed tree delta based on a specific root hash and `value_set`. For
    /// example, if we want to execute another transaction `T_{i+1}'`, we can use the tree `S_i` in
    /// storage and apply the `value_set` of transaction `T_{i+1}`. Then if the storage commits
    /// the returned batch, the state `S_{i+1}` is ready to be read from the tree by calling
    /// [`get_with_proof`](struct.JellyfishMerkleTree.html#method.get_with_proof). Anything inside
    /// the batch is not reachable from public interfaces before being committed.
    pub fn batch_put_value_set(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, K)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(HashValue, TreeUpdateBatch<K>)> {
        let deduped_and_sorted_kvs = value_set
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect::<Vec<_>>();

        let mut batch = TreeUpdateBatch::new();
        let root_node_opt = if let Some(persisted_version) = persisted_version {
            IO_POOL.install(|| {
                self.batch_insert_at(
                    &NodeKey::new_empty_path(persisted_version),
                    version,
                    deduped_and_sorted_kvs.as_slice(),
                    0,
                    &node_hashes,
                    &mut batch,
                )
            })?
        } else {
            self.batch_update_subtree(
                &NodeKey::new_empty_path(version),
                version,
                deduped_and_sorted_kvs.as_slice(),
                0,
                &node_hashes,
                &mut batch,
            )?
        };

        let node_key = NodeKey::new_empty_path(version);
        let root_hash = if let Some(root_node) = root_node_opt {
            APTOS_JELLYFISH_LEAF_COUNT.set(root_node.leaf_count() as i64);
            let hash = root_node.hash();
            batch.put_node(node_key, root_node);
            hash
        } else {
            APTOS_JELLYFISH_LEAF_COUNT.set(0);
            batch.put_node(node_key, Node::Null);
            *SPARSE_MERKLE_PLACEHOLDER_HASH
        };

        Ok((root_hash, batch))
    }

    fn batch_insert_at(
        &self,
        node_key: &NodeKey,
        version: Version,
        kvs: &[(HashValue, Option<&(HashValue, K)>)],
        depth: usize,
        hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
        batch: &mut TreeUpdateBatch<K>,
    ) -> Result<Option<Node<K>>> {
        let node = self.reader.get_node(node_key)?;
        batch.put_stale_node(node_key.clone(), version, &node);

        match node {
            Node::Internal(internal_node) => {
                // There is a small possibility that the old internal node is intact.
                // Traverse all the path touched by `kvs` from this internal node.
                let range_iter = NibbleRangeIterator::new(kvs, depth);
                let new_children: Vec<_> = if depth <= MAX_PARALLELIZABLE_DEPTH {
                    range_iter
                        .collect::<Vec<_>>()
                        .par_iter()
                        .map(|(left, right)| {
                            let mut sub_batch = TreeUpdateBatch::new();
                            Ok((
                                self.insert_at_child(
                                    node_key,
                                    &internal_node,
                                    version,
                                    kvs,
                                    *left,
                                    *right,
                                    depth,
                                    hash_cache,
                                    &mut sub_batch,
                                )?,
                                sub_batch,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .map(|(ret, sub_batch)| {
                            batch.combine(sub_batch);
                            ret
                        })
                        .collect()
                } else {
                    range_iter
                        .map(|(left, right)| {
                            self.insert_at_child(
                                node_key,
                                &internal_node,
                                version,
                                kvs,
                                left,
                                right,
                                depth,
                                hash_cache,
                                batch,
                            )
                        })
                        .collect::<Result<_>>()?
                };

                // Reuse the current `InternalNode` in memory to create a new internal node.
                let mut old_children: Children = internal_node.into();
                let mut new_created_children = HashMap::new();
                for (child_nibble, child_option) in new_children {
                    if let Some(child) = child_option {
                        new_created_children.insert(child_nibble, child);
                    } else {
                        old_children.remove(&child_nibble);
                    }
                }

                if old_children.is_empty() && new_created_children.is_empty() {
                    return Ok(None);
                } else if old_children.len() <= 1 && new_created_children.len() <= 1 {
                    if let Some((new_nibble, new_child)) = new_created_children.iter().next() {
                        if let Some((old_nibble, _old_child)) = old_children.iter().next() {
                            if old_nibble == new_nibble && new_child.is_leaf() {
                                return Ok(Some(new_child.clone()));
                            }
                        } else if new_child.is_leaf() {
                            return Ok(Some(new_child.clone()));
                        }
                    } else {
                        let (old_child_nibble, old_child) =
                            old_children.iter().next().expect("must exist");
                        if old_child.is_leaf() {
                            let old_child_node_key =
                                node_key.gen_child_node_key(old_child.version, *old_child_nibble);
                            let old_child_node = self.reader.get_node(&old_child_node_key)?;
                            batch.put_stale_node(old_child_node_key, version, &old_child_node);
                            return Ok(Some(old_child_node));
                        }
                    }
                }

                let mut new_children = old_children;
                for (child_index, new_child_node) in new_created_children {
                    let new_child_node_key = node_key.gen_child_node_key(version, child_index);
                    new_children.insert(
                        child_index,
                        Child::new(
                            Self::get_hash(&new_child_node_key, &new_child_node, hash_cache),
                            version,
                            new_child_node.node_type(),
                        ),
                    );
                    batch.put_node(new_child_node_key, new_child_node);
                }
                let new_internal_node = InternalNode::new(new_children);
                Ok(Some(new_internal_node.into()))
            }
            Node::Leaf(leaf_node) => self.batch_update_subtree_with_existing_leaf(
                node_key, version, leaf_node, kvs, depth, hash_cache, batch,
            ),
            Node::Null => {
                ensure!(depth == 0, "Null node can only exist at depth 0");
                self.batch_update_subtree(node_key, version, kvs, 0, hash_cache, batch)
            }
        }
    }

    fn insert_at_child(
        &self,
        node_key: &NodeKey,
        internal_node: &InternalNode,
        version: Version,
        kvs: &[(HashValue, Option<&(HashValue, K)>)],
        left: usize,
        right: usize,
        depth: usize,
        hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
        batch: &mut TreeUpdateBatch<K>,
    ) -> Result<(Nibble, Option<Node<K>>)> {
        let child_index = kvs[left].0.get_nibble(depth);
        let child = internal_node.child(child_index);

        let new_child_node_option = match child {
            Some(child) => self.batch_insert_at(
                &node_key.gen_child_node_key(child.version, child_index),
                version,
                &kvs[left..=right],
                depth + 1,
                hash_cache,
                batch,
            )?,
            None => self.batch_update_subtree(
                &node_key.gen_child_node_key(version, child_index),
                version,
                &kvs[left..=right],
                depth + 1,
                hash_cache,
                batch,
            )?,
        };

        Ok((child_index, new_child_node_option))
    }

    fn batch_update_subtree_with_existing_leaf(
        &self,
        node_key: &NodeKey,
        version: Version,
        existing_leaf_node: LeafNode<K>,
        kvs: &[(HashValue, Option<&(HashValue, K)>)],
        depth: usize,
        hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
        batch: &mut TreeUpdateBatch<K>,
    ) -> Result<Option<Node<K>>> {
        let existing_leaf_key = existing_leaf_node.account_key();

        if kvs.len() == 1 && kvs[0].0 == existing_leaf_key {
            if let (key, Some((value_hash, state_key))) = kvs[0] {
                let new_leaf_node = Node::new_leaf(key, *value_hash, (state_key.clone(), version));
                Ok(Some(new_leaf_node))
            } else {
                Ok(None)
            }
        } else {
            let existing_leaf_bucket = existing_leaf_key.get_nibble(depth);
            let mut isolated_existing_leaf = true;
            let mut children = vec![];
            for (left, right) in NibbleRangeIterator::new(kvs, depth) {
                let child_index = kvs[left].0.get_nibble(depth);
                let child_node_key = node_key.gen_child_node_key(version, child_index);
                if let Some(new_child_node) = if existing_leaf_bucket == child_index {
                    isolated_existing_leaf = false;
                    self.batch_update_subtree_with_existing_leaf(
                        &child_node_key,
                        version,
                        existing_leaf_node.clone(),
                        &kvs[left..=right],
                        depth + 1,
                        hash_cache,
                        batch,
                    )?
                } else {
                    self.batch_update_subtree(
                        &child_node_key,
                        version,
                        &kvs[left..=right],
                        depth + 1,
                        hash_cache,
                        batch,
                    )?
                } {
                    children.push((child_index, new_child_node));
                }
            }
            if isolated_existing_leaf {
                children.push((existing_leaf_bucket, existing_leaf_node.into()));
            }

            if children.is_empty() {
                Ok(None)
            } else if children.len() == 1 && children[0].1.is_leaf() {
                let (_, child) = children.pop().expect("Must exist");
                Ok(Some(child))
            } else {
                let new_internal_node = InternalNode::new(
                    children
                        .into_iter()
                        .map(|(child_index, new_child_node)| {
                            let new_child_node_key =
                                node_key.gen_child_node_key(version, child_index);
                            let result = (
                                child_index,
                                Child::new(
                                    Self::get_hash(
                                        &new_child_node_key,
                                        &new_child_node,
                                        hash_cache,
                                    ),
                                    version,
                                    new_child_node.node_type(),
                                ),
                            );
                            batch.put_node(new_child_node_key, new_child_node);
                            result
                        })
                        .collect(),
                );
                Ok(Some(new_internal_node.into()))
            }
        }
    }

    fn batch_update_subtree(
        &self,
        node_key: &NodeKey,
        version: Version,
        kvs: &[(HashValue, Option<&(HashValue, K)>)],
        depth: usize,
        hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
        batch: &mut TreeUpdateBatch<K>,
    ) -> Result<Option<Node<K>>> {
        if kvs.len() == 1 {
            if let (key, Some((value_hash, state_key))) = kvs[0] {
                let new_leaf_node = Node::new_leaf(key, *value_hash, (state_key.clone(), version));
                Ok(Some(new_leaf_node))
            } else {
                Ok(None)
            }
        } else {
            let mut children = vec![];
            for (left, right) in NibbleRangeIterator::new(kvs, depth) {
                let child_index = kvs[left].0.get_nibble(depth);
                let child_node_key = node_key.gen_child_node_key(version, child_index);
                if let Some(new_child_node) = self.batch_update_subtree(
                    &child_node_key,
                    version,
                    &kvs[left..=right],
                    depth + 1,
                    hash_cache,
                    batch,
                )? {
                    children.push((child_index, new_child_node))
                }
            }
            if children.is_empty() {
                Ok(None)
            } else if children.len() == 1 && children[0].1.is_leaf() {
                let (_, child) = children.pop().expect("Must exist");
                Ok(Some(child))
            } else {
                let new_internal_node = InternalNode::new(
                    children
                        .into_iter()
                        .map(|(child_index, new_child_node)| {
                            let new_child_node_key =
                                node_key.gen_child_node_key(version, child_index);
                            let result = (
                                child_index,
                                Child::new(
                                    Self::get_hash(
                                        &new_child_node_key,
                                        &new_child_node,
                                        hash_cache,
                                    ),
                                    version,
                                    new_child_node.node_type(),
                                ),
                            );
                            batch.put_node(new_child_node_key, new_child_node);
                            result
                        })
                        .collect(),
                );
                Ok(Some(new_internal_node.into()))
            }
        }
    }

    /// This is a convenient function that calls
    ///
    /// [`put_value_sets`](struct.JellyfishMerkleTree.html#method.put_value_set) without the node hash
    /// cache and assuming the base version is the immediate previous version.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn put_value_set_test(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, K)>)>,
        version: Version,
    ) -> Result<(HashValue, TreeUpdateBatch<K>)> {
        self.batch_put_value_set(
            value_set.into_iter().map(|(k, v)| (k, v)).collect(),
            None,
            version.checked_sub(1),
            version,
        )
    }

    /// Returns the value (if applicable) and the corresponding merkle proof.
    pub fn get_with_proof(
        &self,
        key: HashValue,
        version: Version,
    ) -> Result<(Option<(HashValue, (K, Version))>, SparseMerkleProof)> {
        self.get_with_proof_ext(key, version)
            .map(|(value, proof_ext)| (value, proof_ext.into()))
    }

    pub fn get_with_proof_ext(
        &self,
        key: HashValue,
        version: Version,
    ) -> Result<(Option<(HashValue, (K, Version))>, SparseMerkleProofExt)> {
        // Empty tree just returns proof with no sibling hash.
        let mut next_node_key = NodeKey::new_empty_path(version);
        let mut siblings = vec![];
        let nibble_path = NibblePath::new_even(key.to_vec());
        let mut nibble_iter = nibble_path.nibbles();

        // We limit the number of loops here deliberately to avoid potential cyclic graph bugs
        // in the tree structure.
        for nibble_depth in 0..=ROOT_NIBBLE_HEIGHT {
            let next_node = self.reader.get_node(&next_node_key).map_err(|err| {
                if nibble_depth == 0 {
                    MissingRootError { version }.into()
                } else {
                    err
                }
            })?;
            match next_node {
                Node::Internal(internal_node) => {
                    let queried_child_index = nibble_iter
                        .next()
                        .ok_or_else(|| format_err!("ran out of nibbles"))?;
                    let (child_node_key, mut siblings_in_internal) = internal_node
                        .get_child_with_siblings(
                            &next_node_key,
                            queried_child_index,
                            Some(self.reader),
                        )?;
                    siblings.append(&mut siblings_in_internal);
                    next_node_key = match child_node_key {
                        Some(node_key) => node_key,
                        None => {
                            return Ok((
                                None,
                                SparseMerkleProofExt::new(None, {
                                    siblings.reverse();
                                    siblings
                                }),
                            ))
                        }
                    };
                }
                Node::Leaf(leaf_node) => {
                    return Ok((
                        if leaf_node.account_key() == key {
                            Some((leaf_node.value_hash(), leaf_node.value_index().clone()))
                        } else {
                            None
                        },
                        SparseMerkleProofExt::new(Some(leaf_node.into()), {
                            siblings.reverse();
                            siblings
                        }),
                    ));
                }
                Node::Null => {
                    return Ok((None, SparseMerkleProofExt::new(None, vec![])));
                }
            }
        }
        bail!("Jellyfish Merkle tree has cyclic graph inside.");
    }

    /// Gets the proof that shows a list of keys up to `rightmost_key_to_prove` exist at `version`.
    pub fn get_range_proof(
        &self,
        rightmost_key_to_prove: HashValue,
        version: Version,
    ) -> Result<SparseMerkleRangeProof> {
        let (account, proof) = self.get_with_proof(rightmost_key_to_prove, version)?;
        ensure!(account.is_some(), "rightmost_key_to_prove must exist.");

        let siblings = proof
            .siblings()
            .iter()
            .rev()
            .zip(rightmost_key_to_prove.iter_bits())
            .filter_map(|(sibling, bit)| {
                // We only need to keep the siblings on the right.
                if !bit {
                    Some(*sibling)
                } else {
                    None
                }
            })
            .rev()
            .collect();
        Ok(SparseMerkleRangeProof::new(siblings))
    }

    #[cfg(test)]
    pub fn get(&self, key: HashValue, version: Version) -> Result<Option<HashValue>> {
        Ok(self.get_with_proof(key, version)?.0.map(|x| x.0))
    }

    fn get_root_node(&self, version: Version) -> Result<Node<K>> {
        self.get_root_node_option(version)?
            .ok_or_else(|| format_err!("Root node not found for version {}.", version))
    }

    fn get_root_node_option(&self, version: Version) -> Result<Option<Node<K>>> {
        let root_node_key = NodeKey::new_empty_path(version);
        self.reader.get_node_option(&root_node_key)
    }

    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        self.get_root_node(version).map(|n| n.hash())
    }

    pub fn get_root_hash_option(&self, version: Version) -> Result<Option<HashValue>> {
        Ok(self.get_root_node_option(version)?.map(|n| n.hash()))
    }

    pub fn get_leaf_count(&self, version: Version) -> Result<usize> {
        self.get_root_node(version).map(|n| n.leaf_count())
    }

    pub fn get_all_nodes_referenced(&self, version: Version) -> Result<Vec<NodeKey>> {
        let mut out_keys = vec![];
        self.get_all_nodes_referenced_impl(NodeKey::new_empty_path(version), &mut out_keys)?;
        Ok(out_keys)
    }

    fn get_all_nodes_referenced_impl(
        &self,
        key: NodeKey,
        out_keys: &mut Vec<NodeKey>,
    ) -> Result<()> {
        match self.reader.get_node(&key)? {
            Node::Internal(internal_node) => {
                for (child_nibble, child) in internal_node.children_sorted() {
                    self.get_all_nodes_referenced_impl(
                        key.gen_child_node_key(child.version, *child_nibble),
                        out_keys,
                    )?;
                }
            }
            Node::Leaf(_) | Node::Null => {}
        };

        out_keys.push(key);
        Ok(())
    }
}

trait NibbleExt {
    fn get_nibble(&self, index: usize) -> Nibble;
    fn common_prefix_nibbles_len(&self, other: HashValue) -> usize;
}

impl NibbleExt for HashValue {
    /// Returns the `index`-th nibble.
    fn get_nibble(&self, index: usize) -> Nibble {
        Nibble::from(if index % 2 == 0 {
            self[index / 2] >> 4
        } else {
            self[index / 2] & 0x0F
        })
    }

    /// Returns the length of common prefix of `self` and `other` in nibbles.
    fn common_prefix_nibbles_len(&self, other: HashValue) -> usize {
        self.common_prefix_bits_len(other) / 4
    }
}

#[cfg(test)]
mod test {
    use super::NibbleExt;
    use aptos_crypto::hash::{HashValue, TestOnlyHash};
    use aptos_types::nibble::Nibble;

    #[test]
    fn test_common_prefix_nibbles_len() {
        {
            let hash1 = b"hello".test_only_hash();
            let hash2 = b"HELLO".test_only_hash();
            assert_eq!(hash1[0], 0b0011_0011);
            assert_eq!(hash2[0], 0b1011_1000);
            assert_eq!(hash1.common_prefix_nibbles_len(hash2), 0);
        }
        {
            let hash1 = b"hello".test_only_hash();
            let hash2 = b"world".test_only_hash();
            assert_eq!(hash1[0], 0b0011_0011);
            assert_eq!(hash2[0], 0b0100_0010);
            assert_eq!(hash1.common_prefix_nibbles_len(hash2), 0);
        }
        {
            let hash1 = b"hello".test_only_hash();
            let hash2 = b"100011001000".test_only_hash();
            assert_eq!(hash1[0], 0b0011_0011);
            assert_eq!(hash2[0], 0b0011_0011);
            assert_eq!(hash1[1], 0b0011_1000);
            assert_eq!(hash2[1], 0b0010_0010);
            assert_eq!(hash1.common_prefix_nibbles_len(hash2), 2);
        }
        {
            let hash1 = b"hello".test_only_hash();
            let hash2 = b"hello".test_only_hash();
            assert_eq!(
                hash1.common_prefix_nibbles_len(hash2),
                HashValue::LENGTH * 2
            );
        }
    }

    #[test]
    fn test_get_nibble() {
        let hash = b"hello".test_only_hash();
        assert_eq!(hash.get_nibble(0), Nibble::from(3));
        assert_eq!(hash.get_nibble(1), Nibble::from(3));
        assert_eq!(hash.get_nibble(2), Nibble::from(3));
        assert_eq!(hash.get_nibble(3), Nibble::from(8));
        assert_eq!(hash.get_nibble(62), Nibble::from(9));
        assert_eq!(hash.get_nibble(63), Nibble::from(2));
    }
}
