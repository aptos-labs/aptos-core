// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines all kinds of structures in the Sparse Merkle Tree maintained in scratch pad.
//! There are four kinds of nodes:
//! - A `SubTree::Empty` represents an empty subtree with zero leaf. Its root hash is assumed to be
//! the default hash.
//!
//! - A `SubTree::NonEmpty` represents a subtree with one or more leaves, it carries its root hash.
//!
//! From a `SubTree::NonEmpty` one may or may not get an reference to its root node, depending on
//! how this subtree structure was created and if the root node has been dropped (when its persisted
//! to DB and given up by any possible cache). A non empty subtree can refer to one of two types of
//! nodes as its root:
//!
//! - An `InternalNode` is a node that has two children. It is same as the internal node in a
//! standard Merkle tree.
//!
//! - A `LeafNode` represents a single account. Similar to what is in storage, a leaf node has a
//! key which is the hash of the account address as well as a value hash which is the hash of the
//! corresponding account content. The difference is that a `LeafNode` does not always have the
//! value, in the case when the leaf was loaded into memory as part of a non-inclusion proof.

use aptos_crypto::{
    HashValue,
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
};
use aptos_types::proof::{SparseMerkleInternalNode, SparseMerkleLeafNode};
use std::sync::{Arc, Weak};

#[derive(Clone, Debug)]
pub(crate) struct InternalNode {
    pub left: SubTree,
    pub right: SubTree,
}

impl InternalNode {
    pub fn calc_hash(&self) -> HashValue {
        SparseMerkleInternalNode::new(self.left.hash(), self.right.hash()).hash()
    }
}

type LeafNode = SparseMerkleLeafNode;

#[derive(Debug)]
pub(crate) enum NodeInner {
    Internal(InternalNode),
    Leaf(LeafNode),
}

#[derive(Debug)]
pub(crate) struct Node {
    generation: u64,
    inner: NodeInner,
}

impl Node {
    pub fn calc_hash(&self) -> HashValue {
        match &self.inner {
            NodeInner::Internal(internal_node) => internal_node.calc_hash(),
            NodeInner::Leaf(leaf_node) => leaf_node.calc_hash(),
        }
    }
}

impl Node {
    pub fn new_leaf(key: HashValue, value: HashValue, generation: u64) -> Self {
        Self {
            generation,
            inner: NodeInner::Leaf(LeafNode::new(key, value)),
        }
    }

    #[cfg(test)]
    pub fn new_internal(left: SubTree, right: SubTree, generation: u64) -> Self {
        Self {
            generation,
            inner: NodeInner::Internal(InternalNode { left, right }),
        }
    }

    pub fn new_internal_from_node(node: InternalNode, generation: u64) -> Self {
        Self {
            generation,
            inner: NodeInner::Internal(node),
        }
    }

    pub fn inner(&self) -> &NodeInner {
        &self.inner
    }
}

#[derive(Debug)]
pub enum Ref<R> {
    Shared(Arc<R>),
    Weak(Weak<R>),
}

impl<R> Ref<R> {
    pub fn new_unknown() -> Self {
        Self::Weak(Weak::new())
    }

    pub fn new_shared(referee: R) -> Self {
        Self::Shared(Arc::new(referee))
    }

    pub fn weak(&self) -> Self {
        Self::Weak(match self {
            Self::Shared(arc) => Arc::downgrade(arc),
            Self::Weak(weak) => weak.clone(),
        })
    }

    pub fn get_if_in_mem(&self) -> Option<Arc<R>> {
        match self {
            Self::Shared(arc) => Some(arc.clone()),
            Self::Weak(weak) => weak.upgrade(),
        }
    }
}

impl<R> Clone for Ref<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Shared(arc) => Self::Shared(arc.clone()),
            Self::Weak(weak) => Self::Weak(weak.clone()),
        }
    }
}

pub(crate) type NodeHandle = Ref<Node>;

#[derive(Clone, Debug)]
pub(crate) enum SubTree {
    Empty,
    NonEmpty { hash: HashValue, root: NodeHandle },
}

impl SubTree {
    pub fn new_empty() -> Self {
        Self::Empty
    }

    pub fn new_unknown(hash: HashValue) -> Self {
        Self::NonEmpty {
            hash,
            root: NodeHandle::new_unknown(),
        }
    }

    pub fn new_leaf(key: HashValue, value: HashValue, generation: u64) -> Self {
        let leaf = Node::new_leaf(key, value, generation);

        Self::NonEmpty {
            hash: leaf.calc_hash(),
            root: NodeHandle::new_shared(leaf),
        }
    }

    #[cfg(test)]
    pub fn new_internal(left: Self, right: Self, generation: u64) -> Self {
        let internal = Node::new_internal(left, right, generation);

        Self::NonEmpty {
            hash: internal.calc_hash(),
            root: NodeHandle::new_shared(internal),
        }
    }

    pub fn hash(&self) -> HashValue {
        match self {
            Self::Empty => *SPARSE_MERKLE_PLACEHOLDER_HASH,
            Self::NonEmpty { hash, .. } => *hash,
        }
    }

    pub fn weak(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::NonEmpty { hash, root } => Self::NonEmpty {
                hash: *hash,
                root: root.weak(),
            },
        }
    }

    pub fn get_node_if_in_mem(&self, min_generation: u64) -> Option<Arc<Node>> {
        match self {
            Self::Empty => None,
            Self::NonEmpty { root, .. } => root.get_if_in_mem().and_then(|n| {
                if n.generation >= min_generation {
                    Some(n)
                } else {
                    None
                }
            }),
        }
    }

    #[cfg(test)]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::NonEmpty {
            root: NodeHandle::Weak(_),
            ..
        })
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        matches!(self, SubTree::Empty)
    }
}
