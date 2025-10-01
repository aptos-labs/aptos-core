// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::node::{InternalNode, NodeRawPtr, NodeRef};
use aptos_drop_helper::ArcAsyncDrop;
use std::{ptr::NonNull, sync::Arc};

pub(crate) struct DescendantIterator<'a, K: ArcAsyncDrop, V: ArcAsyncDrop> {
    root: Option<&'a NodeRef<K, V>>,
    base_layer: u32,
    current_leaf: Option<Box<dyn 'a + Iterator<Item = (K, V)>>>,
    ancestors: Vec<NonNull<InternalNode<K, V>>>,
}

impl<'a, K, V> DescendantIterator<'a, K, V>
where
    K: ArcAsyncDrop + Clone,
    V: ArcAsyncDrop + Clone,
{
    pub fn new(root: &'a NodeRef<K, V>, base_layer: u32) -> Self {
        Self {
            root: Some(root),
            base_layer,
            current_leaf: None,
            ancestors: Vec::new(),
        }
    }

    fn find_next_leaf(
        &mut self,
        current_node: Option<NodeRawPtr<K, V>>,
    ) -> Option<Box<dyn Iterator<Item = (K, V)>>> {
        match current_node {
            None => {
                if let Some(internal) = self.ancestors.pop() {
                    let internal = unsafe { internal.as_ref() };
                    let right = if internal.right_layer > self.base_layer {
                        internal.right.get_raw(self.base_layer)
                    } else {
                        NodeRawPtr::Empty
                    };
                    self.find_next_leaf(Some(right))
                } else {
                    None
                }
            },
            Some(node) => match node {
                NodeRawPtr::Empty => self.find_next_leaf(None),
                NodeRawPtr::Leaf(leaf) => {
                    let leaf = unsafe { leaf.as_ref() };
                    Some(Box::new(leaf.content.clone().into_iter(self.base_layer)))
                },
                NodeRawPtr::Internal(internal) => {
                    let internal_ = unsafe { internal.as_ref() };
                    let left = if internal_.left_layer > self.base_layer {
                        internal_.left.get_raw(self.base_layer)
                    } else {
                        NodeRawPtr::Empty
                    };
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
                            self.find_next_leaf(Some(root.get_raw(self.base_layer)));
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

impl<K, V> Iterator for DescendantIterator<'_, K, V>
where
    K: ArcAsyncDrop + Clone,
    V: ArcAsyncDrop + Clone,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.next_impl()
    }
}
