// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    node::{NodeRef, NodeStrongRef},
    utils,
};
use arc_swap::ArcSwapOption;
use std::{
    fmt,
    fmt::{Debug, Formatter},
    sync::Arc,
};

pub(crate) struct FptFoot<K, V> {
    /// `None` represents NodeRef::Empty, to avoid unnecessary indirection
    node: ArcSwapOption<NodeRef<K, V>>,
}

impl<K, V> FptFoot<K, V> {
    pub fn empty() -> Self {
        Self {
            node: ArcSwapOption::new(None),
        }
    }

    pub fn get(&self) -> NodeRef<K, V> {
        self.node
            .load()
            .as_ref()
            .map(|node_ref_arc| node_ref_arc.as_ref())
            .cloned()
            .unwrap_or_else(|| NodeRef::Empty)
    }

    pub fn get_strong(&self, base_layer: u64) -> NodeStrongRef<K, V> {
        self.get().get_strong(base_layer)
    }

    pub fn set(&self, node_ref: NodeRef<K, V>) {
        self.node.store(Self::empty_to_none(node_ref))
    }

    fn empty_to_none(node_ref: NodeRef<K, V>) -> Option<Arc<NodeRef<K, V>>> {
        if let NodeRef::Empty = node_ref {
            None
        } else {
            Some(Arc::new(node_ref.clone()))
        }
    }
}

pub(crate) struct FlattenPerfectTree<K, V> {
    feet: Vec<FptFoot<K, V>>,
}

impl<K, V> FlattenPerfectTree<K, V> {
    pub fn new_with_empty_feet(height: usize) -> Self {
        let num_leaves = if height == 0 { 0 } else { 1 << (height - 1) };

        let mut feet = Vec::new();
        feet.resize_with(num_leaves, FptFoot::empty);

        Self { feet }
    }

    pub fn get_ref(&self) -> FptRef<K, V> {
        FptRef { feet: &self.feet }
    }

    pub(crate) fn take_for_drop(&mut self) -> Self {
        let mut ret = Self { feet: Vec::new() };
        std::mem::swap(self, &mut ret);

        ret
    }
}

impl<K, V> Debug for FlattenPerfectTree<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlattenPerfectTree({})", self.feet.len())
    }
}

pub(crate) struct FptRef<'a, K, V> {
    feet: &'a [FptFoot<K, V>],
}

impl<'a, K, V> FptRef<'a, K, V> {
    pub fn num_leaves(&self) -> usize {
        self.feet.len()
    }

    pub fn expect_sub_trees(self) -> (Self, Self) {
        assert!(!self.is_single_node());
        let (left, right) = self.feet.split_at(self.feet.len() / 2);
        (Self { feet: left }, Self { feet: right })
    }

    pub fn is_single_node(&self) -> bool {
        self.feet.len() == 1
    }

    pub fn expect_single_node(&self) -> &'a FptFoot<K, V> {
        assert!(self.is_single_node());
        &self.feet[0]
    }

    pub fn expect_foot(&self, foot: usize) -> &'a FptFoot<K, V> {
        &self.feet[foot]
    }

    pub fn height(&self) -> usize {
        utils::binary_tree_height(self.feet.len())
    }

    pub fn into_feet_iter(self) -> impl 'a + Iterator<Item = NodeRef<K, V>> {
        self.feet.iter().map(|foot| foot.get())
    }
}
