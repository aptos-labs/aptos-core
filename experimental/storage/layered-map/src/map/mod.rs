// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod new_layer_impl;

use crate::{iterator::DescendantIterator, node::NodeStrongRef, Key, KeyHash, MapLayer, Value};
use velor_drop_helper::ArcAsyncDrop;
use std::marker::PhantomData;

pub(crate) type DefaultHashBuilder = core::hash::BuildHasherDefault<ahash::AHasher>;

/// A view of content within range (base_layer, top_layer] (n.b. left-exclusive, right-inclusive).
#[derive(Clone, Debug)]
pub struct LayeredMap<K: ArcAsyncDrop, V: ArcAsyncDrop, S = DefaultHashBuilder> {
    /// n.b. base layer content is not visible
    base_layer: MapLayer<K, V>,
    top_layer: MapLayer<K, V>,
    /// Hasher is needed only for spawning a new layer, i.e. for a read only map there's no need to
    /// pay the overhead of constructing it.
    _hash_builder: PhantomData<S>,
}

impl<K, V, S> LayeredMap<K, V, S>
where
    K: ArcAsyncDrop,
    V: ArcAsyncDrop,
{
    pub fn new(base_layer: MapLayer<K, V>, top_layer: MapLayer<K, V>) -> Self {
        Self {
            base_layer,
            top_layer,
            _hash_builder: PhantomData,
        }
    }

    pub fn unpack(self) -> (MapLayer<K, V>, MapLayer<K, V>) {
        let Self {
            base_layer,
            top_layer,
            _hash_builder,
        } = self;

        (base_layer, top_layer)
    }

    pub(crate) fn base_layer(&self) -> u64 {
        self.base_layer.layer()
    }
}

impl<K, V, S> LayeredMap<K, V, S>
where
    K: ArcAsyncDrop + Key,
    V: ArcAsyncDrop + Value,
{
    pub fn get_with_hasher(&self, key: &K, hash_builder: &S) -> Option<V>
    where
        S: core::hash::BuildHasher,
    {
        let key_hash = KeyHash(hash_builder.hash_one(key));
        let mut bits = key_hash.iter_bits();

        let peak = self.top_layer.peak();
        let mut foot = 0;
        for _ in 0..peak.height() - 1 {
            foot = (foot << 1) | bits.next().expect("bits exhausted") as usize;
        }

        self.get_under_node(peak.expect_foot(foot, self.base_layer()), key, &mut bits)
    }

    fn get_under_node(
        &self,
        node: NodeStrongRef<K, V>,
        key: &K,
        remaining_key_bits: &mut impl Iterator<Item = bool>,
    ) -> Option<V> {
        let mut cur_node = node;
        let bits = remaining_key_bits;

        loop {
            match cur_node {
                NodeStrongRef::Empty => return None,
                NodeStrongRef::Leaf(leaf) => {
                    return leaf.get_value(key, self.base_layer()).cloned()
                },
                NodeStrongRef::Internal(internal) => match bits.next() {
                    None => {
                        unreachable!("value on key-prefix not supported.");
                    },
                    Some(bit) => {
                        if bit {
                            cur_node = internal.right.get_strong(self.base_layer());
                        } else {
                            cur_node = internal.left.get_strong(self.base_layer());
                        }
                    },
                },
            } // end match cur_node
        } // end loop
    }

    pub fn get(&self, key: &K) -> Option<V>
    where
        S: core::hash::BuildHasher + Default,
    {
        self.get_with_hasher(key, &Default::default())
    }

    // TODO(aldenhu): make `Item = (&K, &V)`
    pub fn iter(&self) -> impl Iterator<Item = (K, V)> + '_ {
        self.top_layer
            .peak()
            .into_feet_iter()
            .flat_map(|node| DescendantIterator::new(node, self.base_layer()))
    }
}
