// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    node::{InternalNode, NodeStrongRef},
    LayeredMap,
};
use aptos_drop_helper::ArcAsyncDrop;
use std::sync::Arc;

pub struct LayeredMapIterator<'a, K: ArcAsyncDrop, V: ArcAsyncDrop, S> {
    layered_map: &'a LayeredMap<K, V, S>,
    current_leaf: Option<Box<dyn 'a + Iterator<Item = (K, V)>>>,
    ancestors: Vec<Arc<InternalNode<K, V>>>,
    started: bool,
}

impl<'a, K, V, S> LayeredMapIterator<'a, K, V, S>
where
    K: ArcAsyncDrop + Clone,
    V: ArcAsyncDrop + Clone,
{
    pub fn new(layered_map: &'a LayeredMap<K, V, S>) -> Self {
        Self {
            layered_map,
            current_leaf: None,
            ancestors: Vec::new(),
            started: false,
        }
    }

    fn find_next_leaf(
        &mut self,
        current_node: Option<NodeStrongRef<K, V>>,
    ) -> Option<Box<dyn Iterator<Item = (K, V)>>> {
        match current_node {
            None => {
                if let Some(internal) = self.ancestors.pop() {
                    let right = self.layered_map.get_node_strong(&internal.right);
                    self.find_next_leaf(Some(right))
                } else {
                    None
                }
            },
            Some(node) => match node {
                NodeStrongRef::Empty => self.find_next_leaf(None),
                NodeStrongRef::Leaf(leaf) => Some(Box::new(
                    leaf.content
                        .clone()
                        .into_iter(self.layered_map.base_layer()),
                )),
                NodeStrongRef::Internal(internal) => {
                    let left = self.layered_map.get_node_strong(&internal.left);
                    self.ancestors.push(internal);
                    self.find_next_leaf(Some(left))
                },
            },
        }
    }

    pub fn next_impl(&mut self) -> Option<(K, V)> {
        loop {
            match &mut self.current_leaf {
                None => {
                    if self.started {
                        if self.ancestors.is_empty() {
                            return None;
                        } else {
                            self.current_leaf = self.find_next_leaf(None);
                        }
                    } else {
                        self.current_leaf = self.find_next_leaf(Some(self.layered_map.root()));
                        self.started = true
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

impl<'a, K, V, S> Iterator for LayeredMapIterator<'a, K, V, S>
where
    K: ArcAsyncDrop + Clone,
    V: ArcAsyncDrop + Clone,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.next_impl()
    }
}
