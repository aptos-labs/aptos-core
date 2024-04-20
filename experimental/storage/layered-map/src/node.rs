// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::r#ref::Ref;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct InternalNode<K, V> {
    pub left: NodeRef<K, V>,
    pub right: NodeRef<K, V>,
    pub layer: u64,
}

#[derive(Debug)]
pub(crate) struct LeafNode<K, V> {
    pub key: K,
    pub value: Option<V>,
    pub layer: u64,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeRef<K, V> {
    Empty,
    Leaf(Ref<LeafNode<K, V>>),
    Internal(Ref<InternalNode<K, V>>),
}

impl<K, V> NodeRef<K, V> {
    pub fn new_leaf(key: K, value: Option<V>, layer: u64) -> Self {
        Self::Leaf(Ref::Strong(Arc::new(LeafNode { key, value, layer })))
    }

    pub fn new_internal(left: Self, right: Self, layer: u64) -> Self {
        Self::Internal(Ref::Strong(Arc::new(InternalNode { left, right, layer })))
    }

    pub fn get_strong_with_min_layer(&self, min_layer: u64) -> NodeStrongRef<K, V> {
        match self {
            NodeRef::Empty => NodeStrongRef::Empty,
            NodeRef::Leaf(leaf) => match leaf.try_get_strong() {
                None => NodeStrongRef::Empty,
                Some(leaf) => {
                    if leaf.layer >= min_layer {
                        NodeStrongRef::Leaf(leaf)
                    } else {
                        NodeStrongRef::Empty
                    }
                },
            },
            NodeRef::Internal(internal) => match internal.try_get_strong() {
                None => NodeStrongRef::Empty,
                Some(internal) => {
                    if internal.layer >= min_layer {
                        NodeStrongRef::Internal(internal)
                    } else {
                        NodeStrongRef::Empty
                    }
                },
            },
        }
    }

    pub fn take_for_drop(&mut self) -> Self {
        let mut ret = Self::Empty;
        std::mem::swap(self, &mut ret);

        ret
    }
}

#[derive(Debug)]
pub(crate) enum NodeStrongRef<K, V> {
    Empty,
    Leaf(Arc<LeafNode<K, V>>),
    Internal(Arc<InternalNode<K, V>>),
}

impl<K, V> Clone for NodeStrongRef<K, V> {
    fn clone(&self) -> Self {
        match self {
            NodeStrongRef::Empty => NodeStrongRef::Empty,
            NodeStrongRef::Leaf(leaf) => NodeStrongRef::Leaf(leaf.clone()),
            NodeStrongRef::Internal(internal) => NodeStrongRef::Internal(internal.clone()),
        }
    }
}

impl<K, V> NodeStrongRef<K, V> {
    pub fn weak_ref(&self) -> NodeRef<K, V> {
        match self {
            NodeStrongRef::Empty => NodeRef::Empty,
            NodeStrongRef::Leaf(leaf) => NodeRef::Leaf(Ref::Weak(Arc::downgrade(leaf))),
            NodeStrongRef::Internal(internal) => {
                NodeRef::Internal(Ref::Weak(Arc::downgrade(internal)))
            },
        }
    }
}
