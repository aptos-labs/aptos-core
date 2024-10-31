// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::node::{InternalNode, NodeRef, NodeStrongRef};
use aptos_drop_helper::ArcAsyncDrop;
use std::sync::Arc;

pub(crate) struct DescendantIterator<'a, K: ArcAsyncDrop, V: ArcAsyncDrop> {
    root: Option<&'a NodeRef<K, V>>,
    base_layer: u64,
    current_leaf: Option<Box<dyn 'a + Iterator<Item = (K, V)>>>,
    ancestors: Vec<Arc<InternalNode<K, V>>>,
}

impl<'a, K, V> DescendantIterator<'a, K, V>
where
    K: ArcAsyncDrop + Clone,
    V: ArcAsyncDrop + Clone,
{
    pub fn new(root: &'a NodeRef<K, V>, base_layer: u64) -> Self {
        Self {
            root: Some(root),
            base_layer,
            current_leaf: None,
            ancestors: Vec::new(),
        }
    }

    fn find_next_leaf(
        &mut self,
        current_node: Option<NodeStrongRef<K, V>>,
    ) -> Option<Box<dyn Iterator<Item = (K, V)>>> {
        match current_node {
            None => {
                if let Some(internal) = self.ancestors.pop() {
                    let right = internal.right.get_strong(self.base_layer);
                    self.find_next_leaf(Some(right))
                } else {
                    None
                }
            },
            Some(node) => match node {
                NodeStrongRef::Empty => self.find_next_leaf(None),
                NodeStrongRef::Leaf(leaf) => {
                    Some(Box::new(leaf.content.clone().into_iter(self.base_layer)))
                },
                NodeStrongRef::Internal(internal) => {
                    let left = internal.left.get_strong(self.base_layer);
                    self.ancestors.push(internal);
                    self.find_next_leaf(Some(left))
                },
            },
        }
    }

    fn next_impl(&mut self) -> Option<(K, V)> {
        loop {
            match &mut self.current_leaf {
                None => {
                    if let Some(root) = self.root.take() {
                        // Iterater not started yet, consume root and go down.
                        self.current_leaf =
                            self.find_next_leaf(Some(root.get_strong(self.base_layer)));
                    } else if self.ancestors.is_empty() {
                        return None;
                    } else {
                        self.current_leaf = self.find_next_leaf(None);
                    }
                },
                Some(kv_iter) => {
                    if let Some((key, value)) = kv_iter.next() {
                        return Some((key, value));
                    } else {
                        self.current_leaf = None;
                    }
                },
            } // end match self.current_leaf
        } // end loop
    }
}

impl<'a, K, V> Iterator for DescendantIterator<'a, K, V>
where
    K: ArcAsyncDrop + Clone,
    V: ArcAsyncDrop + Clone,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.next_impl()
    }
}
