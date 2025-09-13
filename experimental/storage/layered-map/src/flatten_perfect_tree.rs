// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    node::{NodeRef, NodeStrongRef},
    utils,
};
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

pub(crate) struct FlattenPerfectTree<K, V> {
    leaves: Vec<NodeRef<K, V>>,
}

impl<K, V> FlattenPerfectTree<K, V> {
    pub fn new_with_empty_nodes(height: usize) -> Self {
        let num_leaves = if height == 0 { 0 } else { 1 << (height - 1) };

        Self {
            leaves: vec![NodeRef::Empty; num_leaves],
        }
    }

    pub fn get_ref(&self) -> FptRef<'_, K, V> {
        FptRef {
            leaves: &self.leaves,
        }
    }

    pub fn get_mut(&mut self) -> FptRefMut<'_, K, V> {
        FptRefMut {
            leaves: &mut self.leaves,
        }
    }

    pub(crate) fn take_for_drop(&mut self) -> Self {
        let mut ret = Self { leaves: Vec::new() };
        std::mem::swap(self, &mut ret);

        ret
    }
}

impl<K, V> Debug for FlattenPerfectTree<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlattenPerfectTree({})", self.leaves.len())
    }
}

pub(crate) struct FptRef<'a, K, V> {
    leaves: &'a [NodeRef<K, V>],
}

impl<'a, K, V> FptRef<'a, K, V> {
    pub fn num_leaves(&self) -> usize {
        self.leaves.len()
    }

    pub fn expect_sub_trees(self) -> (Self, Self) {
        assert!(!self.is_single_node());
        let (left, right) = self.leaves.split_at(self.leaves.len() / 2);
        (Self { leaves: left }, Self { leaves: right })
    }

    pub fn is_single_node(&self) -> bool {
        self.leaves.len() == 1
    }

    pub fn expect_single_node(&self, base_layer: u64) -> NodeStrongRef<K, V> {
        assert!(self.is_single_node());
        self.leaves[0].get_strong(base_layer)
    }

    pub fn expect_foot(&self, foot: usize, base_layer: u64) -> NodeStrongRef<K, V> {
        self.leaves[foot].get_strong(base_layer)
    }

    pub fn height(&self) -> usize {
        utils::binary_tree_height(self.leaves.len())
    }

    pub fn into_feet_iter(self) -> impl Iterator<Item = &'a NodeRef<K, V>> {
        self.leaves.iter()
    }
}

pub(crate) struct FptRefMut<'a, K, V> {
    leaves: &'a mut [NodeRef<K, V>],
}

impl<'a, K, V> FptRefMut<'a, K, V> {
    pub fn is_single_node(&self) -> bool {
        self.leaves.len() == 1
    }

    pub fn expect_into_single_node_mut(self) -> &'a mut NodeRef<K, V> {
        assert!(self.is_single_node());
        &mut self.leaves[0]
    }

    pub fn expect_into_sub_trees(self) -> (Self, Self) {
        assert!(!self.is_single_node());
        let (left, right) = self.leaves.split_at_mut(self.leaves.len() / 2);
        (Self { leaves: left }, Self { leaves: right })
    }
}
