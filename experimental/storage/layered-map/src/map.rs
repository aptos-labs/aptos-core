// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    node::{NodeRef, NodeStrongRef},
    Key, MapLayer, Value,
};
use aptos_drop_helper::ArcAsyncDrop;
use aptos_metrics_core::TimerHelper;

#[derive(Clone, Debug)]
pub struct LayeredMap<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    bottom_layer: MapLayer<K, V>,
    top_layer: MapLayer<K, V>,
}

impl<K, V> LayeredMap<K, V>
where
    K: ArcAsyncDrop,
    V: ArcAsyncDrop,
{
    pub fn new(bottom_layer: MapLayer<K, V>, top_layer: MapLayer<K, V>) -> Self {
        Self {
            bottom_layer,
            top_layer,
        }
    }

    pub fn unpack(self) -> (MapLayer<K, V>, MapLayer<K, V>) {
        let Self {
            bottom_layer,
            top_layer,
        } = self;

        (bottom_layer, top_layer)
    }

    fn bottom_layer(&self) -> u64 {
        self.bottom_layer.layer()
    }

    fn top_layer(&self) -> u64 {
        self.top_layer.layer()
    }

    fn get_node_strong(&self, node_ref: &NodeRef<K, V>) -> NodeStrongRef<K, V> {
        node_ref.get_strong_with_min_layer(self.bottom_layer())
    }

    fn root(&self) -> NodeStrongRef<K, V> {
        self.get_node_strong(self.top_layer.root())
    }
}

impl<K, V> LayeredMap<K, V>
where
    K: ArcAsyncDrop + Key,
    V: ArcAsyncDrop + Value,
{
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cur_node = self.root();
        let mut bits = key.iter_bits();

        loop {
            match cur_node {
                NodeStrongRef::Empty => return None,
                NodeStrongRef::Leaf(leaf) => {
                    return if &leaf.key == key {
                        Some(leaf.value.clone())
                    } else {
                        None
                    }
                },
                NodeStrongRef::Internal(internal) => match bits.next() {
                    None => {
                        unreachable!("value on key-prefix not supported.");
                    },
                    Some(bit) => {
                        if bit {
                            cur_node = self.get_node_strong(&internal.right);
                        } else {
                            cur_node = self.get_node_strong(&internal.left);
                        }
                    },
                },
            } // end match cur_node
        } // end loop
    }

    fn new_leaf(&self, item: &(K, V)) -> NodeRef<K, V> {
        let (key, value) = item.clone();
        NodeRef::new_leaf(key, value, self.top_layer() + 1)
    }

    fn new_internal(&self, left: NodeRef<K, V>, right: NodeRef<K, V>) -> NodeRef<K, V> {
        NodeRef::new_internal(left, right, self.top_layer() + 1)
    }

    fn branch_down(
        &self,
        depth: usize,
        node: NodeStrongRef<K, V>,
    ) -> (NodeStrongRef<K, V>, NodeStrongRef<K, V>) {
        use crate::node::NodeStrongRef::*;

        match &node {
            Empty => (Empty, Empty),
            Leaf(leaf) => {
                if leaf.key.bit(depth) {
                    (Empty, node)
                } else {
                    (node, Empty)
                }
            },
            Internal(internal) => (
                self.get_node_strong(&internal.left),
                self.get_node_strong(&internal.right),
            ),
        }
    }

    fn merge_up(&self, left: NodeRef<K, V>, right: NodeRef<K, V>) -> NodeRef<K, V> {
        use crate::node::NodeRef::*;

        match (&left, &right) {
            (Empty, Leaf(..)) => right,
            (Leaf(..), Empty) => left,
            (Empty, Empty) => unreachable!("merge_up with two empty nodes"),
            _ => self.new_internal(left, right),
        }
    }

    fn create_tree(
        &self,
        depth: usize,
        current_root: NodeStrongRef<K, V>,
        items: &[(K, V)],
    ) -> NodeRef<K, V> {
        if items.is_empty() {
            return current_root.weak_ref();
        }

        if items.len() == 1 {
            match &current_root {
                NodeStrongRef::Empty => return self.new_leaf(&items[0]),
                NodeStrongRef::Leaf(leaf) => {
                    let (key, _value) = &items[0];
                    if &leaf.key == key {
                        return self.new_leaf(&items[0]);
                    }
                },
                NodeStrongRef::Internal(_) => {},
            }
        }

        let pivot = items.partition_point(|(key, _value)| !key.bit(depth));
        let (left_items, right_items) = items.split_at(pivot);
        let (left_root, right_root) = self.branch_down(depth, current_root);
        self.merge_up(
            self.create_tree(depth + 1, left_root, left_items),
            self.create_tree(depth + 1, right_root, right_items),
        )
    }

    pub fn new_layer(&self, items: &[(K, V)]) -> MapLayer<K, V> {
        let _timer = TIMER.timer_with(&[self.top_layer.use_case(), "new_layer"]);

        let root = self.create_tree(0, self.root(), items);

        self.top_layer.spawn(root, self.bottom_layer())
    }
}
