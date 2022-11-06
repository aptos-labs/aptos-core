// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

extern crate aptos_types;

use aptos_types::transaction::Version;
use aptos_types::xibble::XibblePath;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct NodeKey {
    // The version at which the node is created.
    version: Version,
    // The nibble path this node represents in the tree.
    nibble_path: XibblePath,
}

impl NodeKey {
    pub fn new_empty_path(version: Version) -> NodeKey {
        todo!()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node<K> {
    /// A wrapper of [`InternalNode`].
    Internal(InternalNode),
    /// A wrapper of [`LeafNode`].
    Leaf(LeafNode<K>),
    /// Represents empty tree only
    Null,
}

impl<K> Node<K> {
    pub fn leaf_count(&self) -> usize {
        todo!()
    }
}

/// Represents an account.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LeafNode<K> {
    _p: PhantomData<K>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalNode {}
