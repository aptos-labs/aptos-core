// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dropper::DROPPER,
    metrics::{LAYER, TIMER},
    node::{NodeRef, NodeStrongRef},
};
use aptos_crypto::HashValue;
use aptos_drop_helper::ArcAsyncDrop;
use aptos_infallible::Mutex;
use aptos_metrics_core::{IntGaugeHelper, TimerHelper};
use std::sync::Arc;

mod dropper;
mod metrics;
mod node;
pub(crate) mod r#ref;

#[cfg(test)]
mod tests;

/// When recursively creating a new `MapLayer` (a crit bit tree overlay), passing down `Vec<(K, Option<V>)>`
/// That's why we require `Key: Clone` and clone the key and value only when the leaf node is
/// created.
pub trait Key: Clone + Eq {
    fn iter_bits(&self) -> impl Iterator<Item = bool>;

    fn bit(&self, depth: usize) -> bool;
}

/// Similar to `Key`, we require `Value: Clone`, another reason being it's tricky to figure out the
/// lifetime if `get()` returns a reference to the value -- we simply clone the value.
pub trait Value: Clone {}
impl<T: Clone> Value for T {}

#[derive(Debug)]
struct LayerInner<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    root: NodeRef<K, V>,
    children: Mutex<Vec<Arc<LayerInner<K, V>>>>,
    use_case: &'static str,
    family: HashValue,
    layer: u64,
    // Oldest layer viewable when self is created -- self won't even weak-link to a node created in
    // a layer older than this.
    base_layer: u64,
}

impl<K: ArcAsyncDrop, V: ArcAsyncDrop> Drop for LayerInner<K, V> {
    fn drop(&mut self) {
        // Drop the tree nodes in a different thread, because that's the slowest part.
        DROPPER.schedule_drop(self.root.take_for_drop());

        let mut stack = self.drain_children_for_drop();
        while let Some(descendant) = stack.pop() {
            if Arc::strong_count(&descendant) == 1 {
                // The only ref is the one we are now holding, so the
                // descendant will be dropped after we free the `Arc`, which results in a chain
                // of such structures being dropped recursively and that might trigger a stack
                // overflow. To prevent that we follow the chain further to disconnect things
                // beforehand.
                stack.extend(descendant.drain_children_for_drop());
            }
        }
        self.log_layer("dropped");
    }
}

impl<K: ArcAsyncDrop, V: ArcAsyncDrop> LayerInner<K, V> {
    fn new_family(use_case: &'static str) -> Arc<Self> {
        let family = HashValue::random();
        Arc::new(Self {
            root: NodeRef::Empty,
            children: Mutex::new(Vec::new()),
            use_case,
            family,
            layer: 0,
            base_layer: 0,
        })
    }

    fn spawn(self: &Arc<Self>, child_root: NodeRef<K, V>, base_layer: u64) -> Arc<Self> {
        let child = Arc::new(Self {
            root: child_root,
            children: Mutex::new(Vec::new()),
            use_case: self.use_case,
            family: self.family,
            layer: self.layer + 1,
            base_layer,
        });
        self.children.lock().push(child.clone());
        child.log_layer("spawn");

        child
    }

    fn drain_children_for_drop(&self) -> Vec<Arc<Self>> {
        self.children.lock().drain(..).collect()
    }

    fn log_layer(&self, event: &'static str) {
        LAYER.set_with(&[self.use_case, event], self.layer as i64);
    }
}

#[derive(Debug)]
pub struct MapLayer<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    inner: Arc<LayerInner<K, V>>,
}

/// Manual implementation because `LayerInner` is deliberately not `Clone`.
impl<K: ArcAsyncDrop, V: ArcAsyncDrop> Clone for MapLayer<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K: ArcAsyncDrop, V: ArcAsyncDrop> MapLayer<K, V> {
    pub fn new_family(use_case: &'static str) -> Self {
        Self {
            inner: LayerInner::new_family(use_case),
        }
    }

    pub fn into_layers_view_since(self, bottom_layer: MapLayer<K, V>) -> LayeredMap<K, V> {
        assert!(bottom_layer.is_family(&self));
        assert!(bottom_layer.inner.layer >= self.inner.base_layer);
        assert!(bottom_layer.inner.layer <= self.inner.layer);

        self.log_layer("view");
        bottom_layer.log_layer("ancestor_ref");

        LayeredMap {
            bottom_layer,
            top_layer: self,
        }
    }

    pub fn view_layers_since(&self, bottom_layer: &MapLayer<K, V>) -> LayeredMap<K, V> {
        self.clone().into_layers_view_since(bottom_layer.clone())
    }

    pub fn log_layer(&self, name: &'static str) {
        self.inner.log_layer(name)
    }

    fn is_family(&self, other: &Self) -> bool {
        self.inner.family == other.inner.family
    }
}

#[derive(Clone, Debug)]
pub struct LayeredMap<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    bottom_layer: MapLayer<K, V>,
    top_layer: MapLayer<K, V>,
}

impl<K, V> LayeredMap<K, V>
where
    K: ArcAsyncDrop + Key,
    V: ArcAsyncDrop + Value,
{
    pub fn unpack(self) -> (MapLayer<K, V>, MapLayer<K, V>) {
        let Self {
            bottom_layer,
            top_layer,
        } = self;

        (bottom_layer, top_layer)
    }

    fn bottom_layer(&self) -> u64 {
        self.bottom_layer.inner.layer
    }

    fn top_layer(&self) -> u64 {
        self.top_layer.inner.layer
    }

    fn get_node_strong(&self, node_ref: &NodeRef<K, V>) -> NodeStrongRef<K, V> {
        node_ref.get_strong_with_min_layer(self.bottom_layer())
    }

    fn root(&self) -> NodeStrongRef<K, V> {
        self.get_node_strong(&self.top_layer.inner.root)
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cur_node = self.root();
        let mut bits = key.iter_bits();

        loop {
            match cur_node {
                NodeStrongRef::Empty => return None,
                NodeStrongRef::Leaf(leaf) => {
                    return if &leaf.key == key {
                        leaf.value.clone()
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

    fn new_leaf(&self, item: &(K, Option<V>)) -> NodeRef<K, V> {
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
        use NodeStrongRef::*;

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
        use NodeRef::*;

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
        items: &[(K, Option<V>)],
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

    pub fn new_layer(&self, items: &[(K, Option<V>)]) -> MapLayer<K, V> {
        let _timer = TIMER.timer_with(&[self.top_layer.inner.use_case, "new_layer"]);
        let root = self.create_tree(0, self.root(), items);
        MapLayer {
            inner: self.top_layer.inner.spawn(root, self.bottom_layer()),
        }
    }
}
