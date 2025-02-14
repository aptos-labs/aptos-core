// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use std::{
    fmt,
    fmt::{Debug, Formatter},
    mem,
};

pub(crate) struct FlattenPerfectTree<Leaf> {
    leaves: Vec<Leaf>,
}

impl<Leaf> FlattenPerfectTree<Leaf> {
    pub fn new_with<MakeLeaf>(height: usize, make_leaf: MakeLeaf) -> Self
    where
        MakeLeaf: FnMut() -> Leaf,
    {
        let num_leaves = if height == 0 { 0 } else { 1 << (height - 1) };

        let mut leaves = Vec::with_capacity(num_leaves);
        leaves.resize_with(num_leaves, make_leaf);

        Self { leaves }
    }

    pub fn get_ref(&self) -> FptRef<Leaf> {
        FptRef {
            leaves: &self.leaves,
        }
    }

    pub(crate) fn take_for_drop(&mut self) -> Self {
        let mut ret = Self { leaves: Vec::new() };
        std::mem::swap(self, &mut ret);

        ret
    }

    pub(crate) unsafe fn transmute<NewLeafType>(self) -> FlattenPerfectTree<NewLeafType> {
        FlattenPerfectTree {
            leaves: mem::transmute(self.leaves),
        }
    }
}

impl<Leaf> Debug for FlattenPerfectTree<Leaf> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlattenPerfectTree({})", self.leaves.len())
    }
}

pub(crate) struct FptRef<'a, Leaf> {
    leaves: &'a [Leaf],
}

impl<'a, Leaf> FptRef<'a, Leaf> {
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

    pub fn expect_single_node(&self) -> &'a Leaf {
        assert!(self.is_single_node());
        &self.leaves[0]
    }

    pub fn expect_leaf(&self, leaf_index: usize) -> &'a Leaf {
        &self.leaves[leaf_index]
    }

    pub fn height(&self) -> usize {
        utils::binary_tree_height(self.leaves.len())
    }

    pub fn into_leaf_iter(self) -> impl Iterator<Item = &'a Leaf> {
        self.leaves.iter()
    }
}
