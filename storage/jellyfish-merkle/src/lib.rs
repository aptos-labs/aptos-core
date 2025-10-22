// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This module implements [`JellyfishMerkleTree`] backed by storage module. The tree itself doesn't
//! persist anything, but realizes the logic of R/W only. The write path will produce all the
//! intermediate results in a batch for storage layer to commit and the read path will return
//! results directly.
//! The public APIs are only [`new`], [`batch_put_value_set_for_shard`], [`get_with_proof`], and
//! [`get_shard_persisted_versions`]. After each put with a `value_set` based on a known version,
//! the tree will return a new root hash with a [`TreeUpdateBatch`] containing all the new nodes
//! and indices of stale nodes.
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
//! [`batch_put_value_set_for_shard`]: struct.JellyfishMerkleTree.html#method.batch_put_value_set_for_shard
//! [`get_shard_persisted_versions`]: struct.JellyfishMerkleTree.html#method.get_shard_persisted_versions
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

use crate::metrics::{APTOS_JELLYFISH_LEAF_COUNT, APTOS_JELLYFISH_LEAF_DELETION_COUNT, COUNTER};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_metrics_core::IntCounterVecHelper;
use aptos_storage_interface::{db_ensure as ensure, db_other_bail, AptosDbError, Result};
use aptos_types::{
    nibble::{nibble_path::NibblePath, Nibble, ROOT_NIBBLE_HEIGHT},
    proof::{SparseMerkleProof, SparseMerkleProofExt, SparseMerkleRangeProof},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use arr_macro::arr;
use itertools::{EitherOrBoth, Itertools};
use node_type::{Child, Children, InternalNode, LeafNode, Node, NodeKey, NodeType};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::arbitrary::Arbitrary;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use rayon::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    marker::PhantomData,
};

const MAX_PARALLELIZABLE_DEPTH: usize = 2;

// Assumes 16 shards here.
const MIN_LEAF_DEPTH: usize = 1;

/// `TreeReader` defines the interface between
/// [`JellyfishMerkleTree`](struct.JellyfishMerkleTree.html)
/// and underlying storage holding nodes.
pub trait TreeReader<K> {
    /// Gets node given a node key. Returns error if the node does not exist.
    fn get_node(&self, node_key: &NodeKey) -> Result<Node<K>> {
        self.get_node_with_tag(node_key, "unknown")
    }

    /// Gets node given a node key. Returns error if the node does not exist.
    fn get_node_with_tag(&self, node_key: &NodeKey, tag: &str) -> Result<Node<K>> {
        self.get_node_option(node_key, tag)?
            .ok_or_else(|| AptosDbError::NotFound(format!("Missing node at {:?}.", node_key)))
    }

    /// Gets node given a node key. Returns `None` if the node does not exist.
    fn get_node_option(&self, node_key: &NodeKey, tag: &str) -> Result<Option<Node<K>>>;

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
}

impl<K> TreeUpdateBatch<K>
where
    K: Key,
{
    pub fn new() -> Self {
        Self {
            node_batch: vec![vec![]],
            stale_node_index_batch: vec![vec![]],
        }
    }

    pub fn combine(&mut self, other: Self) {
        let Self {
            node_batch,
            stale_node_index_batch,
        } = other;

        self.node_batch.extend(node_batch);
        self.stale_node_index_batch.extend(stale_node_index_batch);
    }

    #[cfg(test)]
    pub fn num_stale_node(&self) -> usize {
        self.stale_node_index_batch.iter().map(Vec::len).sum()
    }

    pub fn put_node(&mut self, node_key: NodeKey, node: Node<K>) {
        self.node_batch[0].push((node_key, node))
    }

    pub fn put_stale_node(&mut self, node_key: NodeKey, stale_since_version: Version) {
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

impl<K> std::iter::Iterator for NibbleRangeIterator<'_, K> {
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
    ///
    /// Assumes 16 shards in total here.
    pub fn batch_put_value_set_for_shard(
        &self,
        shard_id: u8,
        value_set: Vec<(HashValue, Option<&(HashValue, K)>)>,
        node_hashes: Option<&HashMap<NibblePath, HashValue>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(Node<K>, TreeUpdateBatch<K>)> {
        let deduped_and_sorted_kvs = value_set
            .into_iter()
            .inspect(|kv| {
                assert!(kv.0.nibble(0) == shard_id);
            })
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect::<Vec<_>>();

        // We currently assume 16 shards in total, therefore the nibble path for the shard root
        // contains exact 1 nibble which is the shard id. `shard_id << 4` here is to put the shard
        // id as the first nibble of the first byte.
        let shard_root_nibble_path = NibblePath::new_odd(vec![shard_id << 4]);
        let shard_root_node_key = NodeKey::new(version, shard_root_nibble_path.clone());

        let mut shard_batch = TreeUpdateBatch::new();
        let shard_root_node_opt = if let Some(persisted_version) = persisted_version {
            THREAD_MANAGER.get_io_pool().install(|| {
                self.batch_insert_at(
                    &NodeKey::new(persisted_version, shard_root_nibble_path),
                    version,
                    deduped_and_sorted_kvs.as_slice(),
                    /*depth=*/ 1,
                    &node_hashes,
                    &mut shard_batch,
                )
            })?
        } else {
            batch_update_subtree(
                &shard_root_node_key,
                version,
                deduped_and_sorted_kvs.as_slice(),
                /*depth=*/ 1,
                &node_hashes,
                &mut shard_batch,
            )?
        };

        let shard_root_node = if let Some(shard_root_node) = shard_root_node_opt {
            shard_batch.put_node(shard_root_node_key, shard_root_node.clone());
            shard_root_node
        } else {
            Node::Null
        };

        Ok((shard_root_node, shard_batch))
    }

    /// Assumes 16 shards here, top levels only contain root node.
    pub fn put_top_levels_nodes(
        &self,
        shard_root_nodes: Vec<Node<K>>,
        persisted_version: Option<Version>,
        version: Version,
    ) -> Result<(HashValue, usize, TreeUpdateBatch<K>)> {
        ensure!(
            shard_root_nodes.len() == 16,
            "sharded root nodes {} must be 16",
            shard_root_nodes.len()
        );

        let children = Children::from_sorted(shard_root_nodes.iter().enumerate().filter_map(
            |(i, shard_root_node)| {
                let node_type = shard_root_node.node_type();
                match node_type {
                    NodeType::Null => None,
                    _ => Some((
                        Nibble::from(i as u8),
                        Child::new(shard_root_node.hash(), version, node_type),
                    )),
                }
            },
        ));
        let root_node = if children.is_empty() {
            Node::Null
        } else {
            Node::Internal(InternalNode::new(children))
        };
        APTOS_JELLYFISH_LEAF_COUNT.set(root_node.leaf_count() as i64);

        let root_hash = root_node.hash();
        let leaf_count = root_node.leaf_count();

        let mut tree_update_batch = TreeUpdateBatch::new();
        if let Some(persisted_version) = persisted_version {
            tree_update_batch.put_stale_node(NodeKey::new_empty_path(persisted_version), version);
        }
        tree_update_batch.put_node(NodeKey::new_empty_path(version), root_node);

        Ok((root_hash, leaf_count, tree_update_batch))
    }

    /// Returns the node versions of the root of each shard, or None if the shard is empty.
    /// Assumes 16 shards here.
    pub fn get_shard_persisted_versions(
        &self,
        root_persisted_version: Option<Version>,
    ) -> Result<[Option<Version>; 16]> {
        let mut shard_persisted_versions = arr![None; 16];
        if let Some(root_persisted_version) = root_persisted_version {
            let root_node_key = NodeKey::new_empty_path(root_persisted_version);
            let root_node = self.reader.get_node_with_tag(&root_node_key, "commit")?;
            match root_node {
                Node::Internal(root_node) => {
                    for shard_id in 0..16 {
                        if let Some(Child { version, .. }) = root_node.child(Nibble::from(shard_id))
                        {
                            shard_persisted_versions[shard_id as usize] = Some(*version);
                        }
                    }
                },
                _ => {
                    unreachable!("Assume the db doesn't have exactly 1 state.")
                },
            }
        }

        Ok(shard_persisted_versions)
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
        let node_opt = self.reader.get_node_option(node_key, "commit")?;

        if node_opt.is_some() {
            batch.put_stale_node(node_key.clone(), version);
        }

        if kvs.is_empty() {
            return Ok(node_opt);
        }

        match node_opt {
            Some(Node::Internal(internal_node)) => {
                // There is a small possibility that the old internal node is intact.
                // Traverse all the path touched by `kvs` from this internal node.
                let range_iter = NibbleRangeIterator::new(kvs, depth);
                let new_child_nodes_or_deletes: Vec<_> = if depth <= MAX_PARALLELIZABLE_DEPTH {
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

                let children: Vec<_> = internal_node
                    .children_sorted()
                    .merge_join_by(new_child_nodes_or_deletes, |(n, _), (m, _)| (*n).cmp(m))
                    .filter(|old_or_new| {
                        !matches!(
                            old_or_new,
                            EitherOrBoth::Right((_, None)) | EitherOrBoth::Both((_, _), (_, None))
                        )
                    })
                    .collect();

                if children.is_empty() {
                    // all children are deleted
                    return Ok(None);
                }

                if children.len() == 1 {
                    // only one child left, could be a leaf node that we need to push up one level.
                    let only_child = children.first().unwrap();
                    match only_child {
                        EitherOrBoth::Left((nibble, old_child)) => {
                            if old_child.is_leaf() {
                                // it's an old leaf
                                let child_key =
                                    node_key.gen_child_node_key(old_child.version, **nibble);
                                let node = self.reader.get_node_with_tag(&child_key, "commit")?;
                                batch.put_stale_node(child_key, version);
                                return Ok(Some(node));
                            }
                        },
                        EitherOrBoth::Right((_nibble, new_node))
                        | EitherOrBoth::Both((_, _), (_nibble, new_node)) => {
                            let new_node =
                                new_node.as_ref().expect("Deletion already filtered out.");
                            if new_node.is_leaf() {
                                // it's a new leaf
                                return Ok(Some(new_node.clone()));
                            }
                        },
                    }
                }

                let children = children.into_iter().map(|old_or_new| {
                    match old_or_new {
                        // an old child
                        EitherOrBoth::Left((nibble, old_child)) => (*nibble, old_child.clone()),
                        // a new or updated child
                        EitherOrBoth::Right((nibble, new_node))
                        | EitherOrBoth::Both((_, _), (nibble, new_node)) => {
                            let new_node =
                                new_node.as_ref().expect("Deletion already filtered out.");
                            let child_key = node_key.gen_child_node_key(version, nibble);
                            batch.put_node(child_key, new_node.clone());
                            let child =
                                Child::for_node(node_key, nibble, new_node, hash_cache, version);
                            (nibble, child)
                        },
                    }
                });

                let new_internal_node = InternalNode::new(Children::from_sorted(children));
                Ok(Some(new_internal_node.into()))
            },
            Some(Node::Leaf(leaf_node)) => batch_update_subtree_with_existing_leaf(
                node_key, version, leaf_node, kvs, depth, hash_cache, batch,
            ),
            None => {
                ensure!(
                    depth <= MIN_LEAF_DEPTH,
                    "Null node can only exist at top levels."
                );
                batch_update_subtree(node_key, version, kvs, depth, hash_cache, batch)
            },
            _ => unreachable!(),
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
            None => batch_update_subtree(
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

    /// This is a convenient function for test only, without providing the node hash
    /// cache and assuming the base version is the immediate previous version.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn put_value_set_test(
        &self,
        value_set: Vec<(HashValue, Option<&(HashValue, K)>)>,
        version: Version,
    ) -> Result<(HashValue, TreeUpdateBatch<K>)> {
        let mut tree_update_batch = TreeUpdateBatch::new();
        let mut shard_root_nodes = Vec::with_capacity(16);
        for shard_id in 0..16 {
            let value_set_for_shard = value_set
                .iter()
                .filter(|(k, _v)| k.nibble(0) == shard_id)
                .cloned()
                .collect();
            let (shard_root_node, shard_batch) = self.batch_put_value_set_for_shard(
                shard_id,
                value_set_for_shard,
                None,
                version.checked_sub(1),
                version,
            )?;

            tree_update_batch.combine(shard_batch);
            shard_root_nodes.push(shard_root_node);
        }

        let (root_hash, _leaf_count, top_levels_batch) =
            self.put_top_levels_nodes(shard_root_nodes, version.checked_sub(1), version)?;
        tree_update_batch.combine(top_levels_batch);

        Ok((root_hash, tree_update_batch))
    }

    /// Returns the value (if applicable) and the corresponding merkle proof.
    pub fn get_with_proof(
        &self,
        key: HashValue,
        version: Version,
    ) -> Result<(Option<(HashValue, (K, Version))>, SparseMerkleProof)> {
        self.get_with_proof_ext(&key, version, 0)
            .map(|(value, proof_ext)| (value, proof_ext.into()))
    }

    pub fn get_with_proof_ext(
        &self,
        key: &HashValue,
        version: Version,
        target_root_depth: usize,
    ) -> Result<(Option<(HashValue, (K, Version))>, SparseMerkleProofExt)> {
        // Empty tree just returns proof with no sibling hash.
        let mut next_node_key = NodeKey::new_empty_path(version);
        let mut out_siblings = Vec::with_capacity(8); // reduces reallocation
        let nibble_path = NibblePath::new_even(key.to_vec());
        let mut nibble_iter = nibble_path.nibbles();

        // We limit the number of loops here deliberately to avoid potential cyclic graph bugs
        // in the tree structure.
        for nibble_depth in 0..=ROOT_NIBBLE_HEIGHT {
            let next_node = self
                .reader
                .get_node_with_tag(&next_node_key, "get_proof")
                .map_err(|err| {
                    if nibble_depth == 0 {
                        AptosDbError::MissingRootError(version)
                    } else {
                        err
                    }
                })?;
            match next_node {
                Node::Internal(internal_node) => {
                    if internal_node.leaf_count() == 1 {
                        // Logically this node should be a leaf node, it got pushed down for
                        // sharding, skip the siblings.
                        let (only_child_nibble, Child { version, .. }) =
                            internal_node.children_sorted().next().unwrap();
                        next_node_key =
                            next_node_key.gen_child_node_key(*version, *only_child_nibble);
                        continue;
                    }
                    let queried_child_index = nibble_iter
                        .next()
                        .ok_or_else(|| AptosDbError::Other("ran out of nibbles".to_string()))?;
                    let child_node_key = internal_node.get_child_with_siblings(
                        &next_node_key,
                        queried_child_index,
                        Some(self.reader),
                        &mut out_siblings,
                        nibble_depth * 4,
                        target_root_depth,
                    )?;
                    next_node_key = match child_node_key {
                        Some(node_key) => node_key,
                        None => {
                            return Ok((
                                None,
                                SparseMerkleProofExt::new_partial(
                                    None,
                                    out_siblings,
                                    target_root_depth,
                                ),
                            ));
                        },
                    };
                },
                Node::Leaf(leaf_node) => {
                    return Ok((
                        if leaf_node.account_key() == key {
                            Some((leaf_node.value_hash(), leaf_node.value_index().clone()))
                        } else {
                            None
                        },
                        SparseMerkleProofExt::new_partial(
                            Some(leaf_node.into()),
                            out_siblings,
                            target_root_depth,
                        ),
                    ));
                },
                Node::Null => {
                    return Ok((None, SparseMerkleProofExt::new(None, vec![])));
                },
            }
        }
        db_other_bail!("Jellyfish Merkle tree has cyclic graph inside.");
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
        self.get_root_node_option(version)?.ok_or_else(|| {
            AptosDbError::NotFound(format!("Root node not found for version {}.", version))
        })
    }

    fn get_root_node_option(&self, version: Version) -> Result<Option<Node<K>>> {
        let root_node_key = NodeKey::new_empty_path(version);
        self.reader.get_node_option(&root_node_key, "get_root")
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
            },
            Node::Leaf(_) | Node::Null => {},
        };

        out_keys.push(key);
        Ok(())
    }
}

/// Get the node hash from the cache if cache is provided, otherwise (for test only) compute it.
fn get_hash<K>(
    node_key: &NodeKey,
    node: &Node<K>,
    hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
) -> HashValue
where
    K: Key,
{
    if let Some(cache) = hash_cache {
        match cache.get(node_key.nibble_path()) {
            Some(hash) => *hash,
            None => {
                COUNTER.inc_with(&["get_hash_miss"]);
                node.hash()
            },
        }
    } else {
        node.hash()
    }
}

fn batch_update_subtree<K>(
    node_key: &NodeKey,
    version: Version,
    kvs: &[(HashValue, Option<&(HashValue, K)>)],
    depth: usize,
    hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
    batch: &mut TreeUpdateBatch<K>,
) -> Result<Option<Node<K>>>
where
    K: Key,
{
    if kvs.len() == 1 {
        if let (key, Some((value_hash, state_key))) = kvs[0] {
            if depth >= MIN_LEAF_DEPTH {
                // Only create leaf node when it is in the shard.
                let new_leaf_node = Node::new_leaf(key, *value_hash, (state_key.clone(), version));
                return Ok(Some(new_leaf_node));
            }
        } else {
            // Deletion, returns empty tree.
            return Ok(None);
        }
    }

    let mut children = vec![];
    for (left, right) in NibbleRangeIterator::new(kvs, depth) {
        let child_index = kvs[left].0.get_nibble(depth);
        let child_node_key = node_key.gen_child_node_key(version, child_index);
        if let Some(new_child_node) = batch_update_subtree(
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
    } else if children.len() == 1 && children[0].1.is_leaf() && depth >= MIN_LEAF_DEPTH {
        let (_, child) = children.pop().expect("Must exist");
        Ok(Some(child))
    } else {
        let new_internal_node = InternalNode::new(Children::from_sorted(children.into_iter().map(
            |(child_index, new_child_node)| {
                let new_child_node_key = node_key.gen_child_node_key(version, child_index);
                let result = (
                    child_index,
                    Child::new(
                        get_hash(&new_child_node_key, &new_child_node, hash_cache),
                        version,
                        new_child_node.node_type(),
                    ),
                );
                batch.put_node(new_child_node_key, new_child_node);
                result
            },
        )));
        Ok(Some(new_internal_node.into()))
    }
}

fn batch_update_subtree_with_existing_leaf<K>(
    node_key: &NodeKey,
    version: Version,
    existing_leaf_node: LeafNode<K>,
    kvs: &[(HashValue, Option<&(HashValue, K)>)],
    depth: usize,
    hash_cache: &Option<&HashMap<NibblePath, HashValue>>,
    batch: &mut TreeUpdateBatch<K>,
) -> Result<Option<Node<K>>>
where
    K: Key,
{
    let existing_leaf_key = existing_leaf_node.account_key();

    if kvs.len() == 1 && &kvs[0].0 == existing_leaf_key {
        if let (key, Some((value_hash, state_key))) = kvs[0] {
            let new_leaf_node = Node::new_leaf(key, *value_hash, (state_key.clone(), version));
            Ok(Some(new_leaf_node))
        } else {
            APTOS_JELLYFISH_LEAF_DELETION_COUNT.inc();
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
                batch_update_subtree_with_existing_leaf(
                    &child_node_key,
                    version,
                    existing_leaf_node.clone(),
                    &kvs[left..=right],
                    depth + 1,
                    hash_cache,
                    batch,
                )?
            } else {
                batch_update_subtree(
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
            children.sort_by_key(|(n, _)| *n)
        }

        if children.is_empty() {
            Ok(None)
        } else if children.len() == 1 && children[0].1.is_leaf() && depth >= MIN_LEAF_DEPTH {
            let (_, child) = children.pop().expect("Must exist");
            Ok(Some(child))
        } else {
            let children = children.into_iter().map(|(child_index, new_child_node)| {
                let new_child_node_key = node_key.gen_child_node_key(version, child_index);
                let result = (
                    child_index,
                    Child::new(
                        get_hash(&new_child_node_key, &new_child_node, hash_cache),
                        version,
                        new_child_node.node_type(),
                    ),
                );
                batch.put_node(new_child_node_key, new_child_node);
                result
            });
            let new_internal_node = InternalNode::new(Children::from_sorted(children));

            Ok(Some(new_internal_node.into()))
        }
    }
}

trait NibbleExt {
    fn get_nibble(&self, index: usize) -> Nibble;
    fn common_prefix_nibbles_len(&self, other: HashValue) -> usize;
}

impl NibbleExt for HashValue {
    /// Returns the `index`-th nibble.
    fn get_nibble(&self, index: usize) -> Nibble {
        Nibble::from(
            if index % 2 == 0 {
                self[index / 2] >> 4
            } else {
                self[index / 2] & 0x0F
            },
        )
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
