// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dropper::DROPPER,
    metrics::LAYER,
    node::{Node, NodeInner, Ref},
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_metrics_core::IntGaugeHelper;
use std::sync::Arc;

mod dropper;
mod metrics;
mod node;
mod updater;
mod utils;

pub trait Key: Send + Sync + PartialEq + 'static {
    fn iter_bits(&self) -> impl Iterator<Item = bool>;
}

/// Value is required to be Clone because it's tricky to return &V from `get()`.
pub trait Value: Clone + Send + Sync + 'static {}

#[derive(Debug)]
struct Inner<K: Key, V: Value> {
    root: Option<Ref<Node<K, V>>>,
    children: Mutex<Vec<Arc<Inner<K, V>>>>,
    use_case: &'static str,
    family: HashValue,
    layer: u64,
}

impl<K: Key, V: Value> Drop for Inner<K, V> {
    fn drop(&mut self) {
        // Drop the root in a different thread, because that's the slowest part.
        DROPPER.schedule_drop(self.root.take());

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
        self.log_generation("dropped");
    }
}

impl<K: Key, V: Value> Inner<K, V> {
    fn new_family(use_case: &'static str) -> Arc<Self> {
        let family = HashValue::random();
        Arc::new(Self {
            root: None,
            children: Mutex::new(Vec::new()),
            use_case,
            family,
            layer: 0,
        })
    }

    fn spawn(self: &Arc<Self>, child_root: Ref<Node<K, V>>) -> Arc<Self> {
        let child = Arc::new(Self {
            root: Some(child_root),
            children: Mutex::new(Vec::new()),
            use_case: self.use_case,
            family: self.family,
            layer: self.layer + 1,
        });
        self.children.lock().push(child.clone());
        child.log_generation("spawn");

        child
    }

    fn drain_children_for_drop(&self) -> Vec<Arc<Self>> {
        self.children.lock().drain(..).collect()
    }

    fn log_generation(&self, event: &'static str) {
        LAYER.set_with(&[self.use_case, event], self.layer as i64);
    }
}

#[derive(Debug)]
pub struct MapLayer<K: Key, V: Value> {
    inner: Arc<Inner<K, V>>,
}

impl<K: Key, V: Value> Clone for MapLayer<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K: Key, V: Value> MapLayer<K, V> {
    pub fn new_family(use_case: &'static str) -> Self {
        Self {
            inner: Inner::new_family(use_case),
        }
    }

    pub fn view_layers_since(&self, bottom_layer: &MapLayer<K, V>) -> LayeredMap<K, V> {
        assert!(bottom_layer.is_family(self));

        self.log_generation("view");
        bottom_layer.log_generation("ancestor_ref");

        LayeredMap {
            bottom_layer: bottom_layer.clone(),
            top_layer: self.clone(),
        }
    }

    pub fn view_self(&self) -> LayeredMap<K, V> {
        self.view_layers_since(self)
    }

    pub fn log_generation(&self, name: &'static str) {
        self.inner.log_generation(name)
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn is_family(&self, other: &Self) -> bool {
        self.inner.family == other.inner.family
    }
}

#[derive(Clone, Debug)]
pub struct LayeredMap<K: Key, V: Value> {
    bottom_layer: MapLayer<K, V>,
    top_layer: MapLayer<K, V>,
}

impl<K: Key, V: Value> LayeredMap<K, V>
where
    K: Key,
    V: Value,
{
    /*
    fn new_layer(&self, new_root: SubTree<V>) -> MapLayer<V> {
        MapLayer {
            inner: self.top_layer.inner.spawn(new_root),
        }
    }
     */

    pub fn unpack(self) -> (MapLayer<K, V>, MapLayer<K, V>) {
        let Self {
            bottom_layer,
            top_layer,
        } = self;

        (bottom_layer, top_layer)
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cur_node = self.top_layer.inner.root.clone();

        let mut bits = key.iter_bits();

        loop {
            match cur_node {
                None => {
                    return None;
                },
                Some(node_ref) => {
                    match node_ref.get_strong() {
                        None => {
                            return None;
                        },
                        Some(node) => {
                            if node.layer < self.bottom_layer.inner.layer {
                                return None;
                            }

                            match &node.inner {
                                NodeInner::Leaf(leaf_node) => {
                                    return if &leaf_node.key == key {
                                        Some(leaf_node.value.clone())
                                    } else {
                                        None
                                    }
                                },
                                NodeInner::Internal(internal) => {
                                    match bits.next() {
                                        None => {
                                            // FIXME(aldenhu): deal with key prefix -- shall we panic or allow storing values on internal nodes
                                            todo!()
                                        },
                                        Some(bit) => {
                                            if bit {
                                                // right
                                                cur_node = internal.right.clone();
                                            } else {
                                                // left
                                                cur_node = internal.left.clone();
                                            }
                                        },
                                    }
                                },
                            } // end match &node.inner
                        },
                    } // end match node_ref.get_strong()
                },
            } // end match cur_node
        } // end loop
    }

    /*
    pub fn batch_update(
        &self,
        updates: Vec<(HashValue, Option<&V>)>,
        usage: StateStorageUsage,
        proof_reader: &impl ProofRead,
    ) -> Result<Self, UpdateError> {
        // Flatten, dedup and sort the updates with a btree map since the updates between different
        // versions may overlap on the same address in which case the latter always overwrites.
        let kvs = updates
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect::<Vec<_>>();

        if kvs.is_empty() {
            if !usage.is_untracked() {
                assert_eq!(self.smt.inner.usage, usage);
            }
            Ok(self.clone())
        } else {
            let current_root = self.smt.root_weak();
            let root = SubTreeUpdater::update(
                current_root,
                &kvs[..],
                proof_reader,
                self.smt.inner.generation + 1,
            )?;
            Ok(self.spawn(root, usage))
        }
    }
    */
}
