// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module implements an in-memory Sparse Merkle Tree that is similar to what we use in
//! storage to represent world state. This tree will store only a small portion of the state -- the
//! part of accounts that have been modified by uncommitted transactions. For example, if we
//! execute a transaction T_i on top of committed state and it modified account A, we will end up
//! having the following tree:
//! ```text
//!              S_i
//!             /   \
//!            o     y
//!           / \
//!          x   A
//! ```
//! where A has the new state of the account, and y and x are the siblings on the path from root to
//! A in the tree.
//!
//! This Sparse Merkle Tree is immutable once constructed. If the next transaction T_{i+1} modified
//! another account B that lives in the subtree at y, a new tree will be constructed and the
//! structure will look like the following:
//! ```text
//!                 S_i        S_{i+1}
//!                /   \      /       \
//!               /     y   /          \
//!              / _______/             \
//!             //                       \
//!            o                          y'
//!           / \                        / \
//!          x   A                      z   B
//! ```
//!
//! Using this structure, we are able to query the global state, taking into account the output of
//! uncommitted transactions. For example, if we want to execute another transaction T_{i+1}', we
//! can use the tree S_i. If we look for account A, we can find its new value in the tree.
//! Otherwise, we know the account does not exist in the tree, and we can fall back to storage. As
//! another example, if we want to execute transaction T_{i+2}, we can use the tree S_{i+1} that
//! has updated values for both account A and B.
//!
//! Each version of the tree holds a strong reference (an `Arc<Node>`) to its root as well as one to
//! its base tree (`S_i` is the base tree of `S_{i+1}` in the above example). The root node in turn,
//! recursively holds all descendant nodes created in the same version, and weak references
//! (a `Weak<Node>`) to all descendant nodes that was created from previous versions.
//! With this construction:
//!     1. Even if a reference to a specific tree is dropped, the nodes belonging to it won't be
//! dropped as long as trees depending on it still hold strong references to it via the chain of
//! "base trees".
//!     2. Even if a tree is not dropped, when nodes it created are persisted to DB, all of them
//! and those created by its previous versions can be dropped, which we express by calling "prune()"
//! on it which replaces the strong references to its root and its base tree with weak references.
//!     3. We can hold strong references to recently accessed nodes that have already been persisted
//! in an LRU flavor cache for less DB reads.
//!
//! This Sparse Merkle Tree serves a dual purpose. First, to support a leader based consensus
//! algorithm, we need to build a tree of transactions like the following:
//! ```text
//! Committed -> T5 -> T6  -> T7
//!              └---> T6' -> T7'
//!                    └----> T7"
//! ```
//! Once T5 is executed, we will have a tree that stores the modified portion of the state. Later
//! when we execute T6 on top of T5, the output of T5 can be visible to T6.
//!
//! Second, given this tree representation it is straightforward to compute the root hash of S_i
//! once T_i is executed. This allows us to verify the proofs we need when executing T_{i+1}.

// See https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=e9c4c53eb80b30d09112fcfb07d481e7
#![allow(clippy::let_and_return)]
// See https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=795cd4f459f1d4a0005a99650726834b
#![allow(clippy::while_let_loop)]

pub mod dropper;
mod metrics;
mod node;
#[cfg(test)]
mod sparse_merkle_test;
#[cfg(any(test, feature = "bench", feature = "fuzzing"))]
pub mod test_utils;
mod updater;
pub mod utils;

use crate::sparse_merkle::{
    dropper::SUBTREE_DROPPER,
    metrics::{GENERATION, TIMER},
    node::{NodeInner, SubTree},
    updater::SubTreeUpdater,
    utils::get_state_shard_id,
};
use velor_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use velor_infallible::Mutex;
use velor_metrics_core::{IntGaugeVecHelper, TimerHelper};
use velor_types::{
    nibble::{nibble_path::NibblePath, Nibble},
    proof::SparseMerkleProofExt,
    state_store::state_key::StateKey,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use thiserror::Error;

type NodePosition = bitvec::vec::BitVec<u8, bitvec::order::Msb0>;
const BITS_IN_NIBBLE: usize = 4;
const BITS_IN_BYTE: usize = 8;

/// The inner content of a sparse merkle tree, we have this so that even if a tree is dropped, the
/// INNER of it can still live if referenced by a previous version.
#[derive(Debug)]
struct Inner {
    root: Option<SubTree>,
    children: Mutex<Vec<Arc<Inner>>>,
    family: HashValue,
    generation: u64,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Drop the root in a different thread, because that's the slowest part.
        SUBTREE_DROPPER.schedule_drop(self.root.take());

        let mut stack = self.drain_children_for_drop();
        while let Some(descendant) = stack.pop() {
            if Arc::strong_count(&descendant) == 1 {
                // The only ref is the one we are now holding, so the
                // descendant will be dropped after we free the `Arc`, which results in a chain
                // of such structures being dropped recursively and that might trigger a stack
                // overflow. To prevent that we follow the chain further to disconnect things
                // beforehand.
                stack.extend(descendant.drain_children_for_drop());
            }
        }
        self.log_generation("drop");
    }
}

impl Inner {
    fn new(root: SubTree) -> Arc<Self> {
        let family = HashValue::random();
        let me = Arc::new(Self {
            root: Some(root),
            children: Mutex::new(Vec::new()),
            family,
            generation: 0,
        });

        me
    }

    fn root(&self) -> &SubTree {
        // root only goes away during Drop
        self.root.as_ref().expect("Root must exist.")
    }

    fn spawn(self: &Arc<Self>, child_root: SubTree) -> Arc<Self> {
        let child = Arc::new(Self {
            root: Some(child_root),
            children: Mutex::new(Vec::new()),
            family: self.family,
            generation: self.generation + 1,
        });
        self.children.lock().push(child.clone());

        child
    }

    fn drain_children_for_drop(&self) -> Vec<Arc<Self>> {
        self.children.lock().drain(..).collect()
    }

    fn log_generation(&self, name: &'static str) {
        GENERATION.set_with(&[name], self.generation as i64);
    }
}

/// The Sparse Merkle Tree implementation.
#[derive(Clone, Debug)]
pub struct SparseMerkleTree {
    inner: Arc<Inner>,
}

impl SparseMerkleTree {
    /// Constructs a Sparse Merkle Tree with a root hash. This is often used when we restart and
    /// the scratch pad and the storage have identical state, so we use a single root hash to
    /// represent the entire state.
    pub fn new(root_hash: HashValue) -> Self {
        let root = if root_hash != *SPARSE_MERKLE_PLACEHOLDER_HASH {
            SubTree::new_unknown(root_hash)
        } else {
            SubTree::new_empty()
        };

        Self {
            inner: Inner::new(root),
        }
    }

    #[cfg(test)]
    fn new_test(root_hash: HashValue) -> Self {
        Self::new(root_hash)
    }

    pub fn new_empty() -> Self {
        Self {
            inner: Inner::new(SubTree::new_empty()),
        }
    }

    pub fn has_same_root_hash(&self, other: &Self) -> bool {
        self.root_hash() == other.root_hash()
    }

    pub fn freeze(&self, base_smt: &SparseMerkleTree) -> FrozenSparseMerkleTree {
        assert!(base_smt.is_family(self));

        self.log_generation("freeze");
        base_smt.log_generation("oldest");

        FrozenSparseMerkleTree {
            base_smt: base_smt.clone(),
            base_generation: base_smt.generation(),
            smt: self.clone(),
        }
    }

    pub fn log_generation(&self, name: &'static str) {
        self.inner.log_generation(name)
    }

    #[cfg(test)]
    fn new_with_root(root: SubTree) -> Self {
        Self {
            inner: Inner::new(root),
        }
    }

    fn root_weak(&self) -> SubTree {
        self.inner.root().weak()
    }

    /// Returns the root hash of this tree.
    pub fn root_hash(&self) -> HashValue {
        self.inner.root().hash()
    }

    fn generation(&self) -> u64 {
        self.inner.generation
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn is_family(&self, other: &Self) -> bool {
        self.inner.family == other.inner.family
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.is_family(other) && self.generation() >= other.generation()
    }

    /// Compares an old and a new SMTs and return the newly created node hashes in between.
    ///
    /// Assumes 16 shards in total.
    pub fn new_node_hashes_since(
        &self,
        since_smt: &Self,
        shard_id: u8,
    ) -> HashMap<NibblePath, HashValue> {
        let _timer = TIMER.timer_with(&["new_node_hashes_since"]);

        assert!(since_smt.is_family(self));

        let mut node_hashes = HashMap::new();
        let mut subtree = self.root_weak();
        let mut pos = NodePosition::with_capacity(HashValue::LENGTH_IN_BITS);
        let since_generation = since_smt.generation() + 1;
        // Assume 16 shards here.
        // We check the top 4 levels first, if there is any leaf node belongs to the shard we are
        // requesting, we collect that node and return earlier (because there is no nodes below
        // this point).
        // Otherwise, once we reach the 5th level (the level of the root of each shard), all nodes
        // at or below it belongs to the requested shard.
        for i in (0..4).rev() {
            if let Some(node) = subtree.get_node_if_in_mem(since_generation) {
                match node.inner() {
                    NodeInner::Internal(internal_node) => {
                        subtree = match (shard_id >> i) & 1 {
                            0 => {
                                pos.push(false);
                                internal_node.left.weak()
                            },
                            1 => {
                                pos.push(true);
                                internal_node.right.weak()
                            },
                            _ => {
                                unreachable!()
                            },
                        }
                    },
                    NodeInner::Leaf(leaf_node) => {
                        if get_state_shard_id(leaf_node.key()) == shard_id {
                            let mut nibble_path = NibblePath::new_even(vec![]);
                            nibble_path.push(Nibble::from(shard_id));
                            node_hashes.insert(nibble_path, subtree.hash());
                        }
                        return node_hashes;
                    },
                }
            } else {
                return node_hashes;
            }
        }
        Self::new_node_hashes_since_impl(
            subtree,
            since_smt.generation() + 1,
            &mut pos,
            &mut node_hashes,
        );
        node_hashes
    }

    /// Recursively generate the partial node update batch of jellyfish merkle
    fn new_node_hashes_since_impl(
        subtree: SubTree,
        since_generation: u64,
        pos: &mut NodePosition,
        node_hashes: &mut HashMap<NibblePath, HashValue>,
    ) {
        if let Some(node) = subtree.get_node_if_in_mem(since_generation) {
            let is_nibble = if let Some(path) = Self::maybe_to_nibble_path(pos) {
                node_hashes.insert(path, subtree.hash());
                true
            } else {
                false
            };
            match node.inner() {
                NodeInner::Internal(internal_node) => {
                    let depth = pos.len();
                    pos.push(false);
                    Self::new_node_hashes_since_impl(
                        internal_node.left.weak(),
                        since_generation,
                        pos,
                        node_hashes,
                    );
                    *pos.get_mut(depth).unwrap() = true;
                    Self::new_node_hashes_since_impl(
                        internal_node.right.weak(),
                        since_generation,
                        pos,
                        node_hashes,
                    );
                    pos.pop();
                },
                NodeInner::Leaf(leaf_node) => {
                    let mut path = NibblePath::new_even(leaf_node.key().to_vec());
                    if !is_nibble {
                        path.truncate(pos.len() / BITS_IN_NIBBLE + 1);
                        node_hashes.insert(path, subtree.hash());
                    }
                },
            }
        }
    }

    fn maybe_to_nibble_path(pos: &NodePosition) -> Option<NibblePath> {
        assert!(pos.len() <= HashValue::LENGTH_IN_BITS);

        if pos.len() % BITS_IN_NIBBLE == 0 {
            let mut bytes = pos.clone().into_vec();
            if pos.len() % BITS_IN_BYTE == 0 {
                Some(NibblePath::new_even(bytes))
            } else {
                // Unused bits in `BitVec` is uninitialized, setting to 0 to make sure.
                if let Some(b) = bytes.last_mut() {
                    *b &= 0xF0
                }

                Some(NibblePath::new_odd(bytes))
            }
        } else {
            None
        }
    }
}

/// In tests and benchmark, reference to ancestors are manually managed
#[cfg(any(feature = "fuzzing", feature = "bench", test))]
impl SparseMerkleTree {
    pub fn freeze_self_and_update(
        &self,
        updates: Vec<(HashValue, Option<HashValue>)>,
        proof_reader: &impl ProofRead,
    ) -> Result<Self, UpdateError> {
        self.clone()
            .freeze(self)
            .batch_update(updates.iter(), proof_reader)
            .map(FrozenSparseMerkleTree::unfreeze)
    }

    pub fn freeze_self_and_get(&self, key: HashValue) -> StateStoreStatus {
        self.clone().freeze(self).get(key)
    }
}

impl Default for SparseMerkleTree {
    fn default() -> Self {
        SparseMerkleTree::new_empty()
    }
}

/// `AccountStatus` describes the result of querying an account from this SparseMerkleTree.
#[derive(Debug, Eq, PartialEq)]
pub enum StateStoreStatus {
    /// The entry exists in the tree, therefore we can give its value.
    ExistsInScratchPad(HashValue),

    /// The entry does not exist in either the tree or DB. This happens when the search reaches
    /// an empty node, or a leaf node that has a different account.
    DoesNotExist,

    /// Tree nodes only exist until `depth` on the route from the root to the leaf address, needs
    /// to check the DB for the rest.
    UnknownSubtreeRoot { hash: HashValue, depth: usize },
}

/// Possible to use AsRef<HashValue> instead, but HashValue has already implemented AsRef<[u8; 32]>
/// and Deref<[u8; 32]>, which brings a lot of changes necessary on the calls sites if we add
/// AsRef<HashValue>.
pub trait HashValueRef {
    fn hash_ref(&self) -> &HashValue;
}

impl HashValueRef for HashValue {
    fn hash_ref(&self) -> &HashValue {
        self
    }
}

impl HashValueRef for &HashValue {
    fn hash_ref(&self) -> &HashValue {
        self
    }
}

impl HashValueRef for &StateKey {
    fn hash_ref(&self) -> &HashValue {
        self.crypto_hash_ref()
    }
}

/// In the entire lifetime of this, in-mem nodes won't be dropped because a reference to the oldest
/// SMT is held inside.
#[derive(Clone, Debug)]
pub struct FrozenSparseMerkleTree {
    pub base_smt: SparseMerkleTree,
    pub base_generation: u64,
    pub smt: SparseMerkleTree,
}

impl FrozenSparseMerkleTree {
    fn spawn(&self, child_root: SubTree) -> Self {
        let smt = SparseMerkleTree {
            inner: self.smt.inner.spawn(child_root),
        };
        smt.log_generation("spawn");

        Self {
            base_smt: self.base_smt.clone(),
            base_generation: self.base_generation,
            smt,
        }
    }

    pub fn unfreeze(self) -> SparseMerkleTree {
        self.smt
    }

    pub fn root_hash(&self) -> HashValue {
        self.smt.root_hash()
    }

    /// Constructs a new Sparse Merkle Tree by applying `updates`, which are considered to happen
    /// all at once.
    /// Since the tree is immutable, existing tree remains the same and may share parts with the
    /// new, returned tree.
    pub fn batch_update<'a>(
        &self,
        updates: impl Iterator<Item = &'a (impl HashValueRef + 'a, Option<impl HashValueRef + 'a>)>,
        proof_reader: &impl ProofRead,
    ) -> Result<Self, UpdateError> {
        // Flatten, dedup and sort the updates with a btree map since the updates between different
        // versions may overlap on the same address in which case the latter always overwrites.
        let kvs = updates
            .map(|(k, v)| (k.hash_ref(), v.as_ref().map(|v| v.hash_ref())))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect::<Vec<_>>();

        self.batch_update_sorted_uniq(&kvs, proof_reader)
    }

    pub fn batch_update_sorted_uniq<'a, K, V>(
        &self,
        sorted_unique_updates: &'a [(K, Option<V>)],
        proof_reader: &impl ProofRead,
    ) -> Result<Self, UpdateError>
    where
        K: 'a + HashValueRef + Sync,
        V: 'a + HashValueRef + Sync,
    {
        if sorted_unique_updates.is_empty() {
            Ok(self.clone())
        } else {
            let current_root = self.smt.root_weak();
            let root = SubTreeUpdater::update(
                current_root,
                sorted_unique_updates,
                proof_reader,
                self.smt.inner.generation + 1,
            )?;
            Ok(self.spawn(root))
        }
    }

    /// Queries a `key` in this `SparseMerkleTree`.
    #[cfg(any(feature = "fuzzing", feature = "bench", test))]
    fn get(&self, key: HashValue) -> StateStoreStatus {
        let mut subtree = self.smt.root_weak();
        let mut bits = key.iter_bits();
        let mut next_depth = 0;

        loop {
            next_depth += 1;
            match subtree {
                SubTree::Empty => return StateStoreStatus::DoesNotExist,
                SubTree::NonEmpty { hash, root: _ } => {
                    match subtree.get_node_if_in_mem(self.base_generation) {
                        None => {
                            return StateStoreStatus::UnknownSubtreeRoot {
                                hash,
                                depth: next_depth - 1,
                            }
                        },
                        Some(node) => match node.inner() {
                            NodeInner::Internal(internal_node) => {
                                subtree = if bits.next().expect("Tree is too deep.") {
                                    internal_node.right.weak()
                                } else {
                                    internal_node.left.weak()
                                };
                                continue;
                            }, // end NodeInner::Internal
                            NodeInner::Leaf(leaf_node) => {
                                return if *leaf_node.key() == key {
                                    StateStoreStatus::ExistsInScratchPad(*leaf_node.value_hash())
                                } else {
                                    StateStoreStatus::DoesNotExist
                                };
                            }, // end NodeInner::Leaf
                        }, // end Some(node) got from mem
                    }
                }, // end SubTree::NonEmpty
            }
        } // end loop
    }
}

/// A type that implements `ProofRead` can provide proof for keys in persistent storage.
pub trait ProofRead: Sync {
    /// Gets verified proof for this key in persistent storage.
    fn get_proof(&self, key: &HashValue, root_depth: usize) -> Option<SparseMerkleProofExt>;
}

impl ProofRead for () {
    fn get_proof(&self, _key: &HashValue, _root_depth: usize) -> Option<SparseMerkleProofExt> {
        unimplemented!()
    }
}

/// All errors `update` can possibly return.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum UpdateError {
    /// The update intends to insert a key that does not exist in the tree, so the operation needs
    /// proof to get more information about the tree, but no proof is provided.
    #[error("Missing Proof")]
    MissingProof,
    /// At `depth` a persisted subtree was encountered and a proof was requested to assist finding
    /// details about the subtree, but the result proof indicates the subtree is empty.
    #[error(
        "Short proof: key: {}, num_siblings: {}, depth: {}",
        key,
        num_siblings,
        depth
    )]
    ShortProof {
        key: HashValue,
        num_siblings: usize,
        depth: usize,
    },
}
