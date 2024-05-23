// Copyright © Aptos Foundation
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
#![allow(dead_code)]

pub mod ancestors;
mod dropper;
mod inc_hash;
mod metrics;
mod node;
// #[cfg(test)]
// mod sparse_merkle_test;
#[cfg(any(test, feature = "bench", feature = "fuzzing"))]
pub mod test_utils;
// mod updater;
pub mod utils;

use crate::sparse_merkle::{
    dropper::SUBTREE_DROPPER,
    inc_hash::{AuthByIncHash, HashAsKey, Root},
    metrics::GENERATION,
    node::SubTree,
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_drop_helper::ArcAsyncDrop;
use aptos_infallible::Mutex;
use aptos_metrics_core::IntGaugeHelper;
use aptos_types::{
    nibble::nibble_path::NibblePath, proof::SparseMerkleLeafNode,
    state_store::state_storage_usage::StateStorageUsage,
};
use fastcrypto::hash::MultisetHash;
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
struct Inner<V: ArcAsyncDrop> {
    root: Option<SubTree<V>>,
    usage: StateStorageUsage,
    children: Mutex<Vec<Arc<Inner<V>>>>,
    family: HashValue,
    generation: u64,
}

impl<V: ArcAsyncDrop> Drop for Inner<V> {
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

impl<V: ArcAsyncDrop> Inner<V> {
    fn new(root: SubTree<V>, usage: StateStorageUsage) -> Arc<Self> {
        let family = HashValue::random();
        let me = Arc::new(Self {
            root: Some(root),
            usage,
            children: Mutex::new(Vec::new()),
            family,
            generation: 0,
        });

        me
    }

    fn root(&self) -> &SubTree<V> {
        // root only goes away during Drop
        self.root.as_ref().expect("Root must exist.")
    }

    fn spawn(
        self: &Arc<Self>,
        child_root: SubTree<V>,
        child_usage: StateStorageUsage,
    ) -> Arc<Self> {
        let child = Arc::new(Self {
            root: Some(child_root),
            usage: child_usage,
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
#[derive(Debug)]
pub struct SparseMerkleTree<V: ArcAsyncDrop> {
    inner: Arc<AuthByIncHash<V>>,
}

impl<V: ArcAsyncDrop> Clone for SparseMerkleTree<V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<V> SparseMerkleTree<V>
where
    V: Clone + CryptoHash + ArcAsyncDrop,
{
    /// Constructs a Sparse Merkle Tree with a root hash. This is often used when we restart and
    /// the scratch pad and the storage have identical state, so we use a single root hash to
    /// represent the entire state.
    pub fn new(_root_hash: HashValue, usage: StateStorageUsage) -> Self {
        Self {
            inner: Arc::new(AuthByIncHash::new(usage)),
        }
    }

    #[cfg(test)]
    fn new_test(root_hash: HashValue) -> Self {
        Self::new(root_hash, StateStorageUsage::new_untracked())
    }

    pub fn new_empty() -> Self {
        Self::new(*SPARSE_MERKLE_PLACEHOLDER_HASH, StateStorageUsage::zero())
    }

    pub fn has_same_root_hash(&self, other: &Self) -> bool {
        self.root_hash() == other.root_hash()
    }

    pub fn freeze(&self, base_smt: &SparseMerkleTree<V>) -> FrozenSparseMerkleTree<V> {
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
    fn new_with_root(_root: SubTree<V>) -> Self {
        unimplemented!()
    }

    fn root_weak(&self) -> SubTree<V> {
        unimplemented!()
    }

    /// Returns the root hash of this tree.
    pub fn root_hash(&self) -> HashValue {
        self.inner.root().hash()
    }

    fn generation(&self) -> u64 {
        self.inner.generation()
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn is_family(&self, other: &Self) -> bool {
        self.inner.is_family(&other.inner)
    }

    pub fn usage(&self) -> StateStorageUsage {
        self.inner.usage
    }

    /// Compares an old and a new SMTs and return the newly created node hashes in between.
    ///
    /// Assumes 16 shards in total.
    pub fn new_node_hashes_since(
        &self,
        _since_smt: &Self,
        _shard_id: u8,
    ) -> HashMap<NibblePath, HashValue> {
        unimplemented!()
    }

    /// Recursively generate the partial node update batch of jellyfish merkle
    fn new_node_hashes_since_impl(
        _subtree: SubTree<V>,
        _since_generation: u64,
        _pos: &mut NodePosition,
        _node_hashes: &mut HashMap<NibblePath, HashValue>,
    ) {
        unimplemented!()
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
impl<V> SparseMerkleTree<V>
where
    V: Clone + CryptoHash + ArcAsyncDrop,
{
    pub fn batch_update(
        &self,
        _updates: Vec<(HashValue, Option<&V>)>,
        _proof_reader: &impl ProofRead,
    ) -> Result<Self, UpdateError> {
        unimplemented!()
    }

    pub fn get(&self, _key: HashValue) -> StateStoreStatus<V> {
        unimplemented!()
    }
}

impl<V> Default for SparseMerkleTree<V>
where
    V: Clone + CryptoHash + ArcAsyncDrop,
{
    fn default() -> Self {
        SparseMerkleTree::new_empty()
    }
}

/// `AccountStatus` describes the result of querying an account from this SparseMerkleTree.
#[derive(Debug, Eq, PartialEq)]
pub enum StateStoreStatus<V> {
    /// The entry exists in the tree, therefore we can give its value.
    ExistsInScratchPad(V),

    /// The entry does not exist in either the tree or DB. This happens when the search reaches
    /// an empty node, or a leaf node that has a different account.
    DoesNotExist,

    /// Tree nodes only exist until `depth` on the route from the root to the leaf address, needs
    /// to check the DB for the rest.
    UnknownSubtreeRoot { hash: HashValue, depth: usize },

    /// Found leaf node, but the value is only in the DB.
    UnknownValue,
}

/// In the entire lifetime of this, in-mem nodes won't be dropped because a reference to the oldest
/// SMT is held inside.
#[derive(Clone, Debug)]
pub struct FrozenSparseMerkleTree<V: ArcAsyncDrop> {
    pub base_smt: SparseMerkleTree<V>,
    pub base_generation: u64,
    pub smt: SparseMerkleTree<V>,
}

impl<V> FrozenSparseMerkleTree<V>
where
    V: Clone + CryptoHash + ArcAsyncDrop,
{
    fn spawn(&self, child_root: Root<V>, child_usage: StateStorageUsage) -> Self {
        let smt = SparseMerkleTree {
            inner: self.smt.inner.spawn(child_root, child_usage),
        };
        smt.log_generation("spawn");

        Self {
            base_smt: self.base_smt.clone(),
            base_generation: self.base_generation,
            smt,
        }
    }

    pub fn unfreeze(self) -> SparseMerkleTree<V> {
        self.smt
    }

    pub fn root_hash(&self) -> HashValue {
        self.smt.root_hash()
    }

    /// Constructs a new Sparse Merkle Tree by applying `updates`, which are considered to happen
    /// all at once.
    /// Since the tree is immutable, existing tree remains the same and may share parts with the
    /// new, returned tree.
    pub fn batch_update(
        &self,
        updates: Vec<(HashValue, Option<&V>)>,
        usage: StateStorageUsage,
        proof_reader: &impl ProofRead,
    ) -> Result<Self, UpdateError> {
        // Flatten, dedup and sort the updates with a btree map since the updates between different
        // versions may overlap on the same address in which case the latter always overwrites.
        let updates = updates
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .map(|(k, v)| (HashAsKey(k), Some(v.cloned())))
            .collect::<Vec<_>>();

        if updates.is_empty() {
            if !usage.is_untracked() {
                assert_eq!(self.smt.inner.usage, usage);
            }
            return Ok(self.clone());
        }

        let content = self
            .smt
            .inner
            .root()
            .content
            .view_layers_since(&self.base_smt.inner.root().content)
            .new_layer(&updates[..]);

        let hashes_to_remove = updates.iter().filter_map(|(k, _)| {
            proof_reader.get_proof(k.0).map(|value_hash| {
                SparseMerkleLeafNode::new(k.0, value_hash)
                    .hash()
                    .into_inner()
            })
        });
        let hashes_to_insert = updates.iter().filter_map(|(k, v)| {
            v.as_ref().and_then(|val| {
                val.as_ref()
                    .map(|v| SparseMerkleLeafNode::new(k.0, v.hash()).hash().into_inner())
            })
        });

        let mut inc_hash = self.smt.inner.root().inc_hash.clone();
        inc_hash.remove_all(hashes_to_remove);
        inc_hash.insert_all(hashes_to_insert);

        let root = Root::new(inc_hash, content);

        Ok(self.spawn(root, usage))
    }

    /// Queries a `key` in this `SparseMerkleTree`.
    pub fn get(&self, key: HashValue) -> StateStoreStatus<V> {
        use StateStoreStatus::*;

        match self
            .smt
            .inner
            .root
            .content
            .view_layers_since(&self.base_smt.inner.root.content)
            .get(&HashAsKey(key))
        {
            Some(val_opt) => match val_opt {
                Some(val) => ExistsInScratchPad(val),
                None => DoesNotExist,
            },
            None => UnknownValue,
        }
    }

    pub fn usage(&self) -> StateStorageUsage {
        self.smt.usage()
    }
}

/// A type that implements `ProofRead` can provide proof for keys in persistent storage.
pub trait ProofRead: Sync {
    /// Gets verified proof for this key in persistent storage.

    // HACK: reuse ProofRead to return value hash on base version
    fn get_proof(&self, key: HashValue) -> Option<HashValue>;
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
