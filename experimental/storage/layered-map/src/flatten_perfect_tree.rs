// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    node::{NodeRawPtr, NodeRef},
    utils,
};
use itertools::Itertools;
use std::{
    fmt,
    fmt::{Debug, Formatter},
};

pub(crate) struct FlattenPerfectTree<K, V> {
    leaves: Vec<(NodeRef<K, V>, u32)>,
}

impl<K, V> FlattenPerfectTree<K, V> {
    pub fn new_with_empty_nodes(height: usize) -> Self {
        let num_leaves = if height == 0 { 0 } else { 1 << (height - 1) };
        let mut leaves = Vec::new();
        leaves.resize_with(num_leaves, || (NodeRef::Empty, 0));
        Self { leaves }
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
        let leaves = self
            .leaves
            .iter()
            .map(|(node_ref, layer)| match node_ref {
                NodeRef::Empty => format!("(Empty, {:?})", layer),
                NodeRef::Leaf(_) => format!("(Leaf, {:?})", layer),
                NodeRef::Internal(_) => format!("(Internal, {:?})", layer),
            })
            .collect_vec();
        write!(f, "FlattenPerfectTree({:?})", leaves)
    }
}

pub(crate) struct FptRef<'a, K, V> {
    leaves: &'a [(NodeRef<K, V>, u32)],
}

impl<'a, K, V> Debug for FptRef<'a, K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let leaves = self
            .leaves
            .iter()
            .map(|(node_ref, layer)| match node_ref {
                NodeRef::Empty => format!("(Empty, {:?})", layer),
                NodeRef::Leaf(_) => format!("(Leaf, {:?})", layer),
                NodeRef::Internal(_) => format!("(Internal, {:?})", layer),
            })
            .collect_vec();
        write!(f, "FlattenPerfectTree({:?})", leaves)
    }
}

impl<'a, K, V> FptRef<'a, K, V> {
    pub fn num_leaves(&self) -> usize {
        self.leaves.len()
    }

    pub fn expect_sub_trees(self) -> (Self, Self) {
        assert!(!self.is_single_node());
        assert_eq!(self.leaves.len() % 2, 0);
        let (left, right) = self.leaves.split_at(self.leaves.len() / 2);
        (Self { leaves: left }, Self { leaves: right })
    }

    pub fn is_single_node(&self) -> bool {
        self.leaves.len() == 1
    }

    pub fn expect_single_node(&self, base_layer: u32) -> (NodeRawPtr<K, V>, u32) {
        assert!(self.is_single_node());
        let (node, layer) = &self.leaves[0];
        // match node {
        //     NodeRef::Empty => (), assert!(layer.is_none()),
        //     NodeRef::Leaf(leaf) => assert!(leaf.is_strong()),
        //     NodeRef::Internal(internal) => assert!(internal.is_strong()),
        // }
        if *layer > base_layer {
            (node.get_raw(base_layer), *layer)
        } else {
            (NodeRawPtr::Empty, 0)
        }
    }

    pub fn expect_foot(&self, foot: usize, base_layer: u32) -> NodeRawPtr<K, V> {
        let (node, node_layer) = &self.leaves[foot];
        if *node_layer > base_layer {
            node.get_raw(base_layer) // base_layer is not used inside get_raw
        } else {
            NodeRawPtr::Empty
        }
    }

    pub fn root_layer(&self) -> u32 {
        self.leaves.iter().map(|(_, x)| *x).max().unwrap()
    }

    pub fn height(&self) -> usize {
        utils::binary_tree_height(self.leaves.len())
    }

    pub fn into_feet_iter(self, base_layer: u32) -> impl Iterator<Item = &'a NodeRef<K, V>> {
        self.leaves
            .iter()
            .filter_map(move |(x, layer)| if *layer > base_layer { Some(x) } else { None })
    }
}

pub(crate) struct FptRefMut<'a, K, V> {
    leaves: &'a mut [(NodeRef<K, V>, u32)],
}

impl<'a, K, V> FptRefMut<'a, K, V> {
    pub fn is_single_node(&self) -> bool {
        self.leaves.len() == 1
    }

    pub fn expect_into_single_node_mut(self) -> &'a mut (NodeRef<K, V>, u32) {
        assert!(self.is_single_node());
        &mut self.leaves[0]
    }

    pub fn expect_into_sub_trees(self) -> (Self, Self) {
        assert!(!self.is_single_node());
        let (left, right) = self.leaves.split_at_mut(self.leaves.len() / 2);
        (Self { leaves: left }, Self { leaves: right })
    }
}
