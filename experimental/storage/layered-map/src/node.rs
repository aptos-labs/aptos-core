// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Weak};

#[derive(Clone, Debug)]
pub(crate) struct InternalNode<K, V> {
    pub left: Option<Ref<Node<K, V>>>,
    pub right: Option<Ref<Node<K, V>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct LeafNode<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Debug)]
pub(crate) enum NodeInner<K, V> {
    Internal(InternalNode<K, V>),
    Leaf(LeafNode<K, V>),
}

#[derive(Debug)]
pub(crate) struct Node<K, V> {
    pub(crate) layer: u64,
    pub(crate) inner: NodeInner<K, V>,
}

impl<K, V> Node<K, V> {
    pub fn new_leaf(key: K, value: V, layer: u64) -> Self {
        Self {
            layer,
            inner: NodeInner::Leaf(LeafNode { key, value }),
        }
    }

    pub fn new_leaf_from_node(node: LeafNode<K, V>, layer: u64) -> Self {
        Self {
            layer,
            inner: NodeInner::Leaf(node),
        }
    }

    pub fn new_internal_from_node(node: InternalNode<K, V>, layer: u64) -> Self {
        Self {
            layer,
            inner: NodeInner::Internal(node),
        }
    }

    pub fn inner(&self) -> &NodeInner<K, V> {
        &self.inner
    }
}

#[derive(Debug)]
pub enum Ref<R> {
    Strong(Arc<R>),
    Weak(Weak<R>),
}

impl<R> Clone for Ref<R> {
    fn clone(&self) -> Self {
        match self {
            Ref::Strong(arc) => Ref::Strong(arc.clone()),
            Ref::Weak(weak) => Ref::Weak(weak.clone()),
        }
    }
}

impl<R> Ref<R> {
    pub fn new_strong(referee: R) -> Self {
        Self::Strong(Arc::new(referee))
    }

    pub fn weak(&self) -> Self {
        Self::Weak(match self {
            Self::Strong(arc) => Arc::downgrade(arc),
            Self::Weak(weak) => weak.clone(),
        })
    }

    pub fn get_strong(&self) -> Option<Arc<R>> {
        match self {
            Self::Strong(arc) => Some(arc.clone()),
            Self::Weak(weak) => weak.upgrade(),
        }
    }
}
