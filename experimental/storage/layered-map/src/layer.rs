// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dropper::DROPPER,
    flatten_perfect_tree::{FlattenPerfectTree, FptRef},
    map::{DefaultHashBuilder, LayeredMap},
    metrics::LAYER,
};
use aptos_crypto::HashValue;
use aptos_drop_helper::ArcAsyncDrop;
use aptos_infallible::Mutex;
use aptos_metrics_core::IntGaugeHelper;
use std::{marker::PhantomData, sync::Arc};

#[derive(Debug)]
struct LayerInner<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    peak: FlattenPerfectTree<K, V>,
    children: Mutex<Vec<Arc<LayerInner<K, V>>>>,
    use_case: &'static str,
    family: HashValue,
    layer: u64,
    // Base layer when self is created -- `self` won't even weak-link to a node created in
    // the base or an older layer.
    base_layer: u64,
}

impl<K: ArcAsyncDrop, V: ArcAsyncDrop> Drop for LayerInner<K, V> {
    fn drop(&mut self) {
        // Drop the tree nodes in a different thread, because that's the slowest part.
        DROPPER.schedule_drop(self.peak.take_for_drop());

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
            peak: FlattenPerfectTree::new_with_empty_nodes(1),
            children: Mutex::new(Vec::new()),
            use_case,
            family,
            layer: 0,
            base_layer: 0,
        })
    }

    fn spawn(self: &Arc<Self>, child_peak: FlattenPerfectTree<K, V>, base_layer: u64) -> Arc<Self> {
        let child = Arc::new(Self {
            peak: child_peak,
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
pub struct MapLayer<K: ArcAsyncDrop, V: ArcAsyncDrop, S = DefaultHashBuilder> {
    inner: Arc<LayerInner<K, V>>,
    /// Carried only for type safety: a LayeredMap can only be with layers of the same hasher type.
    _hash_builder: PhantomData<S>,
}

/// Manual implementation because `LayerInner` is deliberately not `Clone`.
impl<K: ArcAsyncDrop, V: ArcAsyncDrop> Clone for MapLayer<K, V> {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<K: ArcAsyncDrop, V: ArcAsyncDrop> MapLayer<K, V> {
    pub fn new_family(use_case: &'static str) -> Self {
        Self::new(LayerInner::new_family(use_case))
    }

    fn new(inner: Arc<LayerInner<K, V>>) -> Self {
        Self {
            inner,
            _hash_builder: PhantomData,
        }
    }

    pub fn into_layers_view_after(self, base_layer: MapLayer<K, V>) -> LayeredMap<K, V> {
        assert!(base_layer.is_family(&self));
        assert!(base_layer.inner.layer >= self.inner.base_layer);
        assert!(base_layer.inner.layer <= self.inner.layer);

        self.log_layer("view");
        base_layer.log_layer("as_view_base");

        LayeredMap::new(base_layer, self)
    }

    pub fn view_layers_after(&self, base_layer: &MapLayer<K, V>) -> LayeredMap<K, V> {
        self.clone().into_layers_view_after(base_layer.clone())
    }

    pub fn log_layer(&self, name: &'static str) {
        self.inner.log_layer(name)
    }

    fn is_family(&self, other: &Self) -> bool {
        self.inner.family == other.inner.family
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn layer(&self) -> u64 {
        self.inner.layer
    }

    pub(crate) fn use_case(&self) -> &'static str {
        self.inner.use_case
    }

    pub(crate) fn spawn(&self, child_peak: FlattenPerfectTree<K, V>, base_layer: u64) -> Self {
        Self::new(self.inner.spawn(child_peak, base_layer))
    }

    pub(crate) fn peak(&self) -> FptRef<K, V> {
        self.inner.peak.get_ref()
    }
}
