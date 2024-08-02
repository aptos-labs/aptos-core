// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, r#ref::Ref, KeyHash};
use aptos_metrics_core::TimerHelper;
use itertools::Either;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
pub(crate) struct InternalNode<K, V> {
    pub left: NodeRef<K, V>,
    pub right: NodeRef<K, V>,
    pub layer: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct CollisionCell<V> {
    pub value: V,
    pub layer: u64,
}

#[derive(Clone, Debug)]
pub(crate) enum LeafContent<K, V> {
    UniqueLatest { key: K, value: V },
    Collision(BTreeMap<K, CollisionCell<V>>),
}

impl<K, V> LeafContent<K, V> {
    fn into_iter(self, layer: u64) -> impl Iterator<Item = (K, CollisionCell<V>)> {
        match self {
            LeafContent::UniqueLatest { key, value } => {
                Either::Left(std::iter::once((key, CollisionCell { value, layer })))
            },
            LeafContent::Collision(map) => Either::Right(map.into_iter()),
        }
    }

    pub fn combined_with(self, layer: u64, other: Self, other_layer: u64, base_layer: u64) -> Self
    where
        K: Clone + Eq + Ord,
        V: Clone,
    {
        use LeafContent::*;

        match (self, other) {
            // Collision should be rare, this is likely.
            (UniqueLatest { key: old_key, .. }, UniqueLatest { key, value }) if old_key == key => {
                UniqueLatest { key, value }
            },
            (myself, other) => {
                let _timer = TIMER.timer_with(&["_", "leaf_content_collision"]);

                let map: BTreeMap<_, _> = myself
                    .into_iter(layer)
                    .chain(other.into_iter(other_layer))
                    // retire entries that's older than the base_layer
                    .filter(|(_key, cell)| cell.layer >= base_layer)
                    .collect();

                assert!(!map.is_empty());
                if map.len() == 1 {
                    let (key, cell) = map.into_iter().next().unwrap();
                    assert_eq!(cell.layer, other_layer);
                    UniqueLatest {
                        key,
                        value: cell.value,
                    }
                } else {
                    Collision(map)
                }
            },
        }
    }

    fn get(&self, key: &K, min_layer: u64) -> Option<&V>
    where
        K: Eq + Ord,
    {
        use LeafContent::*;

        match self {
            UniqueLatest { key: k, value } => {
                if k == key {
                    Some(value)
                } else {
                    None
                }
            },
            Collision(map) => map.get(key).and_then(|cell| {
                if cell.layer >= min_layer {
                    Some(&cell.value)
                } else {
                    None
                }
            }),
        }
    }
}

#[derive(Debug)]
pub(crate) struct LeafNode<K, V> {
    pub key_hash: KeyHash,
    pub content: LeafContent<K, V>,
    pub layer: u64,
}

impl<K, V> LeafNode<K, V> {
    pub fn get_value(&self, key: &K, min_layer: u64) -> Option<&V>
    where
        K: Eq + Ord,
    {
        self.content.get(key, min_layer)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum NodeRef<K, V> {
    Empty,
    Leaf(Ref<LeafNode<K, V>>),
    Internal(Ref<InternalNode<K, V>>),
}

impl<K, V> NodeRef<K, V> {
    pub fn new_leaf(key_hash: KeyHash, content: LeafContent<K, V>, layer: u64) -> Self {
        Self::Leaf(Ref::Strong(Arc::new(LeafNode {
            key_hash,
            content,
            layer,
        })))
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
