// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, r#ref::Ref, KeyHash};
use aptos_metrics_core::TimerHelper;
use itertools::Either;
use std::{collections::BTreeMap, ptr::NonNull, sync::Arc};

#[derive(Debug)]
pub(crate) struct InternalNode<K, V> {
    pub left: NodeRef<K, V>,
    pub right: NodeRef<K, V>,
    pub layer: u32,
    pub left_layer: u32,
    pub right_layer: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct CollisionCell<V> {
    pub value: V,
    pub layer: u32,
}

#[derive(Clone, Debug)]
pub(crate) enum LeafContent<K, V> {
    UniqueLatest { key: K, value: V },
    Collision(BTreeMap<K, CollisionCell<V>>),
}

impl<K, V> LeafContent<K, V> {
    pub fn into_iter(self, base_layer: u32) -> impl Iterator<Item = (K, V)> {
        match self {
            LeafContent::UniqueLatest { key, value } => Either::Left(std::iter::once((key, value))),
            LeafContent::Collision(map) => {
                Either::Right(map.into_iter().filter_map(move |(key, cell)| {
                    (cell.layer > base_layer).then_some((key, cell.value))
                }))
            },
        }
    }

    fn into_cell_iter(self, layer: u32) -> impl Iterator<Item = (K, CollisionCell<V>)> {
        match self {
            LeafContent::UniqueLatest { key, value } => {
                Either::Left(std::iter::once((key, CollisionCell { value, layer })))
            },
            LeafContent::Collision(map) => Either::Right(map.into_iter()),
        }
    }

    pub fn combined_with(self, layer: u32, other: Self, other_layer: u32, base_layer: u32) -> Self
    where
        K: Clone + Eq + Ord,
        V: Clone,
    {
        use LeafContent::*;

        assert!(layer < other_layer);
        assert!(base_layer < other_layer);

        match (self, other) {
            // Collision should be rare, this is likely.
            (UniqueLatest { key: old_key, .. }, UniqueLatest { key, value }) if old_key == key => {
                UniqueLatest { key, value }
            },
            (myself, other) => {
                let _timer = TIMER.timer_with(&["_", "leaf_content_collision"]);

                let map: BTreeMap<_, _> = myself
                    .into_cell_iter(layer)
                    .chain(other.into_cell_iter(other_layer))
                    // retire entries that's at base_layer or even older
                    .filter(|(_key, cell)| cell.layer > base_layer)
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

    fn get(&self, key: &K, base_layer: u32) -> Option<&V>
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
                if cell.layer > base_layer {
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
    pub layer: u32,
}

impl<K, V> LeafNode<K, V> {
    pub fn get_value(&self, key: &K, base_layer: u32) -> Option<&V>
    where
        K: Eq + Ord,
    {
        self.content.get(key, base_layer)
    }
}

#[derive(Debug)]
pub(crate) enum NodeRef<K, V> {
    Empty,
    Leaf(Ref<LeafNode<K, V>>),
    Internal(Ref<InternalNode<K, V>>),
}

impl<K, V> NodeRef<K, V> {
    pub fn new_leaf(key_hash: KeyHash, content: LeafContent<K, V>, layer: u32) -> Self {
        // println!("creating new leaf. key_hash: {:?}. layer: {}", key_hash, layer);
        Self::Leaf(Ref::Strong(Box::new(LeafNode {
            key_hash,
            content,
            layer,
        })))
    }

    pub fn new_internal(
        left: Self,
        right: Self,
        layer: u32,
        left_layer: u32,
        right_layer: u32,
    ) -> Self {
        // println!("creating new internal.");
        // assert_eq!(left.is_empty(), left_layer.is_none());
        // assert_eq!(right.is_empty(), right_layer.is_none());
        Self::Internal(Ref::Strong(Box::new(InternalNode {
            left,
            right,
            layer,
            left_layer,
            right_layer,
        })))
    }

    pub fn from_raw(node: NodeRawPtr<K, V>) -> Self {
        match node {
            NodeRawPtr::Empty => Self::Empty,
            NodeRawPtr::Leaf(leaf) => Self::Leaf(Ref::from_raw(leaf)),
            NodeRawPtr::Internal(internal) => Self::Internal(Ref::from_raw(internal)),
        }
    }

    pub fn get_raw(&self, base_layer: u32) -> NodeRawPtr<K, V> {
        match self {
            NodeRef::Empty => NodeRawPtr::Empty,
            NodeRef::Leaf(leaf) => NodeRawPtr::Leaf(leaf.get_raw()),
            NodeRef::Internal(internal) => NodeRawPtr::Internal(internal.get_raw()),
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

#[derive(Debug)]
pub(crate) enum NodeRawPtr<K, V> {
    Empty,
    Leaf(NonNull<LeafNode<K, V>>),
    Internal(NonNull<InternalNode<K, V>>),
}

impl<K, V> NodeRawPtr<K, V> {
    /// THIS MUST NOT DANGLE!!!
    pub fn children(&self, depth: usize, base_layer: u32) -> ((Self, u32), (Self, u32)) {
        match self {
            Self::Empty => ((Self::Empty, 0), (Self::Empty, 0)),
            Self::Leaf(leaf) => {
                let leaf_ref = unsafe { leaf.as_ref() };
                if leaf_ref.key_hash.bit(depth) {
                    ((Self::Empty, 0), (Self::Leaf(*leaf), leaf_ref.layer))
                } else {
                    ((Self::Leaf(*leaf), leaf_ref.layer), (Self::Empty, 0))
                }
            },
            Self::Internal(internal) => {
                let internal = unsafe { internal.as_ref() };
                let left = if internal.left_layer > base_layer {
                    (internal.left.get_raw(base_layer), internal.left_layer)
                } else {
                    (Self::Empty, 0)
                };
                let right = if internal.right_layer > base_layer {
                    (internal.right.get_raw(base_layer), internal.right_layer)
                } else {
                    (Self::Empty, 0)
                };
                (left, right)
            },
        }
    }
}

impl<K, V> Clone for NodeRawPtr<K, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Leaf(leaf) => Self::Leaf(*leaf),
            Self::Internal(internal) => Self::Internal(*internal),
        }
    }
}

impl<K, V> Copy for NodeRawPtr<K, V> {}

unsafe impl<K, V> Send for NodeRawPtr<K, V> {}
unsafe impl<K, V> Sync for NodeRawPtr<K, V> {}

// impl<K, V> Clone for NodeRef<K, V> {
//     fn clone(&self) -> Self {
//         match self {
//             NodeRef::Empty => NodeRef::Empty,
//             NodeRef::Leaf(leaf) => NodeRef::Leaf(leaf.clone()),
//             NodeRef::Internal(internal) => NodeRef::Internal(internal.clone()),
//         }
//     }
// }
//
// #[derive(Debug)]
// pub(crate) enum NodeStrongRef<K, V> {
//     Empty,
//     Leaf(Arc<LeafNode<K, V>>),
//     Internal(Arc<InternalNode<K, V>>),
// }
//
// impl<K, V> Clone for NodeStrongRef<K, V> {
//     fn clone(&self) -> Self {
//         match self {
//             NodeStrongRef::Empty => NodeStrongRef::Empty,
//             NodeStrongRef::Leaf(leaf) => NodeStrongRef::Leaf(leaf.clone()),
//             NodeStrongRef::Internal(internal) => NodeStrongRef::Internal(internal.clone()),
//         }
//     }
// }
//
// impl<K, V> NodeStrongRef<K, V> {
//     pub fn weak_ref(&self) -> NodeRef<K, V> {
//         match self {
//             NodeStrongRef::Empty => NodeRef::Empty,
//             NodeStrongRef::Leaf(leaf) => NodeRef::Leaf(Ref::Weak(Arc::downgrade(leaf))),
//             NodeStrongRef::Internal(internal) => {
//                 NodeRef::Internal(Ref::Weak(Arc::downgrade(internal)))
//             },
//         }
//     }
//
//     pub fn children(&self, depth: usize, base_layer: u64) -> (Self, Self) {
//         use NodeStrongRef::*;
//
//         match self {
//             Empty => (Empty, Empty),
//             Leaf(leaf) => {
//                 if leaf.key_hash.bit(depth) {
//                     (Empty, self.clone())
//                 } else {
//                     (self.clone(), Empty)
//                 }
//             },
//             Internal(internal) => (
//                 internal.left.get_strong(base_layer),
//                 internal.right.get_strong(base_layer),
//             ),
//         }
//     }
// }
